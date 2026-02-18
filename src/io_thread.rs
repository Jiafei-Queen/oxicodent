use std::sync::mpsc;
use std::thread;
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

            let mut melchior_history = Vec::<ChatMessage>::new();
            melchior_history.push(ChatMessage { role: "prompt".into(), content: MELCHIOR_PROMPT.into() });

            let mut casper_one_history = Vec::<ChatMessage>::new();
            casper_one_history.push(ChatMessage { role: "prompt".into(), content: CASPER_I_PROMPT.into() });

            let mut casper_two_history = Vec::<ChatMessage>::new();
            casper_two_history.push(ChatMessage { role: "prompt".into(), content: CASPER_II_PROMPT.into() });

            let mut balthazar_history = Vec::<ChatMessage>::new();

            while let Ok(msg) = rx_from_ui.recv() {
                let model = get_model().read().unwrap().clone();
                let history = match model {
                    Model::MELCHIOR => &mut melchior_history,
                    Model::CASPER_I => &mut casper_one_history,
                    Model::CASPER_II => &mut casper_two_history,
                    Model::BALTHAZAR => &mut balthazar_history,
                };

                let mut handle_system_result = |result: String| {
                    let chat_msg = ChatMessage { role: "system".into(), content: result };
                    history.push(chat_msg.clone());
                    client.send_chat_stream(history.clone(), tx_to_ui.clone());
                };

                match msg {
                    AppMessage::UserQuery(content) => {
                        let chat_msg = ChatMessage { role: "user".into(), content };
                        history.push(chat_msg.clone());
                        client.send_chat_stream(history.clone(), tx_to_ui.clone());
                    }
                    AppMessage::AIMsg(AssistantMessage::AssistantReply(content)) => {
                        let chat_msg = ChatMessage { role: "assistant".into(), content };
                        history.push(chat_msg.clone());
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