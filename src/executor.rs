use std::process::Command;

pub fn parse_and_exec_cmd(msg: String) -> Option<String>{
    // 解析 命令执行 代码块
    let mut is_exec = false;
    let mut cmd: Option<String> = None;
    for line in msg.lines() {
        if line.replace(" ", "") == "```exec" { is_exec = true; continue; }
        if line.replace(" ", "") == "```" { break; }
        if is_exec {
            match cmd {
                None => { cmd = Some(line.to_string()) },
                Some(s) => {
                    let new_cmd = format!("{}\n{}",s, line);
                    cmd = Some(new_cmd);
                }
            }
        }
    }

    if let Some(output) = cmd {
        println!("\n\n[DEBUG]: 成功截获命令: {}", output);
        let output = Command::new("sh").arg("-c").arg(output).output();
        if let Ok(out) = output {
            let result = String::from_utf8_lossy(&out.stdout).to_string();
            println!("[DEBUG]: 执行结果: {}", &result);
            return Some(result);
        }
    }

    None
}