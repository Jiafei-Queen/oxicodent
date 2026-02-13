mod config_manager;
mod api_client;
mod executor;

use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use rustyline::DefaultEditor;
use colored::*;
use crate::api_client::ChatMessage;
use api_client::ApiClient;
use crate::config_manager::*;
use crate::executor::*;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_logo();

    let (tx_to_io, rx_from_ui) = mpsc::channel::<AppMessage>();
    let (tx_to_ui, rx_from_io) = mpsc::channel::<AppMessage>();

    let config = Config::load_or_init();
    let client = ApiClient::new(&config);
    let prompt = config.default_prompt;

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
    tx_to_io.send(AppMessage::UserQuery(prompt))?;

    // 消耗 Receiver
    skip_recv(&rx_from_io);

    // 2. 主线程：处理 RustyLine 输入和 UI 渲染
    let mut rl = DefaultEditor::new()?;

    loop {
        // 1. 获取用户输入
        let readline = rl.readline(&format!("{}", "🦀 > ".bright_red()));
        let line = match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                line
            },
            Err(_) => break, // 退出
        };

        // 2. 进入“自动迭代”闭环
        let mut next_input = Some(line);

        while let Some(current_query) = next_input {
            // 发送给 IO 线程（记得要在 IO 线程处理 ExecResult，见下文）
            tx_to_io.send(AppMessage::UserQuery(current_query))?;

            // 监听 AI 说话
            let full_msg = listen(&rx_from_io)?;

            // 尝试解析工具调用
            if let Some(call) = parse_tool_call(full_msg) {
                match call.tool {
                    Tool::Exec => {
                        // 构造反馈给 AI 的上下文
                        let result_for_ai = format!(
                            "--- [ exec_result ] ---\n{}-----------------------",
                            call.result
                        );
                        println!("\n{}", "[系统]: 已自动将执行结果反馈给 AI...".bright_black());

                        // 关键：设置下一次循环的内容，不再经过 readline
                        next_input = Some(result_for_ai);
                    }
                    _ => next_input = None,
                }
            } else {
                // 没有工具调用了，彻底结束这一轮，回到顶层 loop 让用户输入
                next_input = None;
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