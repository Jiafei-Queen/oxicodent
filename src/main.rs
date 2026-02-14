mod config_manager;
mod api_client;
mod worker;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal, style::{Style, Color},
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use std::{io, sync::mpsc, thread, time::Duration, env, fs};
use ratatui::layout::Alignment;
use crate::api_client::{ChatMessage, ApiClient};
use crate::config_manager::*;
use crate::worker::*;

enum AppMessage {
    UserQuery(String),
    ModelChunk(String),
    AssistantReply(String),
    ExecCommand(String),
    ExecResult(String),
    SystemLog(String),
    TaskComplete,
}

struct App {
    input: String,
    history_display: String, // 将历史拼成一个大字符串，方便 Paragraph 渲染
    current_ai_response: String,
    pending_action: PendingAction,
}

enum PendingAction {
    None,
    ConfirmExec(String), // 存储待执行的命令
}

// 这是一个辅助函数，用于在屏幕中央计算出一个矩形区域
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 终端初始化 ---
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // --- 通信与后台线程 ---
    let (tx_to_io, rx_from_ui) = mpsc::channel();
    let (tx_to_ui, rx_from_io) = mpsc::channel();
    let client = ApiClient::new(&Config::load_or_init());

    thread::spawn(move || {
        let mut history: Vec<ChatMessage> = Vec::new();
        while let Ok(msg) = rx_from_ui.recv() {
            match msg {
                AppMessage::UserQuery(q) => {
                    history.push(ChatMessage { role: "user".into(), content: q });
                    client.send_chat_stream(history.clone(), tx_to_ui.clone());
                }
                AppMessage::AssistantReply(content) => {
                    history.push(ChatMessage { role: "assistant".into(), content });
                }
                AppMessage::ExecResult(result) => {
                    let feedback = format!("Command output:\n{}", result);
                    history.push(ChatMessage { role: "user".into(), content: feedback });
                    // 这里可以选择是否立即触发 AI 下一步，或者等待用户
                }
                _ => {}
            }
        }
    });

    let mut app = App {
        input: String::new(),
        history_display: String::new(),
        current_ai_response: String::new(),
        pending_action: PendingAction::None
    };

    // 获得当前目录下的条目
    let mut entries = Vec::new();
    for entry in fs::read_dir(".")? {
        let path = entry?.path();
        entries.push(path.file_name().unwrap().to_string_lossy().to_string());
    }

    // 拼接提示词
    let prompt = format!("{}\n\nCWD: {}\n--- [ DIRS ] ---\n{}----------------",
        read_or_create_prompt(),
        env::current_dir()?.to_string_lossy().to_string(),
        entries.join("\n")
    );

    // 注入提示词
    tx_to_io.send(AppMessage::UserQuery(prompt))?;

    let (ui_to_worker, worker_from_ui) = mpsc::channel();
    let (worker_to_ui, ui_from_worker) = mpsc::channel();

    // 创建 Worker 线程
    thread::spawn(move || {
        while let Ok(msg) = worker_from_ui.recv() {
            if let AppMessage::ExecCommand(cmd) = msg {
                let result = exec_cmd(&*cmd);
                worker_to_ui.send(AppMessage::ExecResult(result)).unwrap();
            }
        }
    });

    // --- UI 渲染循环 ---
    loop {
        // main.rs 渲染部分重构
        terminal.draw(|f| {
            // 重新划分：Logo(固定高度) | 对话区(自动拉伸) | 输入框(固定高度)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8), // Logo 预留 8 行高度
                    Constraint::Min(10),   // 对话区至少保留 10 行
                    Constraint::Length(3), // 输入框 3 行
                ])
                .split(f.size());

            // --- 1. 渲染 Logo (独立区域，不会再干扰对话) ---
            let logo_text = get_logo_text();
            let logo = Paragraph::new(logo_text)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center); // 居中
            f.render_widget(logo, chunks[0]);

            // --- 2. 渲染对话区 (使用 Paragraph 替代 List 以修复 wrap 报错) ---
            let mut display_text = app.history_display.clone();
            if !app.current_ai_response.is_empty() {
                display_text.push_str(&format!("\nAGENT: {}", app.current_ai_response));
            }

            let chat_block = Paragraph::new(display_text)
                .block(Block::default().borders(Borders::ALL).title(" Oxicodent Chat "))
                .wrap(Wrap { trim: true });
            f.render_widget(chat_block, chunks[1]);

            // --- 3. 渲染输入框 ---
            let input_block = Paragraph::new(app.input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" 输入 (回车发送, ESC退出) "));
            f.render_widget(input_block, chunks[2]);

            // 渲染弹窗
            if let PendingAction::ConfirmExec(cmd) = &app.pending_action {
                let area = centered_rect(60, 20, f.size());
                f.render_widget(ratatui::widgets::Clear, area);
                let block = Block::default().title(" 确认执行命令？ ").borders(Borders::ALL).border_style(Style::default().fg(Color::Red));
                let text = Paragraph::new(format!("命令: {}\n\n按 [Y] 确认 / [N] 取消", cmd))
                    .block(block)
                    .alignment(Alignment::Center);
                f.render_widget(text, area);
            }
        })?;

        // --- 异步消息处理 ---
        if let Ok(msg) = rx_from_io.try_recv() {
            match msg {
                AppMessage::ModelChunk(chunk) => app.current_ai_response.push_str(&chunk),
                AppMessage::TaskComplete => {
                    let full_msg = std::mem::take(&mut app.current_ai_response);
                    // 刷新屏幕显示
                    app.history_display.push_str(&format!("\nAGENT: {}\n", full_msg));
                    // 更新 AGENT 输出上下文
                    tx_to_io.send(AppMessage::AssistantReply(full_msg.clone()))?;

                    // 这里触发解析工具调用
                    if let Some(call) = parse_tool_call(full_msg) {
                        match call.tool {
                            Tool::Exec => {
                                app.pending_action = PendingAction::ConfirmExec(call.content);
                            }
                            _ => { }
                        }
                    }
                }
                AppMessage::SystemLog(log) => app.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }

        if let Ok(msg) = ui_from_worker.try_recv() {
            match msg {
                AppMessage::ExecResult(result) => {
                    let result_feedback = format!(
                        "--- [ exec_result ] ---\n{}-----------------------", result
                    );
                    tx_to_io.send(AppMessage::UserQuery(result_feedback))?;
                }
                AppMessage::SystemLog(log) => app.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }

        // --- 事件监听 ---
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if let PendingAction::None = &app.pending_action {
                            let query = std::mem::take(&mut app.input);
                            app.history_display.push_str(&format!("\nUSER: {}\n", query));
                            tx_to_io.send(AppMessage::UserQuery(query))?;
                        }
                    }
                    KeyCode::Char(c) => {
                        if (c == 'c' || c == 'd') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                            return Ok(())
                        }

                        if let PendingAction::ConfirmExec(exec) = &app.pending_action {
                            if c == 'y' || c == 'Y' {
                                ui_to_worker.send(AppMessage::ExecCommand(exec.to_string()))?;
                                app.pending_action = PendingAction::None;
                            } else if c == 'n' || c == 'N' {
                                app.pending_action = PendingAction::None;
                            }
                        } else {
                            app.input.push(c)
                        }
                    }
                    KeyCode::Backspace => {
                        if let PendingAction::None = &app.pending_action {
                            app.input.pop();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // --- 恢复终端 ---
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn get_logo_text() -> String {
    let logo =
r#"  .oooooo.                o8o                            .o8                            .
 d8P'  `Y8b               `"'                           "888                          .o8
   888      888 oooo    ooo oooo   .ooooo.   .ooooo.   .oooo888   .ooooo.  ooo. .oo.   .o888oo
888      888  `88b..8P'  `888  d88' `"Y8 d88' `88b d88' `888  d88' `88b `888P"Y88b    888
888      888    Y888'     888  888       888   888 888   888  888ooo888  888   888    888
  `88b    d88'  .o8"'88b    888  888   .o8 888   888 888   888  888    .o  888   888    888 .
         `Y8bood8P'  o88'   888o o888o `Y8bod8P' `Y8bod8P' `Y8bod88P" `Y8bod8P' o888o o888o   "888"     "#;
    format!("{}\n:: Oxicodent — A Light Coding Agent ::\t(v{})", logo, env!("CARGO_PKG_VERSION"))
}