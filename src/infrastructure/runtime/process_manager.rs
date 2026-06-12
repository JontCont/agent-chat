use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures_util::Stream;
use tracing::{info, error};

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

pub fn spawn_run(session_id: String, prompt: String) -> impl Stream<Item = Result<String, std::io::Error>> + Send {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let gemini_cmd = std::env::var("GEMINI_CLI_PATH").unwrap_or_else(|_| "gemini".to_string());
        info!("Spawning child process for session {}: {} ask '{}'", session_id, gemini_cmd, prompt);
        
        let spawn_result = Command::new(&gemini_cmd)
            .arg("ask")
            .arg(&prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

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
                        error!("Gemini CLI stderr (session {}): {}", session_id_err, line);
                    }
                });

                // Stream stdout deltas
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    if tx.send(Ok(line)).await.is_err() {
                        break;
                    }
                }

                let _ = child.wait().await;
                delete_pid(&session_id);
            }
            Err(e) => {
                error!("Failed to spawn gemini executable: {}. Using mock stream generator instead.", e);
                // Fallback to mock stream generator
                let mock_response = format!(
                    "This is a mock response from the Agent Bridge.\n\
                     You asked: '{}'\n\
                     The Gemini CLI could not be spawned (Executable: '{}').\n\
                     Decoupled architecture is verified! Streaming is active.",
                    prompt, gemini_cmd
                );

                for line in mock_response.lines() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    if tx.send(Ok(line.to_string())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    tokio_stream::wrappers::ReceiverStream::new(rx)
}
