use std::sync::mpsc;
use std::{env, thread};
use std::fs;
use tracing::info;
use crate::api_client::ApiClient;
use crate::app::*;
use crate::ui::Ui;
use crate::worker_thread::{parse_tool_call, WorkerThread};

pub struct IOThread {
    tx_to_io: mpsc::Sender<AppMessage>,
    rx_from_io: mpsc::Receiver<AppMessage>,
}

impl IOThread {
    pub fn send(&self, msg: AppMessage) {
        self.tx_to_io.send(msg).unwrap()
    }

    /* -------- [ 创建 IO 线程 ] -------- */
    pub fn spawn() -> Result<IOThread, String> {
        let (tx_to_io, rx_from_ui) = mpsc::channel();
        let (tx_to_ui, rx_from_io) = mpsc::channel();

        thread::spawn(move || {
            let client = match ApiClient::new() {
                Err(e) => { eprintln!("{}", e); std::process::exit(1) }
                Ok(c) => c
            };

            let mut history = History::new(&client, tx_to_ui.clone());

            while let Ok(msg) = rx_from_ui.recv() {
                let mut handle_system_result = |result: String| {
                    let chat_msg = ChatMessage { role: "system".into(), content: result };
                    history.push(chat_msg);
                    history.send(&client, tx_to_ui.clone())
                };

                match msg {
                    AppMessage::UserQuery(content) => {
                        let chat_msg = ChatMessage { role: "user".into(), content };
                        history.push(chat_msg);
                        history.send(&client, tx_to_ui.clone());
                    }
                    AppMessage::AIMsg(AssistantMessage::AssistantReply(content)) => {
                        let chat_msg = ChatMessage { role: "assistant".into(), content };
                        history.push(chat_msg);
                    }
                    AppMessage::SysMsg(SystemMessage::ExecResult(result)) => {
                        handle_system_result(result);
                    }
                    AppMessage::SysMsg(SystemMessage::ReadResult(result)) => {
                        handle_system_result(result);
                    }
                    AppMessage::SysMsg(SystemMessage::DiffResult(result)) => {
                        handle_system_result(result);
                    }
                    _ => {}
                }
            }
        });

        Ok(IOThread { tx_to_io, rx_from_io })
    }

    /*
     * -------- [ 异步消息处理 ] --------
     * 从 IO 线程获取 Assistant 的回复，并进行处理
     */
    pub fn handle_response(&mut self, ui: &mut Ui, worker_thread: &mut WorkerThread) {
        if let Ok(msg) = self.rx_from_io.try_recv() {
            match msg {
                AppMessage::AIMsg(AssistantMessage::ModelChunk(chunk)) => {
                    ui.current_ai_response.push_str(&chunk);
                    ui.auto_scroll();
                }

                AppMessage::AIMsg(AssistantMessage::TaskComplete) => {
                    let full_msg = ui.current_ai_response.clone();
                    // 刷新屏幕显示
                    ui.history_display.push_str(&format!("\nASSISTANT:\n{}\n", full_msg));
                    // 清空当前正在生成的回复，避免重复显示
                    ui.current_ai_response.clear();
                    // 更新 AGENT 输出上下文
                    self.send(AppMessage::AIMsg(AssistantMessage::AssistantReply(full_msg.clone())));

                    /*
                     * --------[ 这里触发解析工具调用 ] --------
                     */
                    if let Some(call) = parse_tool_call(full_msg) {
                        info!("正在处理工具调用");
                        match call.tool {
                            Tool::Exec =>
                                ui.pending_action = PendingAction::ConfirmExec(call.content),
                            Tool::Read =>
                                worker_thread.send(AppMessage::SysMsg(SystemMessage::Read(call.content))),
                            Tool::Diff(file_path) =>
                                ui.pending_action = PendingAction::ConfirmDiff(file_path, call.content),
                            _ => {}
                        }
                    }
                }

                AppMessage::SysMsg(SystemMessage::SystemLog(log)) =>
                    ui.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }
    }
}

struct History {
    melchior_history: Vec<ChatMessage>,
    casper_i_history: Vec<ChatMessage>,
    casper_ii_history: Vec<ChatMessage>,
    balthazar_history: Vec<ChatMessage>
}

impl History {
    fn match_history(&mut self) -> &mut Vec<ChatMessage> {
        match get_model().read().unwrap().clone() {
            Model::MELCHIOR => &mut self.melchior_history,
            Model::CASPER_I => &mut self.casper_i_history,
            Model::CASPER_II => &mut self.casper_ii_history,
            Model::BALTHAZAR => &mut self.balthazar_history
        }
    }

    pub fn new(client: &ApiClient, sender: mpsc::Sender<AppMessage>) -> Self {
        let to_msg = |content: String| {
            ChatMessage { role: "system".into(), content }
        };

        let cwd = env::current_dir().unwrap().to_string_lossy().to_string();
        let mut paths = cwd.clone();
        for entry in fs::read_dir(".").unwrap() {
            paths.push_str("\n|-- ");
            paths.push_str(entry.unwrap().path().to_str().unwrap());
        }

        info!("MELCHIOR 提示词目录结构：\n```\n{}\n```", paths);

        let mut melchior_history = Vec::<ChatMessage>::new();
        melchior_history.push(to_msg(MELCHIOR_PROMPT.replace("{{ENTRIES}}", paths.as_str()).to_string()));
        let mut casper_i_history = Vec::<ChatMessage>::new();
        casper_i_history.push(to_msg(CASPER_I_PROMPT.to_string()));
        let mut casper_ii_history = Vec::<ChatMessage>::new();
        casper_ii_history.push(to_msg(CASPER_II_PROMPT.to_string()));

        let history = Self {
            melchior_history,
            casper_i_history,
            casper_ii_history,
            balthazar_history: Vec::<ChatMessage>::new()
        };

        history.send(client, sender);
        history
    }

    pub fn push(&mut self, msg: ChatMessage) {
        Self::match_history(self).push(msg)
    }

    pub fn send(&self, api_client: &ApiClient, sender: mpsc::Sender<AppMessage>) {
        let history = match get_model().read().unwrap().clone() {
            Model::MELCHIOR => self.melchior_history.clone(),
            Model::CASPER_I => self.casper_i_history.clone(),
            Model::CASPER_II => self.casper_ii_history.clone(),
            Model::BALTHAZAR => self.balthazar_history.clone()
        };

        api_client.send_chat_stream(history, sender);
    }
}