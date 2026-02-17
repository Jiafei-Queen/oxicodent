use std::time::Duration;
use crate::ui::Ui;
use crate::io_thread::IOThread;
use crate::app::{AppMessage, PendingAction, SystemMessage};
use crate::worker_thread::WorkerThread;
use crossterm::{
    event::{self, Event, KeyCode},
};

/*
 * -------- [ 键盘事件监听 ] --------
 * - 负责修改处理输入
 * - 工具调用在经过 PendingAction 时，进行确认，并向 Worker 线程发送调用内容
 * - 返回是否 退出
 */
pub fn handle_event(ui: &mut Ui, io_thread: &mut IOThread, worker_thread: &mut WorkerThread) -> Result<bool, Box<dyn std::error::Error>> {
    if event::poll(Duration::from_millis(10))? {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('u') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    ui.scroll_offset = ui.scroll_offset.saturating_sub(5);
                    ui.is_auto_scroll = false;
                }
                KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    ui.scroll_offset = ui.scroll_offset.saturating_add(5);
                    ui.is_auto_scroll = false;
                }

                KeyCode::Esc => return Ok(true),
                KeyCode::Enter => {
                    if let PendingAction::None = &ui.pending_action {
                        let query = std::mem::take(&mut ui.input);
                        ui.history_display.push_str(&format!("\nUSER: {}\n", query));
                        io_thread.send(AppMessage::UserQuery(query));
                    }
                }
                KeyCode::Char(c) => {
                    match &ui.pending_action {
                        PendingAction::None => ui.input.push(c),
                        PendingAction::ConfirmExec(exec) => {
                            if c == 'y' || c == 'Y' {
                                worker_thread.send(AppMessage::SysMsg(SystemMessage::ExecCommand(exec.to_string())));
                                ui.pending_action = PendingAction::None;
                            } else if c == 'n' || c == 'N' {
                                ui.pending_action = PendingAction::None;
                            }
                        }
                        PendingAction::ConfirmDiff(file_path, diff) => {
                            if c == 'y' || c == 'Y' {
                                worker_thread.send(AppMessage::SysMsg(SystemMessage::Diff(file_path.to_string(), diff.to_string())));
                                ui.pending_action = PendingAction::None;
                            } else if c == 'n' || c == 'N' {
                                ui.pending_action = PendingAction::None;
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let PendingAction::None = &ui.pending_action {
                        ui.input.pop();
                    }
                }
                _ => {}
            }
        }
    }

    Ok(false)
}