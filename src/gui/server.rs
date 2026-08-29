use std::sync::{Arc, RwLock};
use crossbeam_channel::Sender;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::proc::kill_process;
use crate::sampler::SamplerCommand;

const INDEX_HTML: &str = include_str!("assets/index.html");
const STYLE_CSS: &str = include_str!("assets/style.css");
const APP_JS: &str = include_str!("assets/app.js");

pub struct GuiServer {
    #[allow(dead_code)]
    pub port: u16,
}

impl GuiServer {
    pub async fn start(
        bcast_tx: broadcast::Sender<Arc<String>>,
        latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
        cmd_tx: Sender<SamplerCommand>,
    ) -> Result<(Self, String), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let url = format!("http://127.0.0.1:{port}");

        let cmd_tx_clone = cmd_tx.clone();
        let latest_snapshot_clone = latest_snapshot.clone();

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    continue;
                };

                let bcast_rx = bcast_tx.subscribe();
                let cmd_tx = cmd_tx_clone.clone();
                let latest_snap = latest_snapshot_clone.clone();

                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let n = match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => n,
                        _ => return,
                    };

                    let req_str = String::from_utf8_lossy(&buf[..n]);
                    let mut lines = req_str.lines();
                    let Some(first_line) = lines.next() else { return };
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    if parts.len() < 2 { return };

                    let method = parts[0];
                    let path = parts[1];

                    match (method, path) {
                        ("GET", "/") => {
                            send_response(&mut socket, "text/html; charset=utf-8", INDEX_HTML.as_bytes()).await;
                        }
                        ("GET", "/style.css") => {
                            send_response(&mut socket, "text/css; charset=utf-8", STYLE_CSS.as_bytes()).await;
                        }
                        ("GET", "/app.js") => {
                            send_response(&mut socket, "application/javascript; charset=utf-8", APP_JS.as_bytes()).await;
                        }
                        ("GET", "/api/snapshot") => {
                            let snap_json = latest_snap.read().ok().and_then(|l| l.clone());
                            if let Some(json) = snap_json {
                                send_response(&mut socket, "application/json", json.as_bytes()).await;
                            } else {
                                send_response(&mut socket, "application/json", b"{}").await;
                            }
                        }
                        ("GET", "/api/stream") => {
                            handle_sse_stream(socket, bcast_rx, latest_snap).await;
                        }
                        ("POST", "/api/kill") => {
                            handle_kill(socket, &req_str).await;
                        }
                        ("POST", "/api/focus") => {
                            handle_focus(socket, &req_str).await;
                        }
                        ("POST", "/api/detect-tabs") => {
                            let _ = cmd_tx.send(SamplerCommand::RetriggerTabDetection);
                            send_response(&mut socket, "application/json", b"{\"success\":true}").await;
                        }
                        ("POST", "/api/refresh") => {
                            let _ = cmd_tx.send(SamplerCommand::RetriggerTabDetection);
                            send_response(&mut socket, "application/json", b"{\"success\":true}").await;
                        }
                        _ => {
                            let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                            let _ = socket.write_all(not_found.as_bytes()).await;
                        }
                    }
                });
            }
        });

        Ok((GuiServer { port }, url))
    }
}

async fn send_response(socket: &mut tokio::net::TcpStream, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        content_type,
        body.len()
    );
    let _ = socket.write_all(header.as_bytes()).await;
    let _ = socket.write_all(body).await;
}

async fn handle_sse_stream(
    mut socket: tokio::net::TcpStream,
    mut bcast_rx: broadcast::Receiver<Arc<String>>,
    latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
) {
    let sse_header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    if socket.write_all(sse_header.as_bytes()).await.is_err() {
        return;
    }

    // Immediately send the latest snapshot so the browser never sits on a blank/loading screen
    let initial_snapshot = latest_snapshot.read().ok().and_then(|l| l.clone());
    if let Some(json_str) = initial_snapshot {
        let msg = format!("data: {}\n\n", json_str);
        if socket.write_all(msg.as_bytes()).await.is_err() {
            return;
        }
    }

    loop {
        match bcast_rx.recv().await {
            Ok(json_str) => {
                let msg = format!("data: {}\n\n", json_str);
                if socket.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
}

async fn handle_kill(mut socket: tokio::net::TcpStream, req_str: &str) {
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);

    let res_json = match parsed {
        Ok(val) => {
            if let Some(pid) = val.get("pid").and_then(|p| p.as_u64()) {
                match kill_process(pid as u32) {
                    Ok(_) => "{\"success\":true}".to_string(),
                    Err(e) => format!("{{\"success\":false,\"error\":\"{}\"}}", e.replace('"', "\\\"")),
                }
            } else {
                "{\"success\":false,\"error\":\"Missing pid field\"}".to_string()
            }
        }
        Err(e) => format!("{{\"success\":false,\"error\":\"JSON parse error: {}\"}}", e),
    };

    send_response(&mut socket, "application/json", res_json.as_bytes()).await;
}

async fn handle_focus(mut socket: tokio::net::TcpStream, req_str: &str) {
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);

    let res_json = match parsed {
        Ok(val) => {
            let pid = val.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
            let ppid = val.get("ppid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
            let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let port = val.get("port").and_then(|p| p.as_u64()).map(|p| p as u16);

            match crate::chromium::focus_chromium_tab(pid, ppid, title, port).await {
                Ok(_) => "{\"success\":true}".to_string(),
                Err(e) => format!("{{\"success\":false,\"error\":\"{}\"}}", e.replace('"', "\\\"")),
            }
        }
        Err(e) => format!("{{\"success\":false,\"error\":\"JSON parse error: {}\"}}", e),
    };

    send_response(&mut socket, "application/json", res_json.as_bytes()).await;
}
