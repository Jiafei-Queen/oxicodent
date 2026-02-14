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
use ratatui::text::{Line, Span};
use crate::api_client::{ChatMessage, ApiClient};
use crate::config_manager::*;
use crate::worker::*;

enum AppMessage {
    Prompt(String, Option<Vec<ChatMessage>>),
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
    history_display: String,
    current_ai_response: String,
    pending_action: PendingAction,
    scroll_offset: u16,
    is_auto_scroll: bool
}

impl App {
    fn auto_scroll(&mut self, terminal_height: u16) {
        // 粗略估算对话框高度（总高度 - 输入框3行 - 边框2行）
        let chat_height = terminal_height.saturating_sub(5);

        // 计算当前显示的所有行数（包括 Logo 和 历史记录）
        let logo_lines = 12;
        let history_lines = self.history_display.lines().count() as u16;
        let current_ai_lines = self.current_ai_response.lines().count() as u16;

        let total_lines = logo_lines + history_lines + current_ai_lines;

        if total_lines > chat_height {
            self.scroll_offset = total_lines - chat_height;
        } else {
            self.scroll_offset = 0;
        }
    }
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
                AppMessage::Prompt(content, old_his) => {
                    history.push(ChatMessage { role: "system".into(), content });
                    if let Some(his) = old_his {
                        history.extend(his);
                    }
                    client.send_chat_stream(history.clone() , tx_to_ui.clone());
                }

                AppMessage::UserQuery(content) => {
                    let chat_msg = ChatMessage { role: "user".into(), content };
                    history.push(chat_msg.clone());
                    History::update_history(chat_msg.clone());
                    client.send_chat_stream(history.clone(), tx_to_ui.clone());
                }
                AppMessage::AssistantReply(content) => {
                    let chat_msg = ChatMessage { role: "assistant".into(), content };
                    history.push(chat_msg.clone());
                    History::update_history(chat_msg);
                }
                AppMessage::ExecResult(result) => {
                    let feedback = format!("Command output:\n{}", result);
                    history.push(ChatMessage { role: "system".into(), content: feedback });
                }
                _ => {}
            }
        }
    });

    let mut app = App {
        input: String::new(),
        history_display: String::new(),
        current_ai_response: String::new(),
        pending_action: PendingAction::None,
        scroll_offset: 0,
        is_auto_scroll: true
    };

    // 获得当前目录下的条目
    let mut entries = Vec::new();
    for entry in fs::read_dir(".")? {
        let path = entry?.path();
        entries.push(path.file_name().unwrap().to_string_lossy().to_string());
    }

    let mut has_history = false;
    let mut history: Vec<ChatMessage> = Vec::new();
    if let Ok(old) = History::load_history() {
        has_history = true;
        app.history_display.push_str("聊天记录 `history.txt` 已加载");
        let _ = old.iter().map(
            |h| history.push(
                ChatMessage { role: h.role.clone(), content: h.content.clone() }
            )
        );
    }

    // DEBUG
    if let Err(err) = History::load_history() {
        app.history_display.push_str(err.to_string().as_str())
    }

    // 拼接提示词
    let prompt = format!("{}\n\nCWD: {}\n--- [ DIRS ] ---\n{}----------------",
        read_or_create_prompt(),
        env::current_dir()?.to_string_lossy().to_string(),
        entries.join("\n")
    );

    // 注入提示词
    if has_history {
        tx_to_io.send(AppMessage::Prompt(prompt, Some(history)))?;
    } else {
        tx_to_io.send(AppMessage::Prompt(prompt, None))?;
    }

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
        // --- UI 渲染循环 ---
        terminal.draw(|f| {
            // 重新划分：对话区(自动拉伸) | 输入框(固定高度)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),   // 对话区 chunks[0]
                    Constraint::Length(3), // 输入框 chunks[1]
                ])
                .split(f.size());

            // --- 1. 构建带样式的对话流 ---
            let mut lines = Vec::new();

            // A. 渲染红色居中的 Logo
            let logo_str = get_logo_text();
            for line in logo_str.lines() {
                lines.push(
                    Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::Red),
                    ))
                        .alignment(Alignment::Center),
                );
            }
            lines.push(Line::from("")); // 留白行

            // B. 渲染历史记录（左右对齐）
            for hist_line in app.history_display.lines() {
                if hist_line.starts_with("USER:") {
                    // 用户的话：右对齐，绿色
                    lines.push(
                        Line::from(Span::styled(
                            hist_line,
                            Style::default().fg(Color::Green),
                        ))
                            .alignment(Alignment::Right),
                    );
                } else if hist_line.starts_with("ASSISTANT:") {
                    // 模型的话：左对齐，青色
                    lines.push(
                        Line::from(Span::styled(
                            hist_line,
                            Style::default().fg(Color::Cyan),
                        ))
                            .alignment(Alignment::Left),
                    );
                } else {
                    // 这里的 line 可能是执行结果或者换行，默认左对齐
                    lines.push(Line::from(hist_line).alignment(Alignment::Left));
                }
            }

            // C. 渲染正在生成的 AI 回复
            if !app.current_ai_response.is_empty() {
                // 先添加 ASSISTANT: 标签行
                lines.push(
                    Line::from(Span::styled(
                        "ASSISTANT:",
                        Style::default().fg(Color::Cyan),
                    ))
                        .alignment(Alignment::Left),
                );
                // 将响应按行分割，每行都应用 Cyan 样式
                for line in app.current_ai_response.lines() {
                    lines.push(
                        Line::from(Span::styled(
                            line,
                            Style::default().fg(Color::Cyan),
                        ))
                            .alignment(Alignment::Left),
                    );
                }
            }

            // 渲染对话 Paragraph
            let chat_block = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Oxicodent Chat "))
                .wrap(Wrap { trim: false })
                .scroll((app.scroll_offset, 0));
            f.render_widget(chat_block, chunks[0]);

            // --- 2. 渲染输入框 ---
            let input_block = Paragraph::new(app.input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" 输入 (回车发送, ESC退出) "));
            f.render_widget(input_block, chunks[1]);

            // --- 3. 渲染弹窗 (覆盖在最上方) ---
            if let PendingAction::ConfirmExec(cmd) = &app.pending_action {
                let area = centered_rect(60, 20, f.size());
                // 必须先 Clear，否则弹窗后面会透出聊天记录
                f.render_widget(ratatui::widgets::Clear, area);

                let block = Block::default()
                    .title(" 确认执行命令？ ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD));

                let text = Paragraph::new(format!("\n待执行命令:\n> {}\n\n按 [Y] 确认 / [N] 取消", cmd))
                    .block(block)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });

                f.render_widget(text, area);
            }
        })?;

        // --- 异步消息处理 ---
        if let Ok(msg) = rx_from_io.try_recv() {
            match msg {
                AppMessage::ModelChunk(chunk) => {
                    app.current_ai_response.push_str(&chunk);

                    app.auto_scroll(terminal.size()?.height);
                }

                AppMessage::TaskComplete => {
                    let full_msg = std::mem::take(&mut app.current_ai_response);
                    // 刷新屏幕显示
                    app.history_display.push_str(&format!("\nASSISTANT:\n{}\n", full_msg));
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
                    tx_to_io.send(AppMessage::ExecResult(result_feedback))?;
                }
                AppMessage::SystemLog(log) => app.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }

        // --- 事件监听 ---
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(5);
                        app.is_auto_scroll = false;
                    }
                    KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.scroll_offset = app.scroll_offset.saturating_add(5);
                        app.is_auto_scroll = false;
                    }

                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if let PendingAction::None = &app.pending_action {
                            let query = std::mem::take(&mut app.input);
                            app.history_display.push_str(&format!("\nUSER: {}\n", query));
                            tx_to_io.send(AppMessage::UserQuery(query))?;
                        }
                    }
                    KeyCode::Char(c) => {
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