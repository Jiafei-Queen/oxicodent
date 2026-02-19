use crate::{PendingAction, AppTerminal, get_logo_text};

use ratatui::{text::{Line, Span}, layout::{Constraint, Direction, Layout, Alignment}, widgets::{Block, Borders, Paragraph, Wrap}, style::{Style, Color}, Terminal};
use ratatui::backend::CrosstermBackend;

pub struct Ui {
    terminal: AppTerminal,
    pub input: String,
    pub history_display: String,
    pub current_ai_response: String,
    pub pending_action: PendingAction,
    pub scroll_offset: u16,
    pub is_auto_scroll: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            terminal: Terminal::new(CrosstermBackend::new(std::io::stdout())).unwrap(),
            input: String::new(),
            history_display: String::new(),
            current_ai_response: String::new(),
            pending_action: PendingAction::None,
            scroll_offset: 0,
            is_auto_scroll: true,
        }
    }

    pub fn auto_scroll(&mut self) {
        let terminal_height = self.terminal.size().unwrap().height;
        // 粗略估算对话框高度（总高度 - 输入框3行 - 边框2行）
        let chat_height = terminal_height.saturating_sub(5);

        // 计算当前显示的所有行数（包括 Logo 和 历史记录）
        let logo_lines = 14;
        let history_lines = self.history_display.lines().count() as u16;
        let current_ai_lines = self.current_ai_response.lines().count() as u16;

        let total_lines = logo_lines + history_lines + current_ai_lines;

        if total_lines > chat_height {
            self.scroll_offset = total_lines - chat_height;
        } else {
            self.scroll_offset = 0;
        }
    }


    pub fn render(&mut self) {
        // --- UI 渲染循环 ---
        self.terminal.draw(|f| {
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
            for hist_line in self.history_display.lines() {
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
            if !self.current_ai_response.is_empty() {
                // 先添加 ASSISTANT: 标签行
                lines.push(
                    Line::from(Span::styled(
                        "\nASSISTANT:",
                        Style::default().fg(Color::Cyan),
                    ))
                        .alignment(Alignment::Left),
                );
                // 将响应按行分割，每行都应用 Cyan 样式
                for line in self.current_ai_response.lines() {
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
                .scroll((self.scroll_offset, 0));
            f.render_widget(chat_block, chunks[0]);

            // --- 2. 渲染输入框 ---
            let input_block = Paragraph::new(self.input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" 输入 (回车发送, ESC退出) "));
            f.render_widget(input_block, chunks[1]);

            // --- 3. 渲染弹窗 (覆盖在最上方) ---
            let area = centered_rect(60, 20, f.area());
            let block = Block::default()
                .title(" 确认执行？ ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD));
            match &self.pending_action {
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
        }).unwrap();
    }
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
