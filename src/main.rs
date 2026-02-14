mod config_manager;
mod api_client;
mod worker;

use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use rustyline::DefaultEditor;
use colored::*;
use crate::api_client::ChatMessage;
use api_client::ApiClient;
use crate::config_manager::*;
use crate::worker::*;

// 定义线程间传输的消息类型
enum AppMessage {
    UserQuery(String),      // 用户输入
    ModelChunk(String),     // 模型返回的文本片段
    SystemLog(String),      // 系统通知
    TaskComplete            // 任务结束
}

fn skip_recv(receiver: &Receiver<AppMessage>) {
    loop {
        if let Ok(msg) = receiver.recv() {
            match msg {
                AppMessage::TaskComplete => break,
                _ => {}
            }
        }
    }
}

fn listen(receiver: &Receiver<AppMessage>) -> Result<String, Box<dyn std::error::Error>> {
    let mut full_msg = String::new();

    loop {
        if let Ok(msg) = receiver.recv() {
            match msg {
                AppMessage::ModelChunk(chunk) => {
                    // 拼凑完整消息 用作命令解析
                    full_msg.push_str(chunk.as_str());

                    // 将消息立刻输出到控制台
                    print!("{}", chunk.white());
                    std::io::Write::flush(&mut std::io::stdout())?;
                },

                AppMessage::SystemLog(log) => {
                    eprintln!("\n[ERROR]: {}", log)
                },

                AppMessage::TaskComplete => break,
                _ => {}
            }
        }
    }

    Ok(full_msg)
}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_logo();

    let (tx_to_io, rx_from_ui) = mpsc::channel::<AppMessage>();
    let (tx_to_ui, rx_from_io) = mpsc::channel::<AppMessage>();

    let client = ApiClient::new(&Config::load_or_init());

    // 1. 启动 IO 线程 (网络请求)
    thread::spawn(move || {
        let mut history: Vec<ChatMessage> = Vec::new(); // 简单的会话历史管理

        while let Ok(msg) = rx_from_ui.recv() {
            if let AppMessage::UserQuery(query) = msg {
                history.push(ChatMessage { role: "user".into(), content: query });
                client.send_chat_stream(history.clone(), tx_to_ui.clone());
            }
        }
    });

    println!("正在注入提示词...\n");

    // TEST: 注入提示词
    tx_to_io.send(AppMessage::UserQuery(read_or_create_prompt()))?;

    // 消耗 Receiver
    skip_recv(&rx_from_io);

    // 2. 主线程：处理 RustyLine 输入和 UI 渲染
    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline(&format!("{}", "🦀 > ".bright_red()));
        let line = match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                line
            },
            Err(_) => break,
        };

        // 状态变量：控制自主循环
        let mut next_input_to_ai = Some(line);

        while let Some(current_query) = &next_input_to_ai {
            // 1. 发送消息（用户输入或上一次的执行结果）
            tx_to_io.send(AppMessage::UserQuery(current_query.to_string()))?;

            // 2. 等待并打印 AI 回复
            println!("\n{} ", "Agent:".bright_cyan());
            let full_msg = listen(&rx_from_io)?;
            println!();

            let tool_call = parse_tool_call(full_msg);

            // 3. 尝试解析工具调用
            if let Some(call) = tool_call {
                match call.tool {
                    Tool::Exec => {
                        if confirm(&mut rl) {
                            let result_feedback = format!(
                                "--- [ exec_result ] ---\n{}-----------------------",
                                exec_cmd(call.content)
                            );

                            println!("{}", "[系统]: 命令已执行，正在自动反馈给 AI...".bright_black());
                            next_input_to_ai = Some(result_feedback); // 触发下一轮 while 循环
                        } else {
                            next_input_to_ai = None;
                            println!();
                        }
                    }
                    _ => { next_input_to_ai = None; println!(); }
                }
            } else { next_input_to_ai = None; println!() }
        }
    }

    Ok(())
}

pub fn confirm(rl: &mut DefaultEditor) -> bool {
    loop {
        let readline = rl.readline(&format!("{}", "\n🦀请审查是否进行此操作 [y/n]> ".bright_red()));
        if let Ok(line) = readline {
            if line.trim() == "y" {
                return true
            } else if line.trim() == "n" {
                return false
            }
        } else { std::process::exit(0) }
    }
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