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

use std::sync::{mpsc, RwLock, Arc, OnceLock};
use std::{io, thread, time::Duration, env, fs};
use ratatui::layout::Alignment;
use ratatui::text::{Line, Span};
use crate::api_client::{ChatMessage, ApiClient, Model};
use crate::config_manager::*;
use crate::SystemMessage::ReadResult;
use crate::worker::*;

enum AppMessage {
    UserQuery(String),
    AIMsg(AssistantMessage),
    SysMsg(SystemMessage)
}

enum AssistantMessage {
    ModelChunk(String),
    AssistantReply(String),
    TaskComplete,
}

enum SystemMessage {
    // 提示词：reasoning, coder
    Prompt(String, String),
    // 命令执行
    ExecCommand(String),
    ExecResult(String),
    // 读取文件
    Read(String),
    ReadResult(String),
    // 应用补丁
    Diff(String, String),
    DiffResult(String),
    // 系统日志
    SystemLog(String),
}

struct App {
    input: String,
    history_display: String,
    current_ai_response: String,
    pending_action: PendingAction,
    scroll_offset: u16,
    is_auto_scroll: bool,
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
    ConfirmExec(String),
    ConfirmDiff(String, String)
}

// 定义一个全局静态变量
static CURRENT_MODEL: OnceLock<Arc<RwLock<Model>>> = OnceLock::new();

// 辅助函数：获取这个全局模型
fn get_model() -> &'static Arc<RwLock<Model>> {
    CURRENT_MODEL.get_or_init(|| {
        Arc::new(RwLock::new(Model::Reasoning))
    })
}

// 这是一个辅助函数，用于在屏幕中央计算出一个矩形区域
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let split = |dir, percent, rect| {
        Layout::default()
            .direction(dir)
            .constraints([
                Constraint::Percentage((100 - percent) / 2),
                Constraint::Percentage(percent),
                Constraint::Percentage((100 - percent) / 2),
            ])
            .split(rect)
    };

    let popup_layout = split(Direction::Vertical, percent_y, r);
    split(Direction::Horizontal, percent_x, popup_layout[1])[1]
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

    // --- 创建应用变量 ---
    let mut app = App {
        input: String::new(),
        history_display: String::new(),
        current_ai_response: String::new(),
        pending_action: PendingAction::None,
        scroll_offset: 0,
        is_auto_scroll: true,
    };

    /*
     * -------- [ 创建 IO 线程 ] --------
     */
    thread::spawn(move || {
        let mut reasoning_history = Vec::<ChatMessage>::new();
        let mut coder_history = Vec::<ChatMessage>::new();
        let mut instruct_history = Vec::<ChatMessage>::new();

        while let Ok(msg) = rx_from_ui.recv() {
            let model = get_model().read().unwrap().clone();
            let history = match model {
                Model::Reasoning => &mut reasoning_history,
                Model::Coder => &mut coder_history,
                Model::Instruct => &mut instruct_history,
            };

            let mut handle_system_result = | result: String | {
                let chat_msg = ChatMessage { role: "system".into(), content: result };
                history.push(chat_msg.clone());
                client.send_chat_stream(history.clone(), tx_to_ui.clone());
            };

            match msg {
                AppMessage::SysMsg(SystemMessage::Prompt(reasoning_prompt, coder_prompt)) => {
                    reasoning_history.push(ChatMessage { role: "prompt".into(), content: reasoning_prompt });
                    coder_history.push(ChatMessage { role: "prompt".into(), content: coder_prompt });
                }

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
                AppMessage::SysMsg(ReadResult(result)) => {
                    handle_system_result(result);
                }
                AppMessage::SysMsg(SystemMessage::DiffResult(result)) => {
                    handle_system_result(result);
                }
                _ => {}
            }
        }
    });

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

    tx_to_io.send(AppMessage::SysMsg(SystemMessage::Prompt(reasoning_prompt, coder_prompt)))?;

    let (ui_to_worker, worker_from_ui) = mpsc::channel();
    let (worker_to_ui, ui_from_worker) = mpsc::channel();

    // --- 创建 Worker 线程 ---
    thread::spawn(move || {
        while let Ok(msg) = worker_from_ui.recv() {
            match msg {
                AppMessage::SysMsg(SystemMessage::ExecCommand(cmd)) => {
                    let result = exec_cmd(&*cmd);
                    let _ = worker_to_ui.send(AppMessage::SysMsg(SystemMessage::ExecResult(result)));
                }
                AppMessage::SysMsg(SystemMessage::Read(filename)) => {
                    let content = read_file(filename.as_str());
                    let _ = worker_to_ui.send(AppMessage::SysMsg(ReadResult(content)));
                }
                AppMessage::SysMsg(SystemMessage::Diff(file_path, diff)) => {
                    let result = match apply_patch(file_path.as_str(), diff.as_str()) {
                        Ok(_) => format!("Patch 成功应用至 <{}>", file_path),
                        Err(e) => e
                    };

                    let _ = worker_to_ui.send(AppMessage::SysMsg(SystemMessage::DiffResult(result)));
                }
                _ => {}
            }
        }
    });

    // --- UI 渲染循环 ---
    loop {
        // --- UI 渲染循环 ---
        terminal.draw(|f| {
            // 对话区(自动拉伸) | 输入框(固定高度)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // --- 构建带样式的对话流 ---
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
                        "\nASSISTANT:",
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

            /*
             * -------- [ TUI 渲染 ] --------
             */
            // --- 1. 渲染对话框 ---
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
            let area = centered_rect(60, 20, f.area());
            let block = Block::default()
                .title(" 确认执行？ ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD));
            match &app.pending_action {
                PendingAction::ConfirmExec(cmd) => {
                    f.render_widget(ratatui::widgets::Clear, area);
                    let text = Paragraph::new(format!("\n待执行:\n{}\n\n按 [Y] 确认 / [N] 取消", cmd))
                        .block(block)
                        .alignment(Alignment::Center)
                        .wrap(Wrap { trim: true });
                    f.render_widget(text, area);
                }

                PendingAction::ConfirmDiff(file_path, diff) => {
                    f.render_widget(ratatui::widgets::Clear, area);
                    let text = Paragraph::new(format!("\n待应用补丁至 <{}>:\n{}\n\n按 [Y] 确认 / [N] 取消",file_path, diff))
                        .block(block)
                        .alignment(Alignment::Center)
                        .wrap(Wrap { trim: true });
                    f.render_widget(text, area);
                }
                _ => {}
            }
        })?;

        /*
         * -------- [ 异步消息处理 ] --------
         * 从 IO 线程获取 Assistant 的回复，并进行处理
         */
        if let Ok(msg) = rx_from_io.try_recv() {
            match msg {
                AppMessage::AIMsg(AssistantMessage::ModelChunk(chunk)) => {
                    app.current_ai_response.push_str(&chunk);

                    app.auto_scroll(terminal.size()?.height);
                }

                AppMessage::AIMsg(AssistantMessage::TaskComplete) => {
                    let full_msg = std::mem::take(&mut app.current_ai_response);
                    // 刷新屏幕显示
                    app.history_display.push_str(&format!("\nASSISTANT:\n{}\n", full_msg));
                    // 更新 AGENT 输出上下文
                    tx_to_io.send(AppMessage::AIMsg(AssistantMessage::AssistantReply(full_msg.clone())))?;

                    /*
                     * --------[ 这里触发解析工具调用 ] --------
                     */
                    if let Some(call) = parse_tool_call(full_msg) {
                        match call.tool {
                            Tool::Exec =>
                                app.pending_action = PendingAction::ConfirmExec(call.content),
                            Tool::Read =>
                                ui_to_worker.send(AppMessage::SysMsg(SystemMessage::Read(call.content)))?,
                            Tool::Diff(file_path) =>
                                app.pending_action = PendingAction::ConfirmDiff(file_path, call.content),
                            _ => { }
                        }
                    }
                }

                AppMessage::SysMsg(SystemMessage::SystemLog(log)) =>
                    app.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }

        /*
         * -------- [ 工具调用结果处理 ] --------
         * 从 Worker 线程接收 **工具调用结果**，并做下一步处理
         */
        if let Ok(msg) = ui_from_worker.try_recv() {
            match msg {
                AppMessage::SysMsg(SystemMessage::ExecResult(result)) => {
                    let result_feedback = format!(
                        "--- [ exec_result ] ---\n{}-----------------------", result
                    );
                    tx_to_io.send(AppMessage::SysMsg(SystemMessage::ExecResult(result_feedback)))?;
                }

                AppMessage::SysMsg(ReadResult(result)) =>
                    tx_to_io.send(AppMessage::SysMsg(ReadResult(result)))?,

                AppMessage::SysMsg(SystemMessage::DiffResult(result)) =>
                    tx_to_io.send(AppMessage::SysMsg(SystemMessage::DiffResult(result)))?,

                AppMessage::SysMsg(SystemMessage::SystemLog(log)) =>
                    app.history_display.push_str(&format!("\n[ERROR]: {}\n", log)),
                _ => {}
            }
        }

        /*
         * -------- [ 键盘事件监听 ] --------
         * - 负责修改处理输入
         * - 工具调用在经过 PendingAction 时，进行确认，并向 Worker 线程发送调用内容
         */
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
                        match &app.pending_action {
                            PendingAction::None => app.input.push(c),
                            PendingAction::ConfirmExec(exec) => {
                                if c == 'y' || c == 'Y' {
                                    ui_to_worker.send(AppMessage::SysMsg(SystemMessage::ExecCommand(exec.to_string())))?;
                                    app.pending_action = PendingAction::None;
                                } else if c == 'n' || c == 'N' {
                                    app.pending_action = PendingAction::None;
                                }
                            }
                            PendingAction::ConfirmDiff(file_path, diff) => {
                                if c == 'y' || c == 'Y' {
                                    ui_to_worker.send(AppMessage::SysMsg(SystemMessage::Diff(file_path.to_string(), diff.to_string())))?;
                                    app.pending_action = PendingAction::None;
                                } else if c == 'n' || c == 'N' {
                                    app.pending_action = PendingAction::None;
                                }

                            }
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