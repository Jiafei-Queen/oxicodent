mod config_manager;
mod api_client;
mod ui;
mod app;
mod io_thread;
mod event_handler;
mod worker_thread;

use crossterm::{
    terminal::{enable_raw_mode, EnterAlternateScreen},
    ExecutableCommand,
};

use std::{io, env, fs};
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use crate::config_manager::*;
use crate::app::*;
use crate::event_handler::handle_event;
use crate::io_thread::IOThread;
use crate::ui::Ui;
use crate::worker_thread::WorkerThread;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 终端初始化 ---
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    // --- 创建 IO 线程 ---
    let mut io_thread = IOThread::spawn();

    // --- 获得当前目录下的条目 ---
    let mut entries = Vec::new();
    for entry in fs::read_dir(".")? {
        let path = entry?.path();
        entries.push(path.file_name().unwrap().to_string_lossy().to_string());
    }

    // 拼接提示词
    let (mut reasoning_prompt, coder_prompt) = read_or_create_prompt();

    reasoning_prompt = format!(
        "--- [ SYSTEM PROMPT ] ---\n{}\n\nCWD: {}\n--- [ DIRS ] ---\n{}----------------\nTHESE AIM TO HELP YOU KNOW ABOUT THE PROJECT",
        reasoning_prompt,
        env::current_dir()?.to_string_lossy().to_string(),
        entries.join("\n")
    );

    io_thread.send(AppMessage::SysMsg(SystemMessage::Prompt(reasoning_prompt, coder_prompt)));

    // --- 创建 Worker 线程 ---
    let mut worker_thread = WorkerThread::spawn();
    
    // --- 创建 UI ---
    let mut ui = Ui::new();
    
    // -------- [ 主循环 ] --------
    loop {
        // --- [ 渲染 TUI ]
        ui.render();
        
        // --- [ 异步消息处理 ] ---
        io_thread.handle_response(&mut ui, &mut worker_thread);
        
        
        // --- [ 工具调用结果处理 ] ---
        worker_thread.handle_response(&mut ui, &mut io_thread);
        
        // ---[ 处理事件监听 ] ---
        if handle_event(&mut ui, &mut io_thread, &mut worker_thread)? { break }
    }

    // --- 恢复终端 ---
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
