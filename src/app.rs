use std::io::Stdout;
use std::sync::{Arc, OnceLock, RwLock};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};

pub enum AppMessage {
    UserQuery(String),
    AIMsg(AssistantMessage),
    SysMsg(SystemMessage)
}

pub enum AssistantMessage {
    ModelChunk(String),
    AssistantReply(String),
    TaskComplete,
}

pub enum SystemMessage {
    // 命令执行
    ExecCommand(String),
    ExecResult(String),
    // 读取文件
    Read(String),
    ReadResult(String),
    // 应用补丁
    Diff(String, String),
    DiffResult(String),
    // 系统日志
    SystemLog(String),
}

#[derive(Clone)]
pub enum PendingAction {
    None,
    ConfirmExec(String),
    ConfirmDiff(String, String)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[allow(dead_code, non_camel_case_types)]
#[derive(Clone)]
pub enum Model {
    MELCHIOR,
    CASPER_I,
    CASPER_II,
    BALTHAZAR
}

#[allow(dead_code)]
pub enum Tool {
    Exec,
    Read,
    Diff(String),
    Search(String)
}

pub struct Call {
    pub tool: Tool,
    pub content: String,
}

static CURRENT_MODEL: OnceLock<Arc<RwLock<Model>>> = OnceLock::new();

pub fn get_model() -> &'static Arc<RwLock<Model>> {
    CURRENT_MODEL.get_or_init(|| {
        Arc::new(RwLock::new(Model::MELCHIOR))
    })
}

pub const MELCHIOR_PROMPT: &str = include_str!("../prompt/MELCHIOR_INIT_PROMPT.md");
pub const CASPER_I_PROMPT: &str = include_str!("../prompt/CASPER_I_PROMPT.md");
pub const CASPER_II_PROMPT: &str = include_str!("../prompt/CASPER_II_PROMPT.md");

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn get_logo_text() -> String {
    let logo = r#"
  .oooooo.                o8o                            .o8                            .
 d8P'  `Y8b               `"'                           "888                          .o8
   888      888 oooo    ooo oooo   .ooooo.   .ooooo.   .oooo888   .ooooo.  ooo. .oo.   .o888oo
 888      888  `88b..8P'  `888  d88' `"Y8 d88' `88b d88' `888  d88' `88b `888P"Y88b    888
 888      888    Y888'     888  888       888   888 888   888  888ooo888  888   888    888
  `88b    d88'  .o8"'88b    888  888   .o8 888   888 888   888  888    .o  888   888    888 .
        `Y8bood8P'  o88'   888o o888o `Y8bod8P' `Y8bod8P' `Y8bod88P" `Y8bod8P' o888o o888o   "888"     "#;
    format!("{}\n\t:: Oxicodent — A Light Coding Agent ::\t(v{})", logo, env!("CARGO_PKG_VERSION"))
}
