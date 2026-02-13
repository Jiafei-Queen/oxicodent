use std::process::Command;

pub enum Tool {
    Exec,
    Remem(usize)
}

pub struct Call {
    pub tool: Tool,
    pub content: String,
}

pub fn parse_tool_call(msg: String) -> Option<Call> {
    let mut tool_call: Option<Tool> = None;
    let mut content = String::new();
    let mut in_block = false;

    for line in msg.lines() {
        let line = line.trim();
        if line.starts_with("```exec") {
            tool_call = Some(Tool::Exec);
            in_block = true;
            continue;
        } else if line == "```" && in_block {
            break;
        }

        if in_block {
            content.push_str(&format!("{}\n", line));
        }
    }

    match tool_call {
        None => { None },
        Some(tool) => {
            Some(Call { tool, content})
        }
    }

}

pub fn exec_cmd(cmd: String) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd.as_str())
        .output()
        .expect("failed to execute process");

    let status = &output.status;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    format!("status: {}\nstdout: {}\nstderr: {}\n", status, stdout, stderr)
}