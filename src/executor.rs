use std::process::Command;

fn exec_cmd(cmd: String) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", cmd.as_str()])
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(cmd.as_str())
            .output()
            .expect("failed to execute process")
    };

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    result
}

pub enum Tool {
    Exec,
    Write(String),
    Read([usize;2]),
    Diff(String),
    Remem(usize)
}

pub struct Call {
    pub tool: Tool,
    pub result: String,
}

pub fn parse_tool_call(msg: String) -> Option<Call> {
    let mut tool_call: Option<Tool> = None;
    let mut content = String::new();
    for line in msg.lines() {
        match line.trim() {
            "```exec" => { tool_call = Some(Tool::Exec); continue; }
            "```" => { break; }
            _ => {}
        }

        if let Some(_) = tool_call {
            content.push_str(format!("{}\n", line).as_str())
        }
    }

    match tool_call {
        None => { None }
        Some(tool) => {
            match tool {
                Tool::Exec => {
                    let result = exec_cmd(content);
                    Some(Call { tool, result })
                }
                _ => { None }
            }
        }
    }
}