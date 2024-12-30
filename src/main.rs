use core::str;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;
use std::string::String;
use csv::Reader;
use std::env;

// 读取分组方案=======================================================================================
// 读取JSON文件为字典(HashMap), 键为MDC编码, 值为MDC下的主诊断HashSet
fn read_file_as_str_to_set<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, HashSet<String>>, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;

    Ok(u)
}

// 读取JSON文件为一个嵌套Hashmap, 键为ADRG编码, 值为一个Hashmap, 值的键为表的类型，值为诊断或手术操作编码Hashset
fn read_file_as_str_nestring_hashmap<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, HashMap<String, HashSet<String>>>, Box<dyn Error>> {
    // 打开文件加载到缓冲区
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // 读取缓冲区的内容并且序列化
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

// 读取JSON文件为一个字典Hashmap
fn read_file_as_str_to_str<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    // 打开文件加载到缓冲区
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // 读取缓冲区的内容并且序列化
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

// 读取JSON文件为字典(HashMap), 键为MDC编码, 值为向量
fn read_file_as_str_to_tuple<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;

    Ok(u)
}


// 泛形函数根据输入的数据类型来生成读取文件并序列化为指定的类型
fn read_json_file<T, P: AsRef<Path>>(path: P) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data: T = serde_json::from_reader(reader)?;
    Ok(data)
}

// 读取所有手术或所有诊断列表
fn read_icd9_to_vec<P: AsRef<Path>>(file_path: P) -> Result<HashSet<String>, Box<dyn Error>> {
    let contents = fs::read_to_string(file_path)?;
    let v: HashSet<String> = contents.split(',').map(|s| s.to_string()).collect();
    Ok(v)
}

// 病例结构===========================================================================================
#[derive(Debug, Deserialize)]
struct DrgCase {
    id: String,               // 病例ID
    main_dis: String,         // 主诊断编码(必填)
    main_opt: String,         // 主手术编码(手术病例必填)
    other_dis: Vec<String>,   // 其他诊断编码(列表)
    other_opt: Vec<String>,   // 其他手术编码(列表)
    sex: i32,                 // 性别(0 => 女, 1 => 男)
    age: f64,                 // 年龄(不足一岁以小于1小数表示, 出生天数/365)
    weight: i32,              // 体重
    all_dis: HashSet<String>, // 所有的诊断
    all_opt: HashSet<String>, // 所有的手术
}

impl DrgCase {
    // 初始化方法
    fn new(
        admission_number: String,
        principal_diagnosis: String,
        principal_operation: String,
        other_diagnosis: Vec<String>,
        other_operation: Vec<String>,
        gender: i32,
        old: f64,
        mass: i32,
    ) -> Self {
        let mut tmp_other_dis = other_diagnosis.clone();
        let mut tmp_other_opt = other_operation.clone();
        tmp_other_dis.push(principal_diagnosis.clone());
        tmp_other_opt.push(principal_operation.clone());
        Self {
            id: admission_number,
            main_dis: principal_diagnosis,
            main_opt: principal_operation,
            other_dis: other_diagnosis,
            other_opt: other_operation,
            sex: gender,
            age: old,
            weight: mass,
            all_dis: HashSet::from_iter(tmp_other_dis), // 初始化为主诊断+其他诊断
            all_opt: HashSet::from_iter(tmp_other_opt), // 初始化为主手术操作+其他手术操作
        }
    }

    // 检查病例数是否有主诊断
    fn no_main_diagnosis(&self) -> bool {
        return self.main_dis == "";
    }

    // 检查病例是否有主手术
    fn no_surgery(&self) -> bool {
        return self.main_opt == "";
    }

    // 检查病例是否有其他手术
    fn no_other_surgery(&self) -> bool {
        return self.other_opt.len() == 0;
    }

    // 检查病例是否有其他诊断
    fn no_other_diagnosis(&self) -> bool {
        return self.other_dis.len() == 0;
    }

    // 合并主诊断与其他诊断为一个set
    fn concat_dis(&mut self) {
        let mut temp_dis = self.other_dis.clone();
        let principle_dis = self.main_dis.clone();
        self.all_dis.insert(principle_dis);
        temp_dis.retain(|r| self.all_dis.insert(r.to_string()))
    }

    // 检查病例是否是有效的手术病例
    fn is_vaild_surgrey(&self, all_dis_list: &HashSet<String>) -> bool {
        all_dis_list.contains(&self.main_opt)
    }

}

// 用于读取CSV文件并初始化结构体
#[derive(Debug, Deserialize)]
struct TempDrgCase {
    id: String,               // 病例ID
    main_dis: String,         // 主诊断编码(必填)
    main_opt: String,         // 主手术编码(手术病例必填)
    #[serde(deserialize_with = "custom_deserializer::deserialize_sep_str")]
    other_dis: Vec<String>,   // 其他诊断编码(列表)
    #[serde(deserialize_with = "custom_deserializer::deserialize_sep_str")]
    other_opt: Vec<String>,   // 其他手术编码(列表)
    #[serde(deserialize_with = "custom_deserializer::deserialize_i32")]
    sex: i32,                 // 性别(0 => 女, 1 => 男)
    #[serde(deserialize_with = "custom_deserializer::deserialize_f64")]
    age: f64,                 // 年龄(不足一岁以小于1小数表示, 出生天数/365)
    #[serde(deserialize_with = "custom_deserializer::deserialize_i32")]
    weight: i32,              // 体重
}

// 用于存放分组完了以后的数据
#[derive(Debug, Serialize)]
struct DrgCaseGrouped {
    id: String,               // 病例ID
    main_dis: String,         // 主诊断编码(必填)
    main_opt: String,         // 主手术编码(手术病例必填)
    other_dis: String,        // 其他诊断编码(列表)
    other_opt: String,        // 其他手术编码(列表)
    sex: String,                 // 性别(0 => 女, 1 => 男)
    age: String,                 // 年龄(不足一岁以小于1小数表示, 出生天数/365)
    weight: String,              // 体重
    code: String,             // 分组编码
}

impl DrgCaseGrouped {
    // 重新定义一个初始化方法
    fn new(drgcase: DrgCase, code: String) -> Self {
        let other_dis_str = drgcase.other_dis.join("|");   // 合并其他诊断用"|"分隔
        let other_opt_str = drgcase.other_opt.join("|");   // 合并其他诊断用"|"分隔
        DrgCaseGrouped { 
            id: drgcase.id, 
            main_dis: drgcase.main_dis, 
            main_opt: drgcase.main_opt, 
            other_dis: other_dis_str, 
            other_opt: other_opt_str, 
            sex: drgcase.sex.to_string(), 
            age: drgcase.age.to_string(), 
            weight: drgcase.weight.to_string(), 
            code,
        }
    }
}



// CSV读取的相关操作==================================================================
// 自定义反序列化
mod custom_deserializer {
    use serde::{self, Deserialize, Deserializer};

    // i32类型的反序列化
    pub fn deserialize_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // 移除逗号、空格等
        let clean_str = s.replace(",", "").trim().to_string();
        
        // 尝试转换为数字
        clean_str.parse::<i32>()
            .map_err(serde::de::Error::custom)
    }
    
    // 以"|"为分隔符的文本的反序列化
    pub fn deserialize_sep_str<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
    
    // f64类型的反序列化
    pub fn deserialize_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        
        // 处理空字符串或纯空白
        if s.trim().is_empty() {
            return Ok(0.0);
        }

        // 清理字符串：移除空格和千位分隔符
        let clean_str = s
            .replace(" ", "")
            .replace(",", "")
            .trim()
            .to_string();

        // 尝试解析数字
        match clean_str.parse::<f64>() {
            Ok(num) => Ok(num),
            Err(e) => Err(serde::de::Error::custom(format!("Failed to parse float: {}", e)))
        }
    }
}


// 读取CSV数据
fn read_csv(file_path: &str) -> Result<Vec<DrgCase>, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path(file_path)?;
    let mut case_vec: Vec<DrgCase> = Vec::new();    
    for result in rdr.deserialize() {
        let record: TempDrgCase = result?;
        let case: DrgCase = DrgCase::new(
            record.id, 
            record.main_dis, 
            record.main_opt, 
            record.other_dis, 
            record.other_opt, 
            record.sex, 
            record.age, 
            record.weight
        );
        case_vec.push(case)
    }
    Ok(case_vec)
}


// 写入CSV数据
fn write_csv(drgcases: Vec<DrgCaseGrouped>, file_path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(file_path)?;                       // 创建文件路径
    let mut wrt = csv::Writer::from_writer(file); // 初始化写入模块
    for d in drgcases {
        // 逐行写入
        wrt.serialize(d)?;
    }
    wrt.flush()?;         // 确保数据被写入
    println!("Grouped data is write into your path");
    Ok(())
}

// 判断病例所进入的MDC============================
// 先期分组
fn is_mdca(
    record: &DrgCase,                                // 病例
    adrg_dis_opt: &HashMap<String, HashSet<String>>, // ADRG诊断手术表
    all_opt_list: &HashSet<String>,                  // 全部手术列表
    adrg_type_dict: &HashMap<String, String>,        // ADRG类型及对应入组类型
    mdc_name: String,
) -> String {
    if record.no_surgery() {
        return String::from("KBBZ");
    }
    let mut pred = String::from("KBBZ");
    // 因为MDCA没有主诊表，所以这里要判断病例是否进入MDCA下的ADRG
    let adrg_list = vec![
        "AA1", "AA2", "AB1", "AC1", "AD1", "AE1", "AF1", "AG1", "AG2", "AG3", "AH1", "AH2",
    ];
    // 遍历MDCA下的ADRG
    for cate in adrg_list {
        pred = process_adrg(
            record,
            adrg_dis_opt,
            all_opt_list,
            adrg_type_dict,
            cate.to_string(),
        );
        if pred != "KBBZ" {
            break;
        }
    }
    return pred;
}

// MDCZ先期分组
fn is_mdcz(
    record: &DrgCase,                                // 病例
    adrg_dis_opt: &HashMap<String, HashSet<String>>, // ADRG诊断手术表
    all_opt_list: &HashSet<String>,                  // 全部手术列表
    adrg_type_dict: &HashMap<String, String>,   // ADRG类型及对应入组类型
    mdcz_dis_sheet: &HashMap<String, HashSet<String>>,   // MDC主诊表
    mdc_name: String,
) -> String {
    let tmp_adrg = "ZZ1".to_string();
    let pred = is_mdcz_dis(record, mdcz_dis_sheet, tmp_adrg);
    if pred == "ZZ1" {
        return String::from("MDCZ");
    } else {
        return String::from("KBBZ");
    }
}

// MDCP先期分组
fn is_mdcp(
    record: &DrgCase,                                // 病例
    main_dis_sheet: &HashMap<String, Vec<String>>,   // MDC主诊表
    mdc_name: String,
) -> String {
    // BUG 国家版的分组方案里面MDCP居然没有主诊表
    if record.age <= 0.0795 {
        // 新生儿要求为出生距今29天内的，29 / 365 ≈ 0.0795
        return String::from("MDCP");
    } else {
        return String::from("KBBZ");
    }
}

// MDCY先期分组
fn is_mdcy(
    record: &DrgCase,                               // 病例结构体
    adrg_type_dict: &HashMap<String, String>,       // ADRG类型及对应入组类型
    mdcy_dis_sheet: &HashSet<String>,
    mdc_name: String,
) -> String {
    if mdcy_dis_sheet.is_disjoint(&record.all_dis) {
        return String::from("KBBZ");
    } else {
        return String::from("MDCY");
    }
}

// 特殊的MDCN判断性别
fn is_mdcn(
    record: &DrgCase,                                // 病例
    main_dis_sheet: &HashMap<String, Vec<String>>,   // MDC主诊表
    mdc_name: String,
) -> String {
    // 判断性别为女sex为0
    if (record.sex == 0) && (main_dis_sheet[&record.main_dis][0] == "MDCN") {
        return String::from("MDCY");
    } else {
        return String::from("KBBZ");
    }
}

// 特殊的MDCN判断性别
fn is_mdcm(
    record: &DrgCase,                                // 病例
    main_dis_sheet: &HashMap<String, Vec<String>>,   // MDC主诊表
    mdc_name: String,
) -> String {
    // 判断性别为男sex为1
    if (record.sex == 1) && (main_dis_sheet[&record.main_dis][0] == "MDCM") {
        return String::from("MDCY");
    } else {
        return String::from("KBBZ");
    }
}

// 普通MDC判断
fn is_common_mdc(
    record: &DrgCase,
    main_dis_sheet: &HashMap<String, Vec<String>>,
    mdc_name: String
) -> String {
    // 如果病例的主诊断在MDC主诊表中
    if main_dis_sheet[&record.main_dis][0].to_string() == mdc_name {
        return mdc_name
    } else {
        return String::from("KBBZ")
    }
}

// 各ADRG入组方式===================================
// 包含主手术
fn is_contain_main_opt(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    if record.no_surgery() {
        // 无主手术的无法入组
        return String::from("KBBZ");
    }
    if adrg_dis_opt[&adrg_name].contains(&record.main_opt) {
        adrg_name
    } else {
        String::from("KBBZ")
    }
}

// 同时有两手术
fn is_contain_opt_simultaneously(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_opt1: String = adrg_name.to_string() + "_normal_list"; // 手术条件表1
    let verb_opt2: String = adrg_name.to_string() + "_other_list"; // 手术条件表2

    if record.no_surgery() {
        // 如果没有手术则为空白病组
        return String::from("KBBZ");
    }
    if (!adrg_dis_opt[&verb_opt1].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt2].is_disjoint(&record.all_opt))
    {
        adrg_name
    } else {
        String::from("KBBZ")
    }
}

// 其他诊断或手术或操作1+手术或操作2
fn is_contain_other_dis_or_other_opt1_and_other_opt2(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_other_dis = adrg_name.to_string() + "_other_dis_list";
    let verb_opt1 = adrg_name.to_string() + "_other_opt_list1";
    let verb_opt2 = adrg_name.to_string() + "_other_opt_list2";
    // 为了方便后续的对比, 需要将其他诊断列表转为HashSet
    let tmp_other_dis_set = record
        .other_dis
        .iter()
        .map(|x| x.clone())
        .collect::<HashSet<String>>();

    if record.no_surgery() {
        return String::from("KBBZ");
    }
    if ((!adrg_dis_opt[&verb_other_dis].is_disjoint(&tmp_other_dis_set))
        || (!adrg_dis_opt[&verb_opt1].is_disjoint(&record.all_opt)))
        && (!adrg_dis_opt[&verb_opt2].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 同时包含主要诊断与主要手术
fn is_contain_main_dis_and_main_opt_simultaneously(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_opt = adrg_name.to_string() + "_contain_main_opt_list";
    let verb_main_dis = adrg_name.to_string() + "_contain_main_dis_list";
    if record.no_surgery() {
        // 无手术的病例无法入组
        return String::from("KBBZ");
    }
    if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含主要诊断
fn is_contain_main_dis(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    if adrg_dis_opt[&adrg_name].contains(&record.main_dis) {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 同时包含CB4与CB5手术, CB2入组使用
fn is_contain_cb4_opt_and_cb5_opt(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    if record.no_surgery() {
        return String::from("KBBZ");
    }
    if (!adrg_dis_opt["CB4"].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt["CB5"].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 同时包含CB5与CB6手术, CB3入组使用
fn is_contain_cb5_opt_and_cb6_opt(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    if record.no_surgery() {
        return String::from("KBBZ");
    }
    if (!adrg_dis_opt["CB4"].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt["CB5"].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 入组条件1：主要诊断+主要手术或操作1,
// 入组条件2：主要手术或操作2
// 入组条件3：手术或操作3+手术或操作4
fn is_contain_multi_opt1(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_dis = adrg_name.to_string() + "_main_dis_list";
    let verb_main_opt1 = adrg_name.to_string() + "_main_opt_list1";
    let verb_main_opt2 = adrg_name.to_string() + "_main_opt_list2";
    let verb_opt3 = adrg_name.to_string() + "_other_opt_list3";
    let verb_opt4 = adrg_name.to_string() + "_other_opt_list4";
    if record.no_surgery() {
        return String::from("KBBZ");
    }

    if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (adrg_dis_opt[&verb_main_opt1].contains(&record.main_opt))
    {
        return adrg_name;
    } else if adrg_dis_opt[&verb_main_opt2].contains(&record.main_opt) {
        return adrg_name;
    } else if (!adrg_dis_opt[&verb_opt3].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt4].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 入组条件1：主要诊断+手术或操作1+手术或操作2
// 入组条件2：主要诊断+手术或操作1+手术或操作3+手术或操作4
// 入组条件3：主要诊断+手术或操作4+手术或操作5
fn is_contain_multi_opt2(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_dis = adrg_name.to_string() + "_main_dis_list";
    let verb_opt1 = adrg_name.to_string() + "_other_opt_list1";
    let verb_opt2 = adrg_name.to_string() + "_other_opt_list2";
    let verb_opt3 = adrg_name.to_string() + "_other_opt_list3";
    let verb_opt4 = adrg_name.to_string() + "_other_opt_list4";
    let verb_opt5 = adrg_name.to_string() + "_other_opt_list5";

    if record.no_surgery() {
        return String::from("KBBZ");
    }

    if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (!adrg_dis_opt[&verb_opt1].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt2].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (!adrg_dis_opt[&verb_opt1].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt3].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt4].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (!adrg_dis_opt[&verb_opt4].is_disjoint(&record.all_opt))
        && (!adrg_dis_opt[&verb_opt5].is_disjoint(&record.all_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 入组条件1：主要诊断+主要手术或操作1
// 入组条件2：主要诊断+手术或操作2+手术或操作3
fn is_contain_multi_opt3(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_dis = adrg_name.to_string() + "_main_dis_list";
    let verb_main_opt1 = adrg_name.to_string() + "_main_opt_list1";
    let verb_main_opt2 = adrg_name.to_string() + "_other_opt_list2";
    let verb_opt3 = adrg_name.to_string() + "_other_opt_list3";

    if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (adrg_dis_opt[&verb_main_opt1].contains(&record.main_opt))
    {
        return adrg_name;
    } else if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && ((adrg_dis_opt[&verb_main_opt2].contains(&record.main_opt))
            || (!adrg_dis_opt[&verb_main_opt2].is_disjoint(&record.all_opt)))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 入组条件1：主要诊断1+主要手术或操作
// 入组条件2：主要诊断2+其他诊断+主要手术或操作
fn is_contain_multi_opt4(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_dis1 = adrg_name.to_string() + "_main_dis_list1";
    let verb_main_dis2 = adrg_name.to_string() + "_main_dis_list2";
    let verb_main_opt = adrg_name.to_string() + "_main_opt_list";
    let verb_other_dis = adrg_name.to_string() + "_other_dis_list";

    // 将其他诊断转为HashSet
    let tmp_other_dis_set = record
        .other_dis
        .iter()
        .map(|x| x.clone())
        .collect::<HashSet<String>>();

    if (adrg_dis_opt[&verb_main_dis1].contains(&record.main_dis))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else if (adrg_dis_opt[&verb_main_dis2].contains(&record.main_dis))
        && (!adrg_dis_opt[&verb_other_dis].is_disjoint(&tmp_other_dis_set))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 入组条件1：主要诊断+其他诊断1+主要手术或操作
// 入组条件2：其他诊断2+主要手术或操作
fn is_contain_multi_opt5(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_dis = adrg_name.to_string() + "_main_dis_list";
    let verb_main_opt = adrg_name.to_string() + "_main_opt_list";
    let verb_other_dis1 = adrg_name.to_string() + "_other_dis_list1";
    let verb_other_dis2 = adrg_name.to_string() + "_other_dis_list2";
    // 将其他诊断转为HashSet
    let tmp_other_dis_set = record
        .other_dis
        .iter()
        .map(|x| x.clone())
        .collect::<HashSet<String>>();

    // 无主手术的病例进入空白组
    if record.no_surgery() {
        return String::from("KBBZ");
    }
    // 入组判断
    if (adrg_dis_opt[&verb_main_dis].contains(&record.main_dis))
        && (!adrg_dis_opt[&verb_other_dis1].is_disjoint(&tmp_other_dis_set))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else if (!adrg_dis_opt[&verb_other_dis2].is_disjoint(&tmp_other_dis_set))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含 WB1、WB2、WB3的所有主要手术或操作
fn is_contain_multi_wb_opt(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_main_opt1 = adrg_name.to_string() + "WB1_main_opt_list";
    let verb_main_opt2 = adrg_name.to_string() + "WB2_main_opt_list";
    let verb_main_opt3 = adrg_name.to_string() + "WB3_main_opt_list";

    if record.no_surgery() {
        return String::from("KBBZ");
    }
    if (adrg_dis_opt[&verb_main_opt1].contains(&record.main_opt))
        || (adrg_dis_opt[&verb_main_opt2].contains(&record.main_opt))
        || (adrg_dis_opt[&verb_main_opt3].contains(&record.main_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含其他诊断
fn is_contain_other_dis(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    // 将其他诊断转为HashSet
    let tmp_other_dis_set = record
        .other_dis
        .iter()
        .map(|x| x.clone())
        .collect::<HashSet<String>>();
    if !adrg_dis_opt[&adrg_name].is_disjoint(&tmp_other_dis_set) {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含主诊断或其他诊断
fn is_contain_dis(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    // 将其他诊断转为HashSet
    let tmp_other_dis_set = record
        .other_dis
        .iter()
        .map(|x| x.clone())
        .collect::<HashSet<String>>();
    if (adrg_dis_opt[&adrg_name].contains(&record.main_dis))
        && (!adrg_dis_opt[&adrg_name].is_disjoint(&tmp_other_dis_set))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含所有手术()
fn is_contain_all_opt(record: &DrgCase, all_opt: &HashSet<String>, adrg_name: String) -> String {
    // 如果没有手术则进入空白病组
    if record.no_surgery() {
        return String::from("KBBZ");
    }

    if !all_opt.is_disjoint(&record.all_opt) {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含诊断与主手术入组, WB3入组使用
fn is_contain_dis_and_main_opt(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    let verb_dis = adrg_name.to_string() + "_main_dis_list";
    let verb_main_opt = adrg_name.to_string() + "_main_opt_list";
    if (!adrg_dis_opt[&verb_dis].is_disjoint(&record.all_dis))
        && (adrg_dis_opt[&verb_main_opt].contains(&record.main_opt))
    {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 包含MDCZ的诊断，ZZ1入组使用
fn is_mdcz_dis(
    record: &DrgCase,
    mdcz_dis_opt: &HashMap<String, HashSet<String>>,
    adrg_name: String,
) -> String {
    // BUG 判断有问题
    let mut counter = 0;
    // 遍历不同部分的诊断表
    for cate in vec![
        "belly_dis_sheet",
        "body_spine_dis_sheet",
        "chest_dis_sheet",
        "down_limb_dis_sheet",
        "genital_dis_sheet",
        "head_neck_dis_sheet",
        "pelvis_dis_sheet",
        "up_limb_dis_sheet",
        "urinary_dis_sheet",
    ] {
        // 主诊断或其他诊断位于多个不同部分的诊断表中
        if !mdcz_dis_opt[&cate.to_string()].is_disjoint(&record.all_dis)
        {
            counter += 1;
        }
    }
    if counter > 1 {
        return adrg_name;
    } else {
        return String::from("KBBZ");
    }
}

// 处理每个MDC大类
fn process_mdc(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,
    all_opt_list: &HashSet<String>,
    main_dis_sheet: &HashMap<String, Vec<String>>,
    mdcy_dis_sheet: &HashSet<String>,
    mdcz_dis_sheet: &HashMap<String, HashSet<String>>,
    adrg_type_dict: &HashMap<String, String>,
) -> String {
    let mut pred_mdc = String::from("KBBZ");
    // 逐个处理各MDC(优先处理:MDCA,MDCZ,MDCY,MDCP)
    pred_mdc = is_mdca(record, adrg_dis_opt, all_opt_list, adrg_type_dict, String::from("MDCA"));
    // 优先处理MDCA
    if &pred_mdc == "MDCA" {
        return pred_mdc
    }

    // 优先处理MDCZ
    pred_mdc = is_mdcz(record, adrg_dis_opt, all_opt_list, adrg_type_dict, mdcz_dis_sheet, String::from("MDCZ"));
    if &pred_mdc == "MDCZ" {
        return pred_mdc
    }

    // 优先处理MDCY
    pred_mdc = is_mdcy(record, adrg_type_dict, mdcy_dis_sheet, String::from("MDCY"));
    if &pred_mdc == "MDCY" {
        return pred_mdc
    }

    // 优先处理MDCP
    pred_mdc = is_mdcp(record, main_dis_sheet, String::from("MDCP"));
    if &pred_mdc == "MDCY" {
        return pred_mdc
    }

    // 优先处理MDCN
    pred_mdc = is_mdcn(record, main_dis_sheet, String::from("MDCN"));
    if &pred_mdc == "MDCN" {
        return pred_mdc
    }

    // 优先处理MDCM
    pred_mdc = is_mdcm(record, main_dis_sheet, String::from("MDCM"));
    if &pred_mdc == "MDCM" {
        return pred_mdc
    }
    // 遍历其他普通入组的MDC
    for m in vec![
        "MDCB", "MDCC", "MDCD", "MDCE", "MDCF", "MDCG",
        "MDCH", "MDCI", "MDCJ", "MDCK", "MDCL", "MDCO",
        "MDCQ", "MDCR", "MDCS", "MDCT", "MDCU", "MDCV",
        "MDCW", "MDCX", "MDCY"
    ] {
        pred_mdc = is_common_mdc(record, main_dis_sheet, m.to_string());
        if &pred_mdc == m {
            break;
        }
    }
    return pred_mdc;

}

// 处理每个ADRG入组
fn process_adrg(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>, // ADRG诊断手术表
    all_opt_list: &HashSet<String>,                  // 全部手术列表
    adrg_type_dict: &HashMap<String, String>,   // ADRG类型及对应入组类型
    // mdcz_dis_sheet: &HashMap<String, HashSet<String>>, // MDCZ诊断表
    adrg_name: String,
) -> String {
    let pred_adrg = match adrg_type_dict[&adrg_name].as_str() {
        "is_contain_main_dis" => is_contain_main_dis(record, adrg_dis_opt, adrg_name),
        "is_contain_main_opt" => is_contain_main_opt(record, adrg_dis_opt, adrg_name),
        "is_contain_main_dis_and_main_opt_simultaneously" => {
            is_contain_main_dis_and_main_opt_simultaneously(record, adrg_dis_opt, adrg_name)
        }
        "is_contain_dis" => is_contain_dis(record, adrg_dis_opt, adrg_name),
        "is_contain_opt_simultaneously" => {
            is_contain_opt_simultaneously(record, adrg_dis_opt, adrg_name)
        }
        "is_contain_all_opt" => is_contain_all_opt(record, all_opt_list, adrg_name),
        "is_contain_multi_opt3" => is_contain_multi_opt3(record, adrg_dis_opt, adrg_name),
        "is_contain_other_dis" => is_contain_other_dis(record, adrg_dis_opt, adrg_name),
        "is_contain_multi_opt5" => is_contain_multi_opt5(record, adrg_dis_opt, adrg_name),
        "is_contain_other_dis_or_other_opt1_and_other_opt2" => {
            is_contain_other_dis_or_other_opt1_and_other_opt2(record, adrg_dis_opt, adrg_name)
        }
        "is_contain_cb4_opt_and_cb5_opt" => {
            is_contain_cb4_opt_and_cb5_opt(record, adrg_dis_opt, adrg_name)
        }
        "is_contain_cb5_opt_and_cb6_opt" => {
            is_contain_cb5_opt_and_cb6_opt(record, adrg_dis_opt, adrg_name)
        }
        "is_contain_multi_opt1" => is_contain_multi_opt1(record, adrg_dis_opt, adrg_name),
        "is_contain_multi_opt2" => is_contain_multi_opt2(record, adrg_dis_opt, adrg_name),
        "is_contain_multi_opt4" => is_contain_multi_opt4(record, adrg_dis_opt, adrg_name),
        "is_dis_and_main_opt" => is_contain_dis_and_main_opt(record, adrg_dis_opt, adrg_name),
        "is_contain_multi_wb_opt" => is_contain_multi_wb_opt(record, adrg_dis_opt, adrg_name),
        "is_mdcz_dis" => is_mdcz_dis(record, adrg_dis_opt, adrg_name), // 注意这里需要的是MDCZ的各部分诊断表
        // 默认情况返回空白组
        _ => String::from("KBBZ"),
    };

    pred_adrg
}


struct DrgFunc {}
impl DrgFunc {
    // 判断是否为QY
    fn is_qy(adrg_name: &String) -> bool {
        &adrg_name[1..=2] == "QY"
    }

    fn drg_type(adrg_name: String) -> String {
        let surgery = vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
        let operation = vec!["K", "L", "M", "N", "O", "P", "Q"];
        let internal = vec!["R", "S", "T", "U", "V", "W", "X", "Y", "Z"];
        if adrg_name == "KBBZ".to_string() {
            "KBBZ".to_string()
        } else if DrgFunc::is_qy(&adrg_name) {
            "QY".to_string()
        } else if surgery.contains(&&adrg_name[1..=1]) {
            "surgery".to_string()
        } else if operation.contains(&&adrg_name[1..=1]) {
            "operation".to_string()
        } else if internal.contains(&&adrg_name[1..=1]) {
            "internal".to_string()
        } else {
            "other".to_string()
        }
    }
}


fn qy_judge(record: &DrgCase, adrg_name: String, all_opt_list: &HashSet<String>) -> String {
    // 判断QY
    let internal = vec!["R", "S", "T", "U", "V", "W", "X", "Y", "Z"];
    // 如果预测ADRG为KBBZ，则直接返回
    if adrg_name == "KBBZ".to_string() {
        return "KBBZ".to_string()
    }
    if record.is_vaild_surgrey(all_opt_list) {
        if internal.contains(&&adrg_name[1..=1]) {
            // 如果手术有效但是又进入了内科组，则判定为QY
            let res_adrg = adrg_name[0..=0].to_string() + &("QY".to_string()); 
            return res_adrg
        } else {
            return adrg_name
        }
    } else {
        // 如果当前病例的手术无效，且预测的ADRG不为KBBZ，则返回当前预测ADRG
        return adrg_name
    }    
}

fn which_adrg(
    record: &DrgCase,
    adrg_dis_opt: &HashMap<String, HashSet<String>>,   // ADRG诊断手术表
    all_opt_list: &HashSet<String>,                    // 全部手术列表
    all_dis_list: &HashSet<String>,                    // 所有手术列表
    main_dis_sheet: &HashMap<String, Vec<String>>,     // MDC主诊断列表
    adrg_type_dict: &HashMap<String, String>,          // ADRG类型及对应入组类型
    mdcz_dis_sheet: &HashMap<String, HashSet<String>>, // MDCZ诊断表
    mdcy_dis_sheet: &HashSet<String>,                  // MDCY诊断表
    mdc_sub_adrg: &HashMap<String, Vec<String>>,   // MDC下的各个ADRG
) -> Result<String, Box<dyn std::error::Error>> {
    // 决定进入哪个ADRG
    let mut pred_adrg = "KBBZ".to_string();
    let mut pred_mdc = "KBBZ".to_string();

    // 如果没有主诊断则无法入组，直接进入KBBZ
    if record.no_main_diagnosis() {
        return Ok(String::from("KBBZ"))
    }

    // 主诊断所在的MDC
    let mut target_mdc_list = main_dis_sheet[&record.main_dis].clone();
    let pre_mdc = vec![
        String::from("MDCA"),
        String::from("MDCP"),
        String::from("MDCY"),
        String::from("MDCZ"),
    ];
    target_mdc_list = [pre_mdc, target_mdc_list].concat();
    for mdc in target_mdc_list {
        if mdc == String::from("MDCA") {
            // 优先判断MDCA
            pred_adrg = is_mdca(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, String::from("MDCA"));
            if pred_adrg != String::from("KBBZ") {
                // 如果在MDCA中找到ADRG入组
                break
            }
        }        
        else if mdc == String::from("MDCP") {
            // 判断MDCP新生儿
            pred_mdc = is_mdcp(&record, &main_dis_sheet, String::from("MDCP"));
            if pred_mdc == String::from("MDCP") {
                for adrg in mdc_sub_adrg[&pred_mdc].clone() {
                    pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                    if pred_adrg != "KBBZ".to_string() {
                        break
                    }
                }
            }
        }
        else if mdc == String::from("MDCY") {
            // 判断MDCY
            pred_mdc = is_mdcy(&record, &adrg_type_dict, &mdcy_dis_sheet, String::from("MDCY"));
            if pred_mdc == String::from("MDCY") {
                for adrg in mdc_sub_adrg[&pred_mdc].clone() {
                    pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                    if pred_adrg != "KBBZ".to_string() {
                        break
                    }
                }
            }
        }
        else if mdc == String::from("MDCZ") {
            // 判断MDCZ
            pred_mdc = is_mdcz(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, &mdcz_dis_sheet, String::from("MDCZ"));
            if pred_mdc == String::from("MDCZ") {
                // 判断MDC内的ADRG入组
                for adrg in mdc_sub_adrg[&pred_mdc].clone() {
                    pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                    if pred_adrg != "KBBZ".to_string() {
                        break
                    }
                }
            }
        }
        else if mdc == String::from("MDCN") {
            // 需要判断性别的MDCN的处理
            pred_mdc = is_mdcn(record, main_dis_sheet, "MDCN".to_string());
            if pred_mdc == String::from("MDCN") {
                // 判断MDC内的ADRG入组
                for adrg in mdc_sub_adrg[&pred_mdc].clone() {
                    pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                    if pred_adrg != "KBBZ".to_string() {
                        break
                    }
                }
            }
        }
        else if mdc == String::from("MDCM") {
            // 需要判断性别的MDCM的处理
            pred_mdc = is_mdcm(record, main_dis_sheet, "MDCM".to_string());
            if pred_mdc == String::from("MDCM") {
                // 判断MDC内的ADRG入组
                for adrg in mdc_sub_adrg[&pred_mdc].clone() {
                    pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                    if pred_adrg != "KBBZ".to_string() {
                        break
                    }
                }
            }
        }
        else {
            // 处理其他MDC
            for adrg in mdc_sub_adrg[&mdc].clone() {
                pred_adrg = process_adrg(&record, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg);
                if pred_adrg != String::from("KBBZ") {
                    break
                }
            }
        }

    }
    pred_adrg = qy_judge(record, pred_adrg, all_opt_list);
    Ok(pred_adrg)
}


fn process_drg(
    record: &DrgCase, 
    adrg_name: String, 
    ccmcc_sheet: &HashMap<String, Vec<String>>,
    exclude_sheet: &HashMap<String,String>,
    adrg_drg_name_sheet: &HashMap<String, Vec<String>>,
) -> Result<String, Box<dyn Error>> {
    if (&adrg_name == "KBBZ") || (&adrg_name[1..=2] == "QY") {
        let res = adrg_name.clone();
        return Ok(res)
    }
    // 判定CCMCC并决定进入哪个DRG
    let mut pred_drg = "KBBZ".to_string();
    let drg_wait_dict: HashMap<i32, String> = adrg_drg_name_sheet
        .get(&adrg_name)
        .unwrap()
        .into_iter()
        .map(|x| (x.chars().last().unwrap_or_default().to_digit(10).unwrap() as i32, x.to_string()))
        .collect();

    // 病例其他诊断与CCMMC列表的交集
    let case_ccmcc = record.other_dis.iter()
        .map(|x| ccmcc_sheet.get(x))
        .filter(|x| !x.is_none())
        .collect::<Vec<_>>();

    // 如果没有CCMCC
    if drg_wait_dict.len() == 1 {
        // 如果当前ADRG下只有一个DRG那么DRG结尾必然只有9
        pred_drg = drg_wait_dict[&9].clone();
    }
    else if drg_wait_dict.len() == 2 {
        // 如果当前ADRG下有两个DRG，
        if case_ccmcc.is_empty() {
            // 该病例无并发症，则DRG结尾为5
            pred_drg = drg_wait_dict[&5].clone();
        } else {
            // 有并发症的情况需要考虑是否被主诊断排除
            let mut exclude_label = "exclude";
            for c in case_ccmcc {
                if &c.unwrap()[0] == exclude_sheet.get(&record.main_dis).unwrap_or(&String::from("")) {
                    // 如果被排除了则继续寻找下一个其他诊断
                    continue;
                } else {
                    exclude_label = &c.unwrap()[1].as_str();
                    // 如果找到了MCC就停止
                    if exclude_label == "MCC" { break; } else { continue; }
                }
            }
            if exclude_label == "MCC" {
                // 如果存在MCC
                // ADRG只分1和5的时候，有MCC进入1，没有MCC进入5
                if drg_wait_dict.contains_key(&1) {
                    pred_drg = drg_wait_dict[&1].clone();
                } else {
                    pred_drg = drg_wait_dict[&3].clone();
                }
            } 
            else if exclude_label == "CC" {
                // 如果只有CC
                if drg_wait_dict.contains_key(&1) {
                    // ADRG只分1和5的时候，有CC只能进入5
                    pred_drg = drg_wait_dict[&5].clone();
                } else {
                    pred_drg = drg_wait_dict[&3].clone();
                }
            }
            else {
                // 没有有效CCMCC的情况下返回结尾为5的DRG
                pred_drg = drg_wait_dict[&5].clone();
            }
        }
    } 
    else {
        let mut high_ccmcc_label = "exclude";    // 默认
        for c in case_ccmcc {
           if &c.unwrap()[0] == exclude_sheet.get(&record.main_dis).unwrap_or(&String::from("")) {
               // 如果并发症被排除了，则继续寻找
               continue;
           } 
           else {
               if c.unwrap()[1] == "MCC" {
                   // 如果有MCC为被排除，则无需继续寻找，此时病例最高的并发症类型为MCC
                   high_ccmcc_label = "MCC";
                   break;
               } 
               else {
                   // 并发症类型为CC，继续寻找是否有MCC
                   high_ccmcc_label = c.unwrap()[0].as_str();  
               }
            }
        }
        if high_ccmcc_label == "MCC" {
            // 如果并发症类型为MCC，则DRG以1结尾
            pred_drg = drg_wait_dict[&1].clone();
        } else if high_ccmcc_label == "CC" {
            // 如果并发症类型为CC
            if drg_wait_dict.len() == 3 {
                // 当前ADRG下有3个DRG时，CC病例的DRG以3结尾
                pred_drg = drg_wait_dict[&3].clone();
            } else {
                // 当前ADRG下有2个DRG时，CC病例的DRG以1结尾(意味着1与3合并了)
                pred_drg = drg_wait_dict[&1].clone();
            }
        } else {
            // 无CC和MCC，则DRG结尾为5
            pred_drg = drg_wait_dict[&5].clone();
        }
    } 
    Ok(pred_drg)
}


// 批量分组
fn batch_drg_process(case_vec: Vec<DrgCase>, out_file_path: &str) -> Result<(), Box<dyn Error>> {
    // 读取分组方案数据
    // ADRG内涵诊断和手术操作表
    let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    // 所有手术操作列表
    let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    // 所有诊断列表
    let all_dis_list = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    // 各个MDC的主诊表
    let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    // MDCY的诊断表
    let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    // MDCZ的诊断表
    let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    // 各个ADRG组进入的判断条件
    let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();
    // 读取MDC下的ADRG列表
    let mdc_sub_adrg = read_file_as_str_to_tuple("data\\mdc_sub_adrg.json").unwrap();
    // 读取CCMCC列表
    let ccmcc_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\ccmcc_sheet.json").unwrap();
    // 读取排除表
    let exclude_sheet = read_file_as_str_to_str("D:\\MyScript\\rust\\DrgGrouper\\data\\exclude_sheet.json").unwrap();
    // 读取ADRG下的DRG
    let adrg_drg_name_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\adrg_drg_name_sheet.json").unwrap();
    
    let mut drg_grouped_vec: Vec<DrgCaseGrouped> = Vec::new();
    // 批量分组
    for case in case_vec {
        // 判断最终属于的ADRG
        let result_adrg = which_adrg(
            &case, 
            &adrg_dis_opt, 
            &all_opt_list, 
            &all_dis_list, 
            &main_dis_sheet, 
            &adrg_type_dict, 
            &mdcz_dis_sheet, &mdcy_dis_sheet, 
            &mdc_sub_adrg
        ).unwrap();

        // 判断属于的DRG
        let result_drg = process_drg(
            &case,
            result_adrg,
            &ccmcc_sheet,
            &exclude_sheet,
            &adrg_drg_name_sheet
        ).unwrap();

        // 初始化需要写入的病例类型结构
        let c_wtr = DrgCaseGrouped::new(case, result_drg);
        drg_grouped_vec.push(c_wtr);
    }
    // 写入为CSV文件到本地
    write_csv(drg_grouped_vec, out_file_path)?;

    Ok(())
}


// 单独分组
fn single_drg_process(drgcase: DrgCase) -> Result<String, Box<dyn Error>> {
    // 读取分组方案数据
    // ADRG内涵诊断和手术操作表
    let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    // 所有手术操作列表
    let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    // 所有诊断列表
    let all_dis_list = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    // 各个MDC的主诊表
    let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    // MDCY的诊断表
    let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    // MDCZ的诊断表
    let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    // 各个ADRG组进入的判断条件
    let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();
    // 读取MDC下的ADRG列表
    let mdc_sub_adrg = read_file_as_str_to_tuple("data\\mdc_sub_adrg.json").unwrap();
    // 读取CCMCC列表
    let ccmcc_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\ccmcc_sheet.json").unwrap();
    // 读取排除表
    let exclude_sheet = read_file_as_str_to_str("D:\\MyScript\\rust\\DrgGrouper\\data\\exclude_sheet.json").unwrap();
    // 读取ADRG下的DRG
    let adrg_drg_name_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\adrg_drg_name_sheet.json").unwrap();
    // 判断最终属于的ADRG
    let result_adrg = which_adrg(
        &drgcase, 
        &adrg_dis_opt, 
        &all_opt_list, 
        &all_dis_list, 
        &main_dis_sheet, 
        &adrg_type_dict, 
        &mdcz_dis_sheet, &mdcy_dis_sheet, 
        &mdc_sub_adrg
    ).unwrap();
    // 判断最终属于的DRG
    let result_drg = process_drg(
        &drgcase,
        result_adrg,
        &ccmcc_sheet,
        &exclude_sheet,
        &adrg_drg_name_sheet
    ).unwrap();

    Ok(result_drg)
    
}


fn main() -> Result<(), Box<dyn Error>> {
    // 收集命令行参数
    let args: Vec<String> = env::args().collect();
    match args[1].as_str() {
        "--single" => {
            // 单病例模式
            let id = args[2].to_string();
            let main_dis = args[3].to_string();
            let main_opt = args[4].to_string();
            let other_dis = args[5].split("|").map(|x| x.to_string()).collect::<Vec<String>>();
            let other_opt = args[6].split("|").map(|x| x.to_string()).collect::<Vec<String>>();
            let sex = args[7].parse::<i32>()?;
            let age = args[8].parse::<f64>()?;
            let weight = args[9].parse::<i32>()?;
            // 初始化病例结构
            let case = DrgCase::new(
                id, 
                main_dis, 
                main_opt, 
                other_dis, 
                other_opt, 
                sex, 
                age, 
                weight,
            );
            let drg_code = single_drg_process(case)?;
            println!("result drg code is {}", drg_code);
        }
        "--batch" => {
            // 批量分组
            let in_file_path = args[2].as_str();
            let out_file_path = args[3].as_str();
            // 读取需要分组的病案数据
            let cases_vec = read_csv(in_file_path).unwrap();
            // 批量分组
            batch_drg_process(cases_vec, out_file_path).unwrap();
            println!("Batch group is done, save at {}", out_file_path);
        }
        _ => { println!("wrong input please check your input!!!") }
    }
    Ok(())
}

// 功能测试=======================================
#[cfg(test)]
mod tests {
    use std::{process::CommandArgs};
    use super::*;

    // #[test]
    // fn read_adrg_dis_opt() {
    //     // 测试读取ADRG诊断手术表是否正常
    //     let res: HashMap<String, HashSet<String>> =
    //         read_json_file("data\\adrg_dis_opt_sheet.json").unwrap();
    //     // res.expect("Reading File wrong???");
    //     // println!("{:?}", &res["K85.001"]);
    //     assert_eq!(true, res["AA1"].contains("33.6x00"));
    // }

    // #[test]
    // fn read_adrg_to_drg() {
    //     // 测试读取ADRG下的DRG分组列表
    //     let res: HashMap<String, HashSet<String>> = read_json_file("data\\adrg_drg_name_sheet.json").unwrap();
    //     assert_eq!(true, res["AA2"].contains("AA29"));
    // }

    // #[test]
    // fn read_all_icd9_and_10() {
    //     // 测试读取所有诊断表或手术表是否正常
    //     let res: HashSet<String> = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    //     println!("length of the file is {}", res.len());
    //     let shit: Vec<String> = res.clone().iter().map(|x| x.to_string()).collect();
    //     println!("the second element is {}", shit[1]);
    //     // let test_verb = &shit[2];
    //     let test_verb = String::from("A84.000x001");
    //     assert_eq!(true, res.contains(&test_verb));
    // }

    // #[test]
    // fn read_mdc_main_dis() {
    //     // 测试读取MDC主诊断表
    //     let res = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let test_verb = "A00.100x001";
    //     println!("target mdc is {}", res[test_verb][0]);
    //     assert_eq!(true, res[test_verb][0] == "MDCG");
    // }

    // #[test]
    // fn read_exclude_sheet() {
    //     // 读取主诊断排除表
    //     let res = read_file_as_str_to_str("data\\exclude_sheet.json").unwrap();
    //     assert_eq!(true, res["A01.000x014"] == "表6-3-1");
    // }

    // #[test]
    // fn mdcz_group_test() {
    //     // 进入MDC测试

    //     // 读取数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

    //     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
    //     // 初始化病例
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("03.9202"),
    //         vec![String::from("B20.700x001"), String::from("S21.100x002")],
    //         vec![],
    //         1,
    //         20.0,
    //         2288
    //     );

    //     let res = is_mdcz(
    //         &case,
    //         &adrg_dis_opt,
    //         &all_opt_list,
    //         &adrg_type_dict,
    //         &mdcz_dis_sheet,
    //         String::from("MDCZ")
    //     );

    //     println!("all dis is {:?}", case.all_dis);
    //     println!("all opt is {:?}", case.all_opt);
    //     println!("{}", res);
    //     assert_eq!(true, res == String::from("MDCZ"));
    // }

    // #[test]
    // fn mdcy_group_test() {
    //     // 进入MDC测试

    //     // 读取数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

    //     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
    //     // 初始化病例
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("03.9202"),
    //         vec![String::from("B20.000x001"), String::from("S21.100x002")],
    //         vec![],
    //         1,
    //         20.0,
    //         2288
    //     );

    //     let res = is_mdcy(
    //         &case, 
    //         &adrg_type_dict, 
    //         &mdcy_dis_sheet, 
    //         String::from("MDCY")
    //     );
    //     println!("all dis is {:?}", case.all_dis);
    //     println!("all opt is {:?}", case.all_opt);
    //     let c = mdcy_dis_sheet.intersection(&case.all_dis);
    //     let f = mdcy_dis_sheet.is_disjoint(&case.all_dis);
    //     println!("{}", f);
    //     println!("{:?}", c.into_iter().map(|x| x.to_string()).collect::<Vec<String>>().len());
    //     println!("{}", res);
    //     assert_eq!(true, res == String::from("MDCY"));
    // }

    // #[test]
    // fn mdcp_group_test() {
    //     // 进入MDC测试

    //     // 读取数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

    //     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
    //     // 初始化病例
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("03.9202"),
    //         vec![String::from("B20.000x001"), String::from("S21.100x002")],
    //         vec![],
    //         1,
    //         0.05,
    //         2288
    //     );

    //     let res = is_mdcp(
    //         &case,
    //         &main_dis_sheet,
    //         String::from("MDCP")
    //     );
    //     println!("all dis is {:?}", case.all_dis);
    //     println!("all opt is {:?}", case.all_opt);
    //     let c = case.age <= 0.0795;
    //     println!("{}", res);
    //     println!("{}", case.age);
    //     println!("{}", c);
    //     assert_eq!(true, res == String::from("MDCP"));
    // }

    // #[test]
    // fn mdcp_group_test() {
    //     // 进入MDC测试

    //     // 读取数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

    //     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
    //     // 初始化病例
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("03.9202"),
    //         vec![String::from("B20.000x001"), String::from("S21.100x002")],
    //         vec![],
    //         1,
    //         0.05,
    //         2288
    //     );

    //     let res = is_mdcp(
    //         &case,
    //         &main_dis_sheet,
    //         String::from("MDCP")
    //     );
    //     println!("all dis is {:?}", case.all_dis);
    //     println!("all opt is {:?}", case.all_opt);
    //     let c = case.age <= 0.0795;
    //     println!("{}", res);
    //     println!("{}", case.age);
    //     println!("{}", c);
    //     assert_eq!(true, res == String::from("MDCP"));
    // }

    // #[test]
    // fn mdcp_group_test() {
    //     // 读取分组方案数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

    //     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
    //     // 初始化病例
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("41.0100"),
    //         vec![String::from("B20.000x001"), String::from("S21.100x002")],
    //         vec![String::from("52.8000"), String::from("55.6901")],
    //         1,
    //         20.0,
    //         2288
    //     );

    //     let res = is_mdca(
    //         &case,
    //         &adrg_dis_opt,
    //         &all_opt_list,
    //         &adrg_type_dict,
    //         String::from("MDCA")
    //     );
    //     println!("all dis is {:?}", case.all_dis);
    //     println!("all opt is {:?}", case.all_opt);
    //     println!("{}", res);
    //     assert_eq!(true, res == String::from("MDCA"));
    // }

    // #[test]
    // fn mdcp_group_test() {
    //     // 读取分组方案数据
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
//     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
//     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
//     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
//     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
//     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();

//     let test_other_dis: Vec<String> = vec!["S35.200x005", "S21.100x002"].iter().map(|x| x.to_string()).collect();
//     // 初始化病例
//     let case = DrgCase::new(
//         String::from("0001"),
//         String::from("G12.900"),
//         String::from("41.0100"),
//         vec![String::from("B20.000x001"), String::from("S21.100x002")],
//         vec![String::from("52.8000"), String::from("55.6901")],
//         1,
//         20.0,
//         2288
//     );

//     let res = is_mdca(
//         &case,
//         &adrg_dis_opt,
//         &all_opt_list,
//         &adrg_type_dict,
//         String::from("MDCA")
//     );
//     println!("all dis is {:?}", case.all_dis);
//     println!("all opt is {:?}", case.all_opt);
//     println!("{}", res);
//     assert_eq!(true, res == String::from("AC1"));
// }

    // #[test]
    // fn test_adrg() {
    //     // 读取分组方案数据

    //     // ADRG内涵诊断和手术操作表
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     // 所有手术操作列表
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     // 所有诊断列表
    //     let all_dis_list = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    //     // 各个MDC的主诊表
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     // MDCY的诊断表
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     // MDCZ的诊断表
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     // 各个ADRG组进入的判断条件
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();
    //     // 读取MDC下的ADRG列表
    //     let mdc_sub_adrg = read_file_as_str_to_set("data\\mdc_sub_adrg.json").unwrap();

    //     // 初始化病例结构
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("03.9202"),
    //         vec![String::from("M41.900")],
    //         vec![],
    //         1,
    //         20.0,
    //         2288
    //     );
    //     // MDC列表
    //     let mdc_list = vec![
    //         "MDCA", "MDCP", "MDCY", "MDCZ", "MDCB", "MDCC", "MDCD", 
    //         "MDCE", "MDCF", "MDCG", "MDCH", "MDCI", "MDCJ", "MDCK", "MDCL", 
    //         "MDCM", "MDCN", "MDCO", "MDCQ", "MDCR", "MDCS", "MDCT", "MDCU", 
    //         "MDCV", "MDCW", "MDCX"].iter_mut().map(|x| x.to_string()).collect::<Vec<String>>();

    //     // 无效主诊断，病例进入KBBZ
    //     if case.no_main_diagnosis() {
    //         println!("No main dis no adrg group in result is {}", "KBBZ");
    //     }

    //     // 主诊断所在的MDC
    //     let mut target_mdc_list = main_dis_sheet[&case.main_dis].clone();
    //     let pre_mdc = vec![String::from("MDCA"), String::from("MDCP"), String::from("MDCY"), String::from("MDCZ")];
    //     target_mdc_list = [pre_mdc, target_mdc_list].concat();
    //     println!("{:?}", target_mdc_list);
        
    //     let mut pred_adrg = String::from("KBBZ");
    //     let mut pred_mdc = String::from("KBBZ");
    //     for mdc in target_mdc_list {
    //         if mdc == String::from("MDCA") {
    //             // 优先判断MDCA
    //             pred_adrg = is_mdca( &case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, String::from("MDCA"));
    //             if pred_adrg != String::from("KBBZ") {
    //                 // 如果在MDCA中找到ADRG入组
    //                 println!("predict mdc is {} and is adrg is {}", mdc, pred_adrg);
    //                 break
    //             }
    //         }
    //         else if mdc == String::from("MDCP") {
    //             // 判断MDCP
    //             pred_mdc = is_mdcp(&case, &main_dis_sheet, String::from("MDCP"));
    //             if pred_mdc == String::from("MDCP") {
    //                 for adrg in mdc_sub_adrg[&pred_mdc].clone() {
    //                     pred_adrg = process_adrg(&case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg); 
    //                     if pred_adrg != "KBBZ".to_string() {
    //                         println!("predict mdc is {} and is adrg is {}", mdc, pred_adrg);
    //                         break
    //                     }
    //                 }
            
    //             }
    //         }
    //         else if mdc == String::from("MDCY") {
    //             // 判断MDCY
    //             pred_mdc = is_mdcy(&case, &adrg_type_dict, &mdcy_dis_sheet, String::from("MDCY"));
    //             if pred_mdc == String::from("MDCY") {
    //                 for adrg in mdc_sub_adrg[&pred_mdc].clone() {
    //                     pred_adrg = process_adrg(&case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg); 
    //                     if pred_adrg != "KBBZ".to_string() {
    //                         println!("predict mdc is {} and is adrg is {}", mdc, pred_adrg);
    //                         break
    //                     }
    //                 }
                
    //             }
    //         }
    //         else if mdc == String::from("MDCZ") {
    //             // 判断MDCZ
    //             pred_mdc = is_mdcz(&case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, &mdcz_dis_sheet, String::from("MDCZ"));
    //             if pred_mdc == String::from("MDCZ") {
    //                 // 判断MDC内的ADRG入组
    //                 for adrg in mdc_sub_adrg[&pred_mdc].clone() {
    //                     pred_adrg = process_adrg(&case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg); 
    //                     if pred_adrg != "KBBZ".to_string() {
    //                         println!("predict mdc is {} and is adrg is {}", mdc, pred_adrg);
    //                         break
    //                     }
    //                 }
    //             }
    //         }
    //         else {
    //             // 处理其他MDC
    //             for adrg in mdc_sub_adrg[&mdc].clone() {
    //                 pred_adrg = process_adrg(&case, &adrg_dis_opt, &all_opt_list, &adrg_type_dict, adrg); 
    //                 if pred_adrg != String::from("KBBZ") {
    //                     println!("predict mdc is {} and is adrg is {}", mdc, pred_adrg);
    //                     break
    //                 }
    //             }
    //         }
    //     }
    //     println!("The final predict adrg is {}", pred_adrg);
    //     assert_eq!(true, pred_adrg == "AH1");
    // }

    // #[test]
    // fn test_adrg() {        
    //     // ADRG内涵诊断和手术操作表
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     // 所有手术操作列表
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     // 所有诊断列表
    //     let all_dis_list = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    //     // 各个MDC的主诊表
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     // MDCY的诊断表
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     // MDCZ的诊断表
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     // 各个ADRG组进入的判断条件
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();
    //     // 读取MDC下的ADRG列表
    //     let mdc_sub_adrg = read_file_as_str_to_set("data\\mdc_sub_adrg.json").unwrap();
    //     // 初始化病例结构

    //     // 初始化病例结构
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("31.7400x0001"),
    //         vec![String::from("M41.900")],
    //         vec![],
    //         1,
    //         20.0,
    //         2288
    //     );

    //     // 判断最终属于的ADRG
    //     let result_adrg = which_adrg(
    //         &case, 
    //         &adrg_dis_opt, 
    //         &all_opt_list, 
    //         &all_dis_list, 
    //         &main_dis_sheet, 
    //         &adrg_type_dict, 
    //         &mdcz_dis_sheet, &mdcy_dis_sheet, 
    //         &mdc_sub_adrg
    //     ).unwrap();
    //     println!("result adrg is {}", result_adrg);
    //     assert_eq!(true, result_adrg == String::from("BU2"));
    // }

    // #[test]
    // fn test_drg() {        
    //     // ADRG内涵诊断和手术操作表
    //     let adrg_dis_opt = read_file_as_str_to_set("data\\adrg_dis_opt_sheet.json").unwrap();
    //     // 所有手术操作列表
    //     let all_opt_list = read_icd9_to_vec("data\\all_opt_sheet.txt").unwrap();
    //     // 所有诊断列表
    //     let all_dis_list = read_icd9_to_vec("data\\all_dis_sheet.txt").unwrap();
    //     // 各个MDC的主诊表
    //     let main_dis_sheet = read_file_as_str_to_tuple("data\\main_dis_sheet.json").unwrap();
    //     // MDCY的诊断表
    //     let mdcy_dis_sheet = read_icd9_to_vec("data\\mdcy_dis_sheet.txt").unwrap();
    //     // MDCZ的诊断表
    //     let mdcz_dis_sheet = read_file_as_str_to_set("data\\mdcz_dis_sheet.json").unwrap();
    //     // 各个ADRG组进入的判断条件
    //     let adrg_type_dict = read_file_as_str_to_str("data\\adrg_in_condition.json").unwrap();
    //     // 读取MDC下的ADRG列表
    //     let mdc_sub_adrg = read_file_as_str_to_set("data\\mdc_sub_adrg.json").unwrap();
    //     // 读取CCMCC列表
    //     let ccmcc_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\ccmcc_sheet.json").unwrap();
    //     // 读取排除表
    //     let exclude_sheet = read_file_as_str_to_str("D:\\MyScript\\rust\\DrgGrouper\\data\\exclude_sheet.json").unwrap();
    //     // 读取ADRG下的DRG
    //     let adrg_drg_name_sheet = read_file_as_str_to_tuple("D:\\MyScript\\rust\\DrgGrouper\\data\\adrg_drg_name_sheet.json").unwrap();


    //     // 初始化病例结构
    //     let case = DrgCase::new(
    //         String::from("0001"),
    //         String::from("G12.900"),
    //         String::from("31.7400x0001"),
    //         vec![String::from("M41.900")],
    //         vec![],
    //         1,
    //         20.0,
    //         2288
    //     );

    //     // 判断最终属于的ADRG
    //     let result_adrg = which_adrg(
    //         &case, 
    //         &adrg_dis_opt, 
    //         &all_opt_list, 
    //         &all_dis_list, 
    //         &main_dis_sheet, 
    //         &adrg_type_dict, 
    //         &mdcz_dis_sheet, &mdcy_dis_sheet, 
    //         &mdc_sub_adrg
    //     ).unwrap();
    //     println!("result adrg is {}", result_adrg);

    //     let result_drg = process_drg(
    //         &case,
    //         result_adrg,
    //         &ccmcc_sheet,
    //         &exclude_sheet,
    //         &adrg_drg_name_sheet
    //     ).unwrap();

    //     println!("result drg is {}", result_drg);
    //     assert_eq!(true, result_drg == String::from("BU25"));
    // }

    // #[test]
    // fn test_read_csv() {
        // 测试读取CSV文件
        // let cases_vec = read_csv("D:\\MyScript\\rust\\DrgGrouper\\case_data\\test_case_data.csv").unwrap();
        // for drg_case in &cases_vec {
            // println!("{:?}", drg_case);
        // }
        // assert_eq!(true, cases_vec[0].main_dis == "I50.900x08".to_string());
    // }

    #[test]
    fn test_write_csv() {
        // 测试写入CSV文件
        let cases_vec = read_csv("D:\\MyScript\\rust\\DrgGrouper\\case_data\\test_case_data.csv").unwrap();
        let out_file_path = "D:\\MyScript\\rust\\DrgGrouper\\case_data\\test_result.csv";
        batch_drg_process(cases_vec, out_file_path).unwrap();
        assert_eq!(true, true);
    }
    
}



// DONE: 所有分组方案数据的读取
// DONE: 测试病例结构的初始化
// DONE: 测试进入MDCZ
// DONE: 测试进入MDCY
// DONE: 测试进入MDCP
// DONE: 测试进入MDCA
// DONE: 修复了is_disjonit方法的问题
// DONE: 测试进入MDCA
// DONE: 判断ADRG
// DONE: 判断QY的函数
// DONE: 测试需要判断性别的MDC
// DONE: 写判断进入CCMCC的函数
// DONE: 测试进入DRG
// DONE: 写读取CSV文件批量结构化病例的函数
// DONE: 测试CSV文件的读取
// DONE: 终端的命令行参数控制单个病例分组或者导入表格进行分组


// NOTE 各种不同的读取
/*
1. 读取ADRG诊断手术列表(adrg_dis_opt_sheet) => read_file_as_str_to_set
2. 读取所有诊断列表和手术列表(all_dis_sheet | all_opt_sheet) => read_icd9_to_vec
3. 读取MDC主诊断列表(main_dis_sheet) => read_file_as_str_to_tuple
4. 读取ADRG下的DRG分组编码列表(adrg_drg_name_sheet) => read_json_file
5. 读取CCMCC列表(ccmcc_sheet) => read_json_file
6. 读取主诊断排除表(exclude_sheet) => read_file_as_str_to_str
7. 读取ADRG入组条件列表(adrg_in_condition) => read_file_as_str_to_str
8. 读取MDCY的诊断列表(mdcy_dis_sheet) => read_icd9_to_vec
9. 读取MDCZ的诊断列表(mdcz_dis_sheet) => read_file_as_str_to_set
10. 读取病案CSV数据 => read_csv
*/
