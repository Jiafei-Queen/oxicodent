use std::fs;
use std::process::Command;
use diffy::{apply, Patch};

#[allow(dead_code)]
pub enum Tool {
    Exec,
    Read,
    Diff(String),
    Remem(usize),
    Search(String)
}

pub struct Call {
    pub tool: Tool,
    pub content: String,
}

/// 验证文件路径是否在当前工作目录下，防止路径穿越攻击
/// 返回解析后的安全路径
fn resolve_safe_path(file_path: &str) -> Result<std::path::PathBuf, String> {
    let base_dir = std::env::current_dir()
        .map_err(|e| format!("无法获取当前工作目录: {}", e))?;

    let full_path = base_dir.join(file_path);

    use std::path::Component;
    let mut normalized_path = std::path::PathBuf::new();
    for component in full_path.components() {
        match component {
            Component::ParentDir => { normalized_path.pop(); }
            Component::CurDir => {}
            c => normalized_path.push(c),
        }
    }

    if !normalized_path.starts_with(&base_dir) {
        return Err(format!("越权访问: {}", file_path));
    }

    Ok(normalized_path)
}

pub fn parse_tool_call(msg: String) -> Option<Call> {
    let mut tool: Option<Tool> = None;
    let mut content = String::new();
    let mut in_block = false;

    for line in msg.lines() {
        if line.starts_with("```exec") {
            tool = Some(Tool::Exec);
            in_block = true;
            continue;
        } else if line.starts_with("```read") {
            let filename = line.strip_prefix("```read:")
                .unwrap_or("").trim().to_string();
            return Some(Call { tool: Tool::Read, content: filename })
        } else if line.starts_with("```diff") {
            // 安全地提取文件名，移除 ```diff 前缀
            let mut filename = line.strip_prefix("```diff:")
                .unwrap_or("").trim().to_string();

            // 移除可能的前后引号
            if filename.starts_with('"') && filename.ends_with('"') {
                filename = filename[1..filename.len()-1].to_string();
            } else if filename.starts_with('\'') && filename.ends_with('\'') {
                filename = filename[1..filename.len()-1].to_string();
            }

            tool = Some(Tool::Diff(filename));
            in_block = true;
            continue
        } else if line == "```" && in_block {
            return Some(Call { tool: tool.unwrap(), content })
        }

        if in_block {
            content.push_str(&format!("{}\n", line));
        }
    }

    None
}

/// 安全地执行命令，避免 shell 注入
/// 将命令字符串解析为程序名和参数，直接执行而不通过 shell
pub fn exec_cmd(cmd: &str) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output();

    match output {
        Ok(result) => {
            let status = &result.status;
            let stdout = String::from_utf8_lossy(&result.stdout).to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            format!("status: {}\nstdout: {}\nstderr: {}\n", status, stdout, stderr)
        }
        Err(e) => {
            format!("命令执行失败: {}", e)
        }
    }
}

pub fn read_file(filename: &str) -> String {
    let mut output = String::new();
    let full_content = fs::read_to_string(filename).unwrap();

    let mut count = 0;
    for line in full_content.lines() {
        count += 1;
        output.push_str(format!("{}) {}\n", count, line).as_str());
    }

    output
}

pub fn apply_patch(file_path: &str, diff: &str) -> Result<(), String> {
    // 安全校验：防止路径穿越攻击
    let safe_path = resolve_safe_path(file_path)?;

    // 使用解析后的安全路径进行所有操作
    // 1. 读取文件
    let original = fs::read_to_string(&safe_path)
        .map_err(|e| format!("无法读取目标文件 <{}>: {}", safe_path.display(), e))?;

    // 2. 解析 Patch
    let patch = Patch::from_str(diff)
        .map_err(|e| format!("无法解析Patch: {}", e))?;

    // 3. 应用 Patch
    let applied = apply(&original, &patch)
        .map_err(|e| format!("无法应用Patch: {}", e))?;

    if applied == original {
        return Err(format!(
            "Patch 应用失败：文件 <{}> 内容未发生任何变化。请检查你的 Diff 上下文是否与当前文件内容匹配。",
            file_path
        ));
    }

    // 4. 写入文件
    fs::write(&safe_path, applied)
        .map_err(|e| format!("无法写入文件 <{}>: {}", safe_path.display(), e))?;

    Ok(())
}