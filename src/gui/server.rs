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
                        ("POST", "/api/trim") => {
                            handle_trim(socket, &req_str).await;
                        }
                        ("POST", "/api/export-parquet") => {
                            handle_export_parquet(socket, latest_snap).await;
                        }
                        ("POST", "/api/detect-tabs") => {
                            let _ = cmd_tx.send(SamplerCommand::RetriggerTabDetection);
                            send_response(&mut socket, "application/json", b"{\"success\":true}").await;
                        }
                        ("POST", "/api/refresh") => {
                            let _ = cmd_tx.send(SamplerCommand::RetriggerTabDetection);
                            send_response(&mut socket, "application/json", b"{\"success\":true}").await;
                        }
                        ("POST", "/api/copilot/park") => {
                            handle_copilot_park(socket, latest_snap).await;
                        }
                        ("POST", "/api/copilot/action") => {
                            handle_copilot_action(socket, &req_str).await;
                        }
                        ("GET", "/api/copilot/stashes") => {
                            handle_copilot_list_stashes(socket).await;
                        }
                        ("POST", "/api/copilot/restore") => {
                            handle_copilot_restore(socket, &req_str).await;
                        }
                        ("GET", "/api/copilot/status") => {
                            let status = crate::notifications::badge::global_badge().get_status();
                            let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/copilot/anchor") => {
                            handle_copilot_anchor(socket, latest_snap).await;
                        }
                        ("GET", "/api/copilot/clipboard") => {
                            handle_copilot_clipboard(socket).await;
                        }
                        ("POST", "/api/copilot/stash-rabbithole") => {
                            handle_copilot_stash_rabbithole(socket, latest_snap).await;
                        }
                        ("GET", "/api/knowledge/briefing") => {
                            let briefing = crate::knowledge::orientation::ExecutiveBriefing::generate();
                            let json = serde_json::to_string(&briefing).unwrap_or_else(|_| "{}".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/knowledge/profile") => {
                            let profile = {
                                let mut adapt = crate::knowledge::adaptive_engine::global_adaptive().lock().unwrap();
                                adapt.load_or_recalibrate().clone()
                            };
                            let json = serde_json::to_string(&profile).unwrap_or_else(|_| "{}".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/knowledge/history") => {
                            let history = {
                                let cons = crate::knowledge::consolidator::global_consolidator().lock().unwrap();
                                cons.get_recent_profiles(14)
                            };
                            let json = serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/knowledge/tab-analytics") => {
                            let report = crate::knowledge::tab_analytics::TabAnalyticsReport::generate();
                            let json = serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/knowledge/explorer-analytics") => {
                            let (top_roots, forgotten) = {
                                let store = crate::knowledge::explorer_history::global_explorer_history().lock().unwrap();
                                let roots = store.get_top_workspace_roots(8);
                                let forg = store.get_forgotten_explorer_tabs();
                                (roots, forg)
                            };
                            let active_open = crate::explorer::resolve_open_explorer_folders();
                            let payload = serde_json::json!({
                                "active_open": active_open,
                                "top_roots": top_roots,
                                "forgotten_tabs": forgotten,
                            });
                            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
                        }
                        ("GET", "/api/knowledge/closure") => {
                            let closure = crate::knowledge::closure::DailyClosureSummary::load_latest(std::path::Path::new("memoryblast_data"));
                            let json = serde_json::to_string(&closure).unwrap_or_else(|_| "null".to_string());
                            send_response(&mut socket, "application/json", json.as_bytes()).await;
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

async fn handle_trim(mut socket: tokio::net::TcpStream, req_str: &str) {
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);

    let res_json = match parsed {
        Ok(val) => {
            if let Some(pid) = val.get("pid").and_then(|p| p.as_u64()) {
                match crate::optimizer::OptimizationAssistant::trim_working_set(pid as u32) {
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

async fn handle_export_parquet(
    mut socket: tokio::net::TcpStream,
    latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
) {
    let snap_json = latest_snapshot.read().ok().and_then(|l| l.clone());
    let res_json = if let Some(json) = snap_json {
        if let Ok(snap) = serde_json::from_str::<crate::sampler::SystemSnapshot>(&json) {
            let pipeline = crate::telemetry::TelemetryPipeline::new(10, "memoryblast_data");
            match pipeline.export_snapshot_to_parquet(&snap) {
                Ok(path) => format!(
                    "{{\"success\":true,\"path\":\"{}\"}}",
                    path.to_string_lossy().replace('\\', "/")
                ),
                Err(e) => format!("{{\"success\":false,\"error\":\"{}\"}}", e),
            }
        } else {
            "{\"success\":false,\"error\":\"Failed to parse snapshot\"}".to_string()
        }
    } else {
        "{\"success\":false,\"error\":\"No snapshot available\"}".to_string()
    };

    send_response(&mut socket, "application/json", res_json.as_bytes()).await;
}

async fn handle_copilot_park(
    mut socket: tokio::net::TcpStream,
    latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
) {
    let snap_json = latest_snapshot.read().ok().and_then(|l| l.clone());
    let mut candidate_pids = Vec::new();
    let mut known_ports = Vec::new();

    if let Some(json) = snap_json {
        if let Ok(snap) = serde_json::from_str::<crate::sampler::SystemSnapshot>(&json) {
            for p in &snap.processes {
                if p.is_chromium && (p.proc_type_label == "Renderer" || !p.display_title.is_empty()) {
                    candidate_pids.push((p.pid, p.display_title.clone(), p.working_set_bytes));
                }
                if let Some(port) = p.debug_port {
                    if !known_ports.contains(&port) {
                        known_ports.push(port);
                    }
                }
            }
        }
    }

    let park_res = crate::copilot::park_current_context(&known_ports, &candidate_pids).await;
    let res = match park_res {
        Ok(session) => serde_json::json!({
            "success": true,
            "session": session,
        }),
        Err(e) => serde_json::json!({
            "success": false,
            "error": e.to_string(),
        }),
    };

    let json_bytes = serde_json::to_vec(&res).unwrap_or_default();
    send_response(&mut socket, "application/json", &json_bytes).await;
}

async fn handle_copilot_action(mut socket: tokio::net::TcpStream, req_str: &str) {
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);

    let res_json = match parsed {
        Ok(val) => {
            let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let pid = val.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
            let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("");
            let port = val.get("port").and_then(|p| p.as_u64()).map(|p| p as u16);

            match crate::copilot::handle_triage_action(action, pid, title, port).await {
                Ok(msg) => format!("{{\"success\":true,\"message\":\"{}\"}}", msg),
                Err(e) => format!("{{\"success\":false,\"error\":\"{}\"}}", e.replace('"', "\\\"")),
            }
        }
        Err(e) => format!("{{\"success\":false,\"error\":\"JSON parse error: {}\"}}", e),
    };

    send_response(&mut socket, "application/json", res_json.as_bytes()).await;
}

async fn handle_copilot_list_stashes(mut socket: tokio::net::TcpStream) {
    let res = {
        let manager_res = crate::copilot::StashManager::new(std::path::Path::new("memoryblast_data"))
            .map_err(|e| e.to_string());
        match manager_res {
            Ok(mgr) => match mgr.list_sessions().map_err(|e| e.to_string()) {
                Ok(sessions) => serde_json::json!({
                    "success": true,
                    "sessions": sessions,
                }),
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": e,
                }),
            },
            Err(e) => serde_json::json!({
                "success": false,
                "error": e,
            }),
        }
    };

    let json_bytes = serde_json::to_vec(&res).unwrap_or_default();
    send_response(&mut socket, "application/json", &json_bytes).await;
}

async fn handle_copilot_restore(mut socket: tokio::net::TcpStream, req_str: &str) {
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);

    let res_json = match parsed {
        Ok(val) => {
            if let Some(id) = val.get("id").and_then(|i| i.as_i64()) {
                match crate::copilot::restore_session(id) {
                    Ok(cnt) => format!("{{\"success\":true,\"restored_count\":{cnt}}}"),
                    Err(e) => format!("{{\"success\":false,\"error\":\"{}\"}}", e.to_string().replace('"', "\\\"")),
                }
            } else {
                "{\"success\":false,\"error\":\"Missing session id\"}".to_string()
            }
        }
        Err(e) => format!("{{\"success\":false,\"error\":\"JSON parse error: {}\"}}", e),
    };

    send_response(&mut socket, "application/json", res_json.as_bytes()).await;
}

async fn handle_copilot_anchor(
    mut socket: tokio::net::TcpStream,
    latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
) {
    let anchor = crate::copilot::anchor::load_latest_anchor(std::path::Path::new("memoryblast_data"))
        .unwrap_or_else(|| {
            let snap_json = latest_snapshot.read().ok().and_then(|l| l.clone());
            let mut tabs = Vec::new();
            if let Some(json) = snap_json {
                if let Ok(snap) = serde_json::from_str::<crate::sampler::SystemSnapshot>(&json) {
                    for p in snap.processes {
                        if p.is_chromium && !p.display_title.is_empty() {
                            tabs.push((p.display_title, String::new()));
                        }
                    }
                }
            }
            crate::copilot::anchor::ResumeAnchor::capture(std::path::Path::new("."), &tabs)
        });

    let res = serde_json::json!({
        "success": true,
        "anchor": anchor,
    });
    let json_bytes = serde_json::to_vec(&res).unwrap_or_default();
    send_response(&mut socket, "application/json", &json_bytes).await;
}

async fn handle_copilot_clipboard(mut socket: tokio::net::TcpStream) {
    let events = {
        let clip_lock = crate::copilot::clipboard::global_clipboard().lock().unwrap();
        clip_lock.get_recent(20)
    };

    let res = serde_json::json!({
        "success": true,
        "events": events,
    });
    let json_bytes = serde_json::to_vec(&res).unwrap_or_default();
    send_response(&mut socket, "application/json", &json_bytes).await;
}

async fn handle_copilot_stash_rabbithole(
    mut socket: tokio::net::TcpStream,
    latest_snapshot: Arc<RwLock<Option<Arc<String>>>>,
) {
    let snap_json = latest_snapshot.read().ok().and_then(|l| l.clone());
    let mut divergent_tabs = Vec::new();
    let mut candidate_pids = Vec::new();

    if let Some(json) = snap_json {
        if let Ok(snap) = serde_json::from_str::<crate::sampler::SystemSnapshot>(&json) {
            for p in &snap.processes {
                if p.is_chromium && !p.display_title.is_empty() {
                    if crate::copilot::fixation::is_tangent_topic(&p.display_title) {
                        divergent_tabs.push((p.display_title.clone(), String::new()));
                        candidate_pids.push(p.pid);
                    }
                }
            }
        }
    }

    let drawer_name = format!(
        "CuriosityDrawer_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );

    let saved_path = {
        let fixation_lock = crate::copilot::fixation::global_fixation().lock().unwrap();
        fixation_lock.stash_into_curiosity_drawer(&drawer_name, &divergent_tabs)
    };

    // Terminate candidate processes to reclaim RAM and close tabs
    for pid in candidate_pids {
        let _ = crate::proc::kill_process(pid);
    }

    let res = match saved_path {
        Ok(path) => serde_json::json!({
            "success": true,
            "drawer_name": drawer_name,
            "path": path.to_string_lossy(),
            "stashed_count": divergent_tabs.len(),
        }),
        Err(e) => serde_json::json!({
            "success": false,
            "error": e.to_string(),
        }),
    };

    let json_bytes = serde_json::to_vec(&res).unwrap_or_default();
    send_response(&mut socket, "application/json", &json_bytes).await;
}

