use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;

const ROOT_DIR: &str = ".oxicodent";
const CONFIG_FILENAME: &str = "config.json";
const PROMPT_FILENAME: &str = "prompt.md";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub api_key: String,
    pub api_base: String, // 方便支持 Ollama 或自定义代理
    pub model: String,
}

impl Config {
    /// 加载配置，如果不存在则引导用户创建
    pub fn load_or_init() -> Self {
        let mut path = get_home_path();
        path.push(CONFIG_FILENAME);

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

fn get_home_path() -> PathBuf {
    let mut path = home::home_dir().expect("无法获取主目录");
    path.push(ROOT_DIR);
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

pub fn read_or_create_prompt() -> String {
    let mut path = get_home_path();
    path.push(PROMPT_FILENAME);

    if path.exists() {
        fs::read_to_string(&path).unwrap()
    } else {
        fs::write(&path, get_default_prompt_content()).unwrap();
        println!("已在 ~/{}/{} 写入默认提示词，可随时更改", ROOT_DIR, PROMPT_FILENAME);
        get_default_prompt_content().to_string()
    }

}

fn get_default_prompt_content() -> &'static str {
r#"# Role: Oxicodent Agent 框架

你正运行在一个 Coding Agent 框架中，它可以给你提供简单的工具调用，方便你辅佐用户写代码

## ⚠️ 绝对准则 (Hard Rules)
1. **禁止废话**：除了必要的意图说明，严禁解释代码原理。
2. **原子化操作**：所有的代码变更必须通过 ```exec 使用 `sed`或`patch` 进行修改。
3. **验证闭环**：修改文件后，必须立刻接一个 ```exec cargo check，严禁询问“你是否要检查”。
4. **拆解步骤**：当收到复杂指令时，请尝试将任务拆解成多个步骤，依次执行

## 工具调用
你可以使用 Markdown 代码块 ```exec 在 Bash 中执行命令，事例如下：
```exec
echo "Hello World!"
```
框架会自动检测 ```exec 代码块，并在用户监管下执行

> **注意**：每次输出的对话内容只允许进行一次工具调用，在调用完成后，程序会自动向你输出结果
"#
}