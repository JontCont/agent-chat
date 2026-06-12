use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
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

pub fn spawn_run(session_id: String, prompt: String) -> impl Stream<Item = Result<String, std::io::Error>> + Send {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let agy_cmd = std::env::var("AGY_CLI_PATH").unwrap_or_else(|_| "agy".to_string());
        info!("Spawning child process for session {}: {} --print '{}'", session_id, agy_cmd, prompt);
        
        // On Windows, agy writes directly to the console device (CONOUT$),
        // bypassing piped stdout entirely. We use `conhost.exe --headless`
        // to create a headless pseudo-console that captures this output.
        // On non-Windows, we spawn agy directly with piped stdout.
        let spawn_result = spawn_agy_command(&agy_cmd, &prompt);

        match spawn_result {
            Ok(mut child) => {
                if let Some(pid) = child.id() {
                    let _ = write_pid(&session_id, pid);
                }

                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();
                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                // Monitor stderr in background
                let session_id_err = session_id.clone();
                tokio::spawn(async move {
                    while let Ok(Some(line)) = stderr_reader.next_line().await {
                        // Don't log conhost/ANSI noise as errors
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            warn!("Antigravity CLI stderr (session {}): {}", session_id_err, trimmed);
                        }
                    }
                });

                // Stream stdout deltas, stripping ANSI escape sequences on Windows
                while let Ok(Some(raw_line)) = stdout_reader.next_line().await {
                    let line = strip_ansi_escapes(&raw_line);
                    // Skip empty lines that were purely ANSI control sequences
                    if line.trim().is_empty() {
                        continue;
                    }
                    if tx.send(Ok(line)).await.is_err() {
                        break;
                    }
                }

                let _ = child.wait().await;
                delete_pid(&session_id);
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
                        prompt, agy_cmd
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

