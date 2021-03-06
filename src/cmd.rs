use crate::file;

use clap::{Arg, App, ArgMatches, SubCommand};
use serde::{Serialize, Deserialize, Deserializer, de::Error};
use log::{info, warn};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecConfig {

    #[serde(default)]
    pub command: String,

    #[serde(default)]
    pub environment_file: String,

    #[serde(deserialize_with = "check_json_file")]
    pub collection_file: String,

    #[serde(default = "default_report_file")]
    pub report_file: String,

    #[serde(default)]
    pub data_file: String,

    #[serde(deserialize_with = "check_non_zero")]
    #[serde(default = "default_to_one")]
    pub thread_count: u64,

    #[serde(default)]
    pub iterations: u64,

    #[serde(default)]
    pub execution_time: u64,

    #[serde(default = "default_to_one")]
    pub thread_delay: u64,

    #[serde(deserialize_with = "check_non_zero")]
    pub rampup_time: u64,
    
    #[serde(default)]
    pub handle_cookies: bool,

    #[serde(default)]
    pub continue_on_error: bool,

    #[serde(default)]
    pub log_to_file: bool,

    #[serde(default)]
    pub influxdb: InfluxDB
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct InfluxDB {
    pub url: String,
    pub username: String,
    pub password: String,
    pub dbname: String,
}

const CONFIG_ARG_NAME: &str = "config json file";
const JSON_EXT: &str = ".json";
const DEFAULT_REPORT_FILE: &str = "report.csv";

fn default_report_file() -> String {
    String::from(DEFAULT_REPORT_FILE)
}

fn default_to_one() -> u64 {
    1
}

fn create_cmd_app<'a, 'b>() -> App<'a, 'b> {
    let config_arg = create_config_arg(CONFIG_ARG_NAME);

    App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .arg(&config_arg))
        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(&config_arg))
}

fn create_config_arg<'a>(arg_name: &'a str) -> Arg<'a, 'a> {
    Arg::with_name(arg_name)
        .short("c")
        .long("config")
        .takes_value(true)
        .required(true)
        .validator(|s: String| {
            match s.ends_with(JSON_EXT) {
                true => Ok(()),
                false => Err(String::from("File should be a .json file"))
            }
        })
        .display_order(0)
        .help("Execution configuration json file")
}

fn get_config_from_file(config_file_path: String) -> Result<ExecConfig, Box<dyn std::error::Error + Send + Sync>> {
    info!("Parsing config file {}", config_file_path);
    
    let content = file::get_content(&config_file_path)?;
    let config: ExecConfig = serde_json::from_str(&content)?;

    if config.execution_time == 0 && config.iterations == 0 {
        return Err("Both execution time and iterations cannot be 0".into());
    }

    if config.execution_time > 0 && config.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(config)
}


pub fn get_config() -> Result<ExecConfig, Box<dyn std::error::Error + Send + Sync>> {
    let matches = create_cmd_app().get_matches();
    let (subcommand, subcommand_args) = matches.subcommand();

    if subcommand == "" {
        return Err("No subcommand found. Should either be 'bombard' or 'report'".into())
    }

    let config_file_path = get_value_as_str(subcommand_args, CONFIG_ARG_NAME); 
    let mut config = get_config_from_file(config_file_path)?;

    config.command = subcommand.to_string(); 
    Ok(config)
}


fn check_non_zero <'de, D>(deserializer: D) -> Result<u64, D::Error> 
where D: Deserializer<'de> {
    
    let val = u64::deserialize(deserializer)?;
    if val == 0 {
        return Err(Error::custom("Value cannot be zero"))
    }

    Ok(val)
}

fn check_json_file <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {
    
    let val = String::deserialize(deserializer)?;
    if !val.ends_with(JSON_EXT)  {
        return Err(Error::custom("File should be a .json file"))
    }

    Ok(val)
}

fn get_value_as_str(matches: Option<&ArgMatches>, arg: &str) -> String {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.to_string(),
                        None => "".to_string()
        },
        None => "".to_string()
    }
}