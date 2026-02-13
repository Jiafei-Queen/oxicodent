mod config_manager;
mod api_client;

use api_client::ApiClient;
use crate::config_manager::*;
use std::sync::mpsc;
use std::thread;
use rustyline::DefaultEditor;
use colored::*;
use crate::api_client::ChatMessage;

// 定义线程间传输的消息类型
enum AppMessage {
    UserQuery(String),      // 用户输入
    ModelChunk(String),     // 模型返回的文本片段
    ExecCommand(String),    // 需要执行的命令
    SystemLog(String),      // 系统通知
    TaskComplete   // 任务结束
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_logo();

    let (tx_to_io, rx_from_ui) = mpsc::channel::<AppMessage>();
    let (tx_to_ui, rx_from_io) = mpsc::channel::<AppMessage>();
    let client = ApiClient::new(&Config::load_or_init());

    // 1. 启动 IO 线程 (网络请求)
    let tx_to_ui_clone = tx_to_ui.clone();
    thread::spawn(move || {
        let mut history: Vec<ChatMessage> = Vec::new(); // 简单的会话历史管理

        while let Ok(msg) = rx_from_ui.recv() {
            if let AppMessage::UserQuery(query) = msg {
                history.push(ChatMessage { role: "user".into(), content: query });
                client.send_chat_stream(history.clone(), tx_to_ui.clone());

                // 这里调用同步的 reqwest 或其他 API 客户端
                // 模拟流式返回
                for chunk in vec!["分析中...", "准备执行...", "```exec\nls\n```"] {
                    tx_to_ui_clone.send(AppMessage::ModelChunk(chunk.to_string())).unwrap();
                    thread::sleep(std::time::Duration::from_millis(200));
                }
                tx_to_ui_clone.send(AppMessage::TaskComplete).unwrap();
            }
        }
    });

    // 2. 主线程：处理 RustyLine 输入和 UI 渲染
    println!("{}", "Oxicodent 已就绪，请输入指令...".bright_green());
    let mut rl = DefaultEditor::new()?;

    loop {
        // 读取输入
        let readline = rl.readline(&format!("{}", "🦀 > ".bright_red()));
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                tx_to_io.send(AppMessage::UserQuery(line))?;

                // 进入 UI 监听循环，直到模型回复完成
                loop {
                    if let Ok(msg) = rx_from_io.recv() {
                        match msg {
                            AppMessage::ModelChunk(chunk) => {
                                // 这里可以接入我们的状态机，实时解析代码块
                                print!("{}", chunk.white());
                                std::io::Write::flush(&mut std::io::stdout())?;
                            }
                            AppMessage::TaskComplete => break, // 这一轮对话结束，回到提示符
                            _ => {}
                        }
                    }
                }
                println!();
            }
            Err(_) => {
                println!("[Oxicodent exited]");
                break;
            }
        }
    }

    Ok(())
}

fn print_logo() {
    println!("\n  .oooooo.                o8o                            .o8                            .   ");
    println!(" d8P'  `Y8b               `\"'                           \"888                          .o8   ");
    println!("888      888 oooo    ooo oooo   .ooooo.   .ooooo.   .oooo888   .ooooo.  ooo. .oo.   .o888oo ");
    println!("888      888  `88b..8P'  `888  d88' `\"Y8 d88' `88b d88' `888  d88' `88b `888P\"Y88b    888   ");
    println!("888      888    Y888'     888  888       888   888 888   888  888ooo888  888   888    888   ");
    println!("`88b    d88'  .o8\"'88b    888  888   .o8 888   888 888   888  888    .o  888   888    888 . ");
    println!(" `Y8bood8P'  o88'   888o o888o `Y8bod8P' `Y8bod8P' `Y8bod88P\" `Y8bod8P' o888o o888o   \"888\" ");
    println!("\t:: Oxicodent — A Light Coding Agent ::\t(v{})\n", env!("CARGO_PKG_VERSION"))
}