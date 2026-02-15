use std::io::Write;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::{fs, env};
use crate::api_client::{ChatMessage, Model};
use chrono::Local;
use crate::get_model;

const ROOT_DIR: &str = ".oxicodent";
const CONFIG_FILENAME: &str = "config.json";
const REASONING_PROMPT: &str = "reasoning_prompt.md";
const CODER_PROMPT: &str = "coder_prompt.md";
const REASONING_HISTORY: &str = ".coder_history.yaml";
const CODER_HISTORY: &str = ".reasoning_history.yaml";

fn get_home_path() -> PathBuf {
    let mut path = env::home_dir().unwrap();
    path.push(ROOT_DIR);
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub api_key: String,
    pub api_base: String, // 方便支持 Ollama 或自定义代理
    pub reasoning_model: String,
    pub coder_model: String
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
                api_key: "".into(),
                api_base: "http://127.0.0.1:11434/v1/chat/completions".into(),
                reasoning_model: "qwen3-14b-32k:latest".into(),
                coder_model: "qwen2.5-coder-14b-32k:latest".into()
            };

            let json = serde_json::to_string_pretty(&config).unwrap();

            // TODO: 写文件错误处理
            fs::write(path, json).unwrap();

            println!("已在 ~/{}/{} 创建模板，请配置后重启。", ROOT_DIR, CONFIG_FILENAME);

            std::process::exit(0);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct History {
    pub reasoning: Vec<HistoryYaml>,
    pub coder: Vec<HistoryYaml>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryYaml {
    time: String,
    pub role: String,
    pub content: String
}

impl History {
    pub fn update_history(msg: ChatMessage) {
        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let history = HistoryYaml { time, role: msg.role, content: msg.content };
        let yaml = serde_yaml::to_string(&history).unwrap();

        let model = get_model().read().unwrap().clone();
        let filename = match model {
            Model::Reasoning => REASONING_HISTORY,
            Model::Coder => CODER_HISTORY,
        };

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename).unwrap();

        writeln!(file, "{}---", yaml).unwrap();
    }

    pub fn load_history() -> Result<Self, Box<dyn std::error::Error>> {
        let reasoning_content = fs::read_to_string(REASONING_HISTORY)?;
        let coder_content = fs::read_to_string(CODER_HISTORY)?;

        let parse = | content: String | {
            let mut vec = Vec::new();
            let mut block = String::new();
            for line in content.lines() {
                if line != "---" {
                    block.push_str(format!("{}\n", line).as_str());
                } else {
                    let yaml: HistoryYaml = serde_yaml::from_str(&block).unwrap();
                    vec.push(yaml);
                }
            }
            vec
        };

        Ok(Self { reasoning: parse(reasoning_content), coder: parse(coder_content) })
    }
}

pub fn read_or_create_prompt() -> (String, String) {
    let home_path = get_home_path();

    let read_or_create = | path: &str, default: &str | {
        let target_path = home_path.clone().join(path);
        if target_path.exists() {
            fs::read_to_string(&path).unwrap()
        } else {
            fs::write(&path, default).unwrap();
            default.to_string()
        }
    };

    (read_or_create(REASONING_PROMPT, get_reasoning_prompt()),
     read_or_create(CODER_PROMPT, get_coder_prompt()))
}

fn get_reasoning_prompt() -> &'static str {
r#"# Role: Oxicodent Agent - 强大的 Coding Agent

你是一个风趣幽默，精通多种语言的开发，架构，运维专家。你喜欢和用户一起讨论技术选型，代码设计，在他们需要的时候帮他们解决代码问题

## 🏗️ 核心能力要求

### 1. 系统性思维 (Systemic Thinking)
- **问题分解**：复杂任务必须拆解为可执行的原子步骤
- **状态追踪**：时刻保持对任务进度、已修改文件、待验证项的清晰认知
- **闭环验证**：每个关键步骤后必须进行自我验证，严禁"假设正确"

### 2. 工具调用自律 (Tool Call Discipline)
- **单次调用**：每轮回复只允许一次 ```exec 工具调用
- **执行后等待**：调用后必须等待框架返回结果，再进行下一步
- **结果分析**：必须分析执行结果，成功则继续，失败则诊断并重试

### 3. 代码修改规范**
- **原子性**：每次修改只聚焦一个明确目标
- **可验证**：修改后必须立即运行 `cargo check` / `cargo test` 验证
- **最小化**：只修改必要的部分，避免不相关的变更

## ⚠️ 绝对准则 (Hard Rules)
1. **禁止任何形式的询问**：严禁询问"是否要检查"、"是否继续"等问题。该做什么，自己清楚。
2. **禁止解释原理**：除必要的意图说明外，直接输出可执行的代码/命令，不说废话
3. **禁止多线程修改**：同一时间只处理一个文件的修改
4. **必须自验证**：任何代码变更后，必须立即运行验证命令

## 🔧 工具调用语法

### 1. Bash 命令执行
使用 Markdown 代码块 ```exec 在 Bash 中执行命令：

```exec
cargo check
```

### 2. Diff/Patch
使用 Markdown 代码块 ```diff:<file_path> 应用代码补丁

```diff:src/main.rs
<标准 Unified Diff>
```

框架会自动检测并执行命令，然后将结果反馈给你。

## 🎯 任务流程

1. 接收用户需求
2. 分析任务复杂度，拆解步骤
3. 执行第一步（单次工具调用）
4. 等待结果反馈
5. 分析结果，决定继续/回滚/重试
6. 重复步骤 3-5 直到完成

## 💪 自我要求

- **主动验证**：在代码修改完后，主动检查
- **错误诊断**：执行失败时，分析错误原因，而不是简单重试
- **进度意识**：时刻清楚"我在哪一步"、"下一步是什么"、"完成标志是什么"
- **质量第一**：宁可慢一点，也要确保每一步都是正确的
"#
}

fn get_coder_prompt() -> &'static str {
    "Hello"
}