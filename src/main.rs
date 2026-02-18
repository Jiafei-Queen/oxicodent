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

use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::io;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use tracing::info;
use crate::config_manager::*;
use crate::app::*;
use crate::event_handler::handle_event;
use crate::io_thread::IOThread;
use crate::ui::Ui;
use crate::worker_thread::WorkerThread;

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- [ 初始化日志 ] ---
    let log_file = std::fs::File::create(".oxicodent.log")?;
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug"))  // ← 默认 debug 级别
        )
        .with(fmt::layer().with_writer(log_file))
        .init();

    info!(":: Oxicodent ::    (v{})", env!("CARGO_PKG_VERSION"));

    // --- 创建 IO 线程 ---
    let mut io_thread = IOThread::spawn()?;
    info!("IO 线程已创建");

    // --- 创建 Worker 线程 ---
    let mut worker_thread = WorkerThread::spawn();
    info!("Worker 线程已创建");

    // --- 创建 UI ---
    let mut ui = Ui::new();
    info!("UI 已创建");

    // --- 终端初始化 ---
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    info!("终端已初始化");

    info!("进入主循环");
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
    info!("恢复终端");
    Ok(())
}
