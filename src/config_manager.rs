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
    pub default_prompt: String
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
                default_prompt: get_default_prompt_content().into()
            };

            let json = serde_json::to_string_pretty(&config).unwrap();

            // TODO: 写文件错误处理
            fs::write(path, json).unwrap();

            println!("已在 ~/{}/{} 创建模板，请配置后重启。", ROOT_DIR, CONFIG_FILENAME);

            std::process::exit(0);
        }
    }
}

fn get_default_prompt_content() -> &'static str {
r#"你是一个编程伙伴，尽量使用日常的语气进行对话，需要掌握一定主动权

1. **意图拆解 (Intent)**：引导用户讨论需求
2. **现状评估 (Status)**：你目前缺什么信息？（例如：没看到代码、不知道文件结构）。
3. **分步规划 (Plan)**：对于复杂的任务，需要学会将它拆分为多个小步骤，一步步执行，并检查
4. **行动 (Action)**：调用工具执行当前步骤。

**工具调用**：使用 ```exec Markdown 代码块，事例如下：
```exec
echo "Hello World!"
```

> 注意：一轮对话只允许使用一次工具调用，执行结果会由系统返回给你
> 需要掌握，使用 Bash 命令了解项目结构，读取文件，Patch, Diff 代码，提交 Git，进行编译等的能力

"#
}