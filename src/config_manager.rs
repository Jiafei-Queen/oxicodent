use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::{fs, env};

const ROOT_DIR: &str = ".oxicodent";
const CONFIG_FILENAME: &str = "config.json";

fn get_home_path() -> Result<PathBuf, String> {
    let mut path = env::home_dir().expect("无法获得用户主目录");
    path.push(ROOT_DIR);
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(&path) {
            return Err(format!("无法创建应用配置文件目录 <{}>: {}", &path.to_string_lossy(), e))
        }
    }

    Ok(path)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub api_key: String,
    pub api_base: String, // 方便支持 Ollama 或自定义代理
    pub melchior_model: String,
    pub casper_model: String,
    pub balthazar_model: String,
}

impl Config {
    /// 加载配置，如果不存在则引导用户创建
    pub fn load_or_init() -> Result<Self, String> {
        let mut path = get_home_path()?;
        path.push(CONFIG_FILENAME);

        if path.exists() {
            // TODO: fs 读取文件错误处理
            let content = fs::read_to_string(&path).unwrap();
            match serde_json::from_str(&content) {
                Err(e) =>
                    Err(format!("配置文件 JSON 解析错误 <{}>: {}", &path.to_string_lossy(), e)),
                Ok(c) => Ok(c)
            }
        } else {
            let config = Config {
                api_key: "".into(),
                api_base: "http://127.0.0.1:11434/v1/chat/completions".into(),
                melchior_model: "qwen3-14b-32k:latest".into(),
                casper_model: "qwen2.5-coder-14b-32k:latest".into(),
                balthazar_model: "qwen3-4b-32k-instruct:latest".into()
            };

            let json = serde_json::to_string_pretty(&config)
                .expect("Config 结构体 -> JSON 转换错误");

            if let Err(e) = fs::write(&path, json) {
                return Err(format!("无法写入文件 <{}>: {}", &path.to_string_lossy(), e))
            }

            Err(format!("已在 ~/{}/{} 创建模板，请配置后重启。", ROOT_DIR, CONFIG_FILENAME))
        }
    }
}

pub fn get_melchior_prompt() -> &'static str {
    "MELCHIOR PROMPT FILED BY `mach.lua`"
}

pub fn get_casper_one_prompt() -> &'static str {
    "CASPER I PROMPT FILED BY `mach.lua`"
}

pub fn get_casper_two_prompt() -> &'static str {
    "CASPER II PROMPT FILED BY `mach.lua`"
}
