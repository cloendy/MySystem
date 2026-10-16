use crate::platform::sync_payload::{SyncPayload, SyncResponse, merge_payload, read_local_payload};
use once_cell::sync::Lazy;
use rusqlite;
use serde::Serialize;
// use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Manager;
use tiny_http::{Header, Method, Request, Response, Server};

pub const SYNC_HTTP_PORT: u16 = 54322;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeviceInfoResponse {
    device_id: String,
    device_name: String,
    platform: String,
    protocol_version: u32,
}

static HTTP_SERVER: Lazy<Mutex<Option<Arc<Server>>>> = Lazy::new(|| Mutex::new(None));
static HTTP_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

fn json_response<T: Serialize>(value: &T, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let header = Header::from_bytes("Content-Type", "application/json").unwrap();
    Response::from_string(body)
        .with_status_code(status)
        .with_header(header)
}

fn text_response(text: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(text).with_status_code(status)
}

fn handle_request(
    request: Request,
    device_id: &str,
    device_name: &str,
    platform: &str,
    app_handle: &tauri::AppHandle,
) {
    let method = request.method().clone();
    let url = request.url().to_string();

    // 去掉 query 参数，只保留路径
    let path = url.split('?').next().unwrap_or(&url).to_string();

    println!("[HTTP] {} {}", method, path);

    match (method, path.as_str()) {
        (Method::Get, "/api/info") => {
            let info = DeviceInfoResponse {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                platform: platform.to_string(),
                protocol_version: 1,
            };
            let _ = request.respond(json_response(&info, 200));
        }
        (Method::Post, "/api/sync/exchange") => {
            handle_sync_exchange(request, app_handle, device_id, device_name);
        }
        _ => {
            let _ = request.respond(text_response("Not Found", 404));
        }
    }
}

fn handle_sync_exchange(
    mut request: Request,
    app_handle: &tauri::AppHandle,
    device_id: &str,
    device_name: &str,
) {
    // 读取请求体
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        let _ = request.respond(text_response(&format!("读取请求失败: {}", e), 400));
        return;
    }

    let incoming: SyncPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            let _ = request.respond(text_response(&format!("解析请求失败: {}", e), 400));
            return;
        }
    };

    println!(
        "[HTTP] 收到来自 {} 的同步请求，文档数 {}，笔记数 {}",
        incoming.sender_device_name,
        incoming.documents.len(),
        incoming.notes.len()
    );

    // 打开本地数据库
    let dir = match app_handle.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            let _ = request.respond(text_response(&format!("获取数据目录失败: {}", e), 500));
            return;
        }
    };
    let path = dir.join("mydata.db");
    let mut conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            let _ = request.respond(text_response(&format!("打开数据库失败: {}", e), 500));
            return;
        }
    };

    // 合并
    let (accepted, rejected, merged) = match merge_payload(&mut conn, &incoming) {
        Ok(r) => r,
        Err(e) => {
            let _ = request.respond(text_response(&format!("合并失败: {}", e), 500));
            return;
        }
    };

    // 读取合并后的本地全量数据返回给发起方
    let server_payload = match read_local_payload(&conn, device_id, device_name) {
        Ok(p) => p,
        Err(e) => {
            let _ = request.respond(text_response(&format!("读取本地数据失败: {}", e), 500));
            return;
        }
    };

    let response = SyncResponse {
        accepted,
        rejected,
        merged,
        server_payload,
    };

    println!(
        "[HTTP] 同步完成: 新增 {}，合并 {}，拒绝 {}",
        accepted, merged, rejected
    );

    let _ = request.respond(json_response(&response, 200));
}

/// 启动 HTTP 服务端
pub fn start_http_server(
    device_id: String,
    device_name: String,
    platform: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = HTTP_SERVER.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }

    let server = Server::http(("0.0.0.0", SYNC_HTTP_PORT))
        .map_err(|e| format!("绑定 HTTP 端口 {} 失败: {}", SYNC_HTTP_PORT, e))?;
    let server = Arc::new(server);
    *guard = Some(server.clone());

    let running = HTTP_RUNNING.clone();
    running.store(true, Ordering::Relaxed);

    let app_handle_clone = app_handle.clone();
    thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            match server.recv_timeout(Duration::from_millis(500)) {
                Ok(Some(request)) => {
                    handle_request(
                        request,
                        &device_id,
                        &device_name,
                        &platform,
                        &app_handle_clone,
                    );
                }
                Ok(None) => {
                    // 超时，继续循环
                }
                Err(e) => {
                    eprintln!("[HTTP] 接收请求出错: {}", e);
                    break;
                }
            }
        }
        println!("[HTTP] 服务端线程已退出");
    });

    println!("[HTTP] 服务端已启动，端口 {}", SYNC_HTTP_PORT);
    Ok(())
}

/// 停止 HTTP 服务端
pub fn stop_http_server() {
    HTTP_RUNNING.store(false, Ordering::Relaxed);
    let mut guard = HTTP_SERVER.lock().unwrap();
    *guard = None;
    println!("[HTTP] 已发送停止信号");
}

// ===== Tauri 命令 =====

#[tauri::command]
pub fn start_sync_server(
    app: tauri::AppHandle,
    device_id: String,
    device_name: String,
    platform: String,
) -> Result<(), String> {
    start_http_server(device_id, device_name, platform, app)
}

#[tauri::command]
pub fn stop_sync_server() -> Result<(), String> {
    stop_http_server();
    Ok(())
}
