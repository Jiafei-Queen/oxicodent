use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;

const ROOT_DIR: &str = ".oxicodent";
const CONFIG_FILENAME: &str = "config.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub api_key: String,
    pub api_base: String, // 方便支持 Ollama 或自定义代理
    pub model: String,
}

impl Config {
    /// 获取配置文件路径
    fn get_home_path() -> PathBuf {
        let mut path = home::home_dir().expect("无法获取主目录");
        path.push(ROOT_DIR);
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        path.push(CONFIG_FILENAME);
        path
    }

    /// 加载配置，如果不存在则引导用户创建
    pub fn load_or_init() -> Self {
        let path = Self::get_home_path();
        if path.exists() {
            // TODO: fs 读取文件错误处理
            let content = fs::read_to_string(path).unwrap();

            // TODO: JSON 加载错误处理
            serde_json::from_str(&content).unwrap()
        } else {
            println!("{}", "首次运行：未发现配置文件。");

            // 这里可以触发一个交互式提示，让用户输入 API Key
            let config = Config {
                api_key: "YOUR_API_KEY".into(),
                api_base: "https://api.anthropic.com/v1".into(),
                model: "claude-3-5-sonnet-20241022".into(),
            };

            let json = serde_json::to_string_pretty(&config).unwrap();

            // TODO: 写文件错误处理
            fs::write(path, json).unwrap();

            println!("已在 ~/{}/{} 创建模板，请配置后重启。", ROOT_DIR, CONFIG_FILENAME);

            std::process::exit(0);
        }
    }
}