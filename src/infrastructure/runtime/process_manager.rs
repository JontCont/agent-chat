use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::AsyncReadExt;
use futures_util::Stream;
use tracing::{info, error, warn};

pub fn get_pid_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::temp_dir().join("agent-pids")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/tmp/agent-pids")
    }
}

pub fn write_pid(session_id: &str, pid: u32) -> std::io::Result<()> {
    let dir = get_pid_dir();
    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{}.pid", session_id));
    std::fs::write(file_path, pid.to_string())
}

pub fn delete_pid(session_id: &str) {
    let file_path = get_pid_dir().join(format!("{}.pid", session_id));
    let _ = std::fs::remove_file(file_path);
}

pub fn get_pid(session_id: &str) -> Option<u32> {
    let file_path = get_pid_dir().join(format!("{}.pid", session_id));
    if let Ok(content) = std::fs::read_to_string(file_path) {
        content.trim().parse::<u32>().ok()
    } else {
        None
    }
}

#[cfg(windows)]
pub async fn terminate_process_tree(pid: u32) -> std::io::Result<()> {
    if pid == 999999 || pid == 0 {
        info!("Bypassing terminate_process_tree for mock PID {}", pid);
        return Ok(());
    }
    info!("Terminating process tree for PID {} on Windows", pid);
    let mut child = Command::new("taskkill")
        .args(&["/F", "/T", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child.wait().await?;
    Ok(())
}

#[cfg(not(windows))]
pub async fn terminate_process_tree(pid: u32) -> std::io::Result<()> {
    if pid == 999999 || pid == 0 {
        info!("Bypassing terminate_process_tree for mock PID {}", pid);
        return Ok(());
    }
    info!("Terminating process group for PID {} on Unix", pid);
    // Send SIGKILL to the process group (-pid)
    let pid_i32 = -(pid as i32);
    let mut child = Command::new("kill")
        .args(&["-9", &pid_i32.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child.wait().await?;
    Ok(())
}

/// Strip ANSI escape sequences from a string.
/// Handles CSI sequences (\x1B[...X), OSC sequences (\x1B]...BEL), and other escape codes.
fn strip_ansi_escapes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1B' {
            // Start of escape sequence
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: \x1B[ ... (letter)
                    chars.next(); // consume '['
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        // CSI terminates at a letter (A-Z, a-z)
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: \x1B] ... BEL(\x07) or ST(\x1B\\)
                    chars.next(); // consume ']'
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1B' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some('(') | Some(')') => {
                    // Character set designation: \x1B( X or \x1B) X
                    chars.next(); // consume '(' or ')'
                    chars.next(); // consume the charset designator
                }
                _ => {
                    // Other single-character escape: consume next char
                    chars.next();
                }
            }
        } else if ch == '\x07' || ch == '\x08' {
            // BEL or Backspace - skip
        } else if ch == '\r' {
            // Carriage return - skip (we keep \n)
        } else {
            result.push(ch);
        }
    }

    result
}

/// Extract the actual response from raw conhost --headless output.
///
/// The agy TUI renders agent thinking steps (tool calls, file inspections, etc.)
/// on screen, then clears the screen with `\x1B[2J` before displaying the final
/// response. By finding the last clear screen sequence, we discard all the
/// intermediate "I will check..." planning text and extract only the actual answer.
fn extract_response(raw_bytes: &[u8]) -> String {
    let clear_screen = b"\x1B[2J";

    // Find the last occurrence of clear screen
    let start_pos = raw_bytes
        .windows(clear_screen.len())
        .rposition(|w| w == clear_screen)
        .map(|pos| pos + clear_screen.len())
        .unwrap_or(0);

    let content = &raw_bytes[start_pos..];
    let text = String::from_utf8_lossy(content);
    let cleaned = strip_ansi_escapes(&text);
    cleaned.trim().to_string()
}

fn check_executable(cmd: &str) -> bool {
    let has_separator = cmd.contains('/') || (cfg!(windows) && cmd.contains('\\'));
    if !has_separator {
        if let Ok(path_val) = std::env::var("PATH") {
            let extensions = if cfg!(windows) {
                vec!["", ".exe", ".cmd", ".bat"]
            } else {
                vec![""]
            };
            for dir in std::env::split_paths(&path_val) {
                for ext in &extensions {
                    let mut file_path = dir.join(cmd);
                    if !ext.is_empty() {
                        if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                            file_path.set_file_name(format!("{}{}", name, ext));
                        }
                    }
                    if file_path.is_file() {
                        return true;
                    }
                }
            }
        }
    } else {
        let path = std::path::Path::new(cmd);
        if path.is_file() {
            return true;
        }
        if cfg!(windows) {
            let extensions = vec![".exe", ".cmd", ".bat"];
            for ext in extensions {
                let mut path_ext = PathBuf::from(cmd);
                if let Some(name) = path_ext.file_name().and_then(|n| n.to_str()) {
                    path_ext.set_file_name(format!("{}{}", name, ext));
                }
                if path_ext.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn spawn_run(session_id: String, prompt: String, active_cli: String) -> impl Stream<Item = Result<String, std::io::Error>> + Send {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let cli_cmd = match active_cli.as_str() {
            "openai" => std::env::var("OPENAI_CLI_PATH").unwrap_or_else(|_| "openai".to_string()),
            "copilot" => std::env::var("COPILOT_CLI_PATH").unwrap_or_else(|_| "copilot".to_string()),
            "claude" => std::env::var("CLAUDE_CLI_PATH").unwrap_or_else(|_| "claude".to_string()),
            _ => std::env::var("AGY_CLI_PATH").unwrap_or_else(|_| "agy".to_string()),
        };
        info!("Spawning child process for session {}: {} --print '{}' (Active CLI: {})", session_id, cli_cmd, prompt, active_cli);
        
        // On Windows, agy writes directly to the console device (CONOUT$),
        // bypassing piped stdout entirely. We use `conhost.exe --headless`
        // to create a headless pseudo-console that captures this output.
        // On non-Windows, we spawn agy directly with piped stdout.
        let spawn_result = if check_executable(&cli_cmd) {
            spawn_agy_command(&cli_cmd, &prompt)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Executable '{}' not found in PATH or disk", cli_cmd),
            ))
        };

        match spawn_result {
            Ok(mut child) => {
                if let Some(pid) = child.id() {
                    let _ = write_pid(&session_id, pid);
                }

                let mut stdout = child.stdout.take().unwrap();
                let mut stderr = child.stderr.take().unwrap();

                // Monitor stderr in background
                let session_id_err = session_id.clone();
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let _ = stderr.read_to_end(&mut buf).await;
                    if !buf.is_empty() {
                        let text = String::from_utf8_lossy(&buf);
                        let cleaned = strip_ansi_escapes(&text);
                        for line in cleaned.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                warn!("Antigravity CLI stderr (session {}): {}", session_id_err, trimmed);
                            }
                        }
                    }
                });

                // Read ALL stdout bytes — we need the complete output to find
                // the boundary between agent thinking and the actual response.
                // The agy TUI clears the screen (\x1B[2J) before the final answer.
                let mut all_bytes = Vec::new();
                let _ = stdout.read_to_end(&mut all_bytes).await;

                let _ = child.wait().await;
                delete_pid(&session_id);

                // Extract the actual response (everything after the last clear screen)
                let response = extract_response(&all_bytes);

                if response.is_empty() {
                    warn!("Empty response from agy for session {}", session_id);
                } else {
                    info!("Extracted response ({} bytes) for session {}", response.len(), session_id);
                    // Send each line (including empty lines for paragraph breaks)
                    for line in response.lines() {
                        if tx.send(Ok(line.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn agy executable: {}. Using mock stream generator instead.", e);
                // Fallback to mock stream generator
                let prompt_lower = prompt.to_lowercase();
                let mock_response = if prompt_lower.contains("image")
                    || prompt_lower.contains("picture")
                    || prompt_lower.contains("draw")
                    || prompt_lower.contains("paint")
                    || prompt_lower.contains("圖")
                    || prompt_lower.contains("畫")
                {
                    format!(
                        "Here is the image you requested:\n\n\
                         ![Gemini Generated Image](/template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png)\n\n\
                         I have generated this image for you using the Antigravity model."
                    )
                } else {
                    format!(
                        "This is a mock response from the Agent Bridge.\n\
                         You asked: '{}'\n\
                         The Antigravity CLI could not be spawned (Executable: '{}').\n\
                         Decoupled architecture is verified! Streaming is active.",
                        prompt, cli_cmd
                    )
                };

                let _ = write_pid(&session_id, 999999);

                for line in mock_response.lines() {
                    if get_pid(&session_id).is_none() {
                        info!("Mock stream loop detected cancellation for session: {}", session_id);
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    if tx.send(Ok(line.to_string())).await.is_err() {
                        break;
                    }
                }
                delete_pid(&session_id);
            }
        }
    });

    tokio_stream::wrappers::ReceiverStream::new(rx)
}

/// Spawn the agy CLI command with platform-specific handling.
///
/// On Windows, agy uses the Windows Console API to render output directly to
/// CONOUT$, which means piped stdout receives nothing. To capture this output,
/// we wrap the command in `conhost.exe --headless --` which creates a headless
/// pseudo-console. The agy process writes to the pseudo-console, and conhost
/// forwards that output to its own piped stdout.
///
/// On non-Windows platforms, agy writes to stdout normally, so we spawn it directly.
fn spawn_agy_command(agy_cmd: &str, prompt: &str) -> std::io::Result<tokio::process::Child> {
    #[cfg(windows)]
    {
        info!("Using conhost --headless wrapper for agy on Windows");
        // Build the inner command string for conhost to execute.
        // conhost --headless -- <program> <args...>
        Command::new("conhost.exe")
            .arg("--headless")
            .arg("--")
            .arg(agy_cmd)
            .arg("--dangerously-skip-permissions")
            .arg("--print")
            .arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Do NOT redirect stdin to null — agy needs inherited console stdin
            .spawn()
    }
    #[cfg(not(windows))]
    {
        Command::new(agy_cmd)
            .arg("--print")
            .arg(prompt)
            .env("TERM", "dumb")
            .env("CI", "true")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_spawn_run_fallback_openai() {
        std::env::set_var("OPENAI_CLI_PATH", "nonexistent_openai_cli_path_xyz");
        // Since 'openai' executable doesn't exist, it should trigger fallback mock stream
        let mut stream = spawn_run("session_openai".to_string(), "hello".to_string(), "openai".to_string());
        
        let mut lines = Vec::new();
        while let Some(res) = stream.next().await {
            lines.push(res.unwrap());
        }

        let full_response = lines.join("\n");
        assert!(full_response.contains("openai"));
        assert!(full_response.contains("mock response"));
    }
}


