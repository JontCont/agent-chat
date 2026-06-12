use crate::application::ports::runtime_gateway::RuntimeGateway;
use reqwest::Client;
use futures_util::{Stream, StreamExt};
use std::io;
use std::future::Future;
use std::pin::Pin;
use tracing::{info, error};

pub struct DaemonClient {
    client: Client,
    daemon_url: String,
}

impl DaemonClient {
    pub fn new(daemon_url: String) -> Self {
        Self {
            client: Client::new(),
            daemon_url,
        }
    }
}

impl RuntimeGateway for DaemonClient {
    fn send_message(&self, session_id: &str, content: &str) -> Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let url = format!("{}/local/sessions/{}/messages", self.daemon_url, session_id);
        let client = self.client.clone();
        let content = content.to_string();

        tokio::spawn(async move {
            info!("Sending prompt to Daemon endpoint: {}", url);
            let res = client.post(&url)
                .json(&serde_json::json!({ "content": content }))
                .send()
                .await;

            let response = match res {
                Ok(r) => r,
                Err(e) => {
                    error!("Connection to Daemon refused or failed: {:?}", e);
                    let _ = tx.send(Err(io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))).await;
                    return;
                }
            };

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_res) = stream.next().await {
                let chunk = match chunk_res {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        error!("Error reading stream chunk from Daemon: {:?}", e);
                        let _ = tx.send(Err(io::Error::new(io::ErrorKind::Other, e.to_string()))).await;
                        return;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);
                
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim_end().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if !line.is_empty() {
                        if tx.send(Ok(line)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn cancel_run(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let url = format!("{}/local/sessions/{}/cancel", self.daemon_url, session_id);
        let client = self.client.clone();
        Box::pin(async move {
            info!("Sending cancel request to Daemon at: {}", url);
            let res = client.post(&url).send().await;
            match res {
                Ok(resp) if resp.status().is_success() => Ok(()),
                Ok(resp) => {
                    error!("Daemon returned failure status on cancel: {}", resp.status());
                    Err(io::Error::new(io::ErrorKind::Other, format!("Daemon returned error status: {}", resp.status())))
                }
                Err(e) => {
                    error!("Failed to connect to Daemon for cancellation: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))
                }
            }
        })
    }

    fn delete_session(&self, session_id: &str) -> Pin<Box<dyn Future<Output = Result<(), io::Error>> + Send>> {
        let url = format!("{}/local/sessions/{}", self.daemon_url, session_id);
        let client = self.client.clone();
        Box::pin(async move {
            info!("Sending DELETE request to Daemon at: {}", url);
            let res = client.delete(&url).send().await;
            match res {
                Ok(resp) if resp.status().is_success() => Ok(()),
                Ok(resp) => {
                    error!("Daemon returned failure status on DELETE: {}", resp.status());
                    Err(io::Error::new(io::ErrorKind::Other, format!("Daemon returned error status: {}", resp.status())))
                }
                Err(e) => {
                    error!("Failed to connect to Daemon for session delete: {:?}", e);
                    Err(io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))
                }
            }
        })
    }
}
