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

use std::io;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use crate::config_manager::*;
use crate::app::*;
use crate::event_handler::handle_event;
use crate::io_thread::IOThread;
use crate::ui::Ui;
use crate::worker_thread::WorkerThread;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 创建 IO 线程 ---
    let mut io_thread = IOThread::spawn()?;

    // --- 创建 Worker 线程 ---
    let mut worker_thread = WorkerThread::spawn();

    // --- 创建 UI ---
    let mut ui = Ui::new();

    // --- 终端初始化 ---
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    io_thread.handle_response(&mut ui, &mut worker_thread);

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
