use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const DISCOVERY_PORT: u16 = 54321;
// pub const SYNC_HTTP_PORT: u16 = 54322;
const MAGIC: &str = "MYSYSTEM_V1";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub ip_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryMessage {
    magic: String,
    msg_type: String,
    device_id: Option<String>,
    device_name: Option<String>,
    platform: Option<String>,
    reply_port: Option<u16>,
}

// 监听线程的全局状态
static LISTENER_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));
static LISTENER_STARTED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// 启动 UDP 监听线程：收到 discover 消息时回复 announce
pub fn start_listener(
    device_id: String,
    device_name: String,
    platform: String,
) -> Result<(), String> {
    let mut started = LISTENER_STARTED.lock().unwrap();
    if *started {
        return Ok(());
    }
    *started = true;

    let running = LISTENER_RUNNING.clone();
    running.store(true, Ordering::Relaxed);

    let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))
        .map_err(|e| format!("绑定发现端口 {} 失败: {}", DISCOVERY_PORT, e))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("设置广播失败: {}", e))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| e.to_string())?;

    thread::spawn(move || {
        let mut buf = [0u8; 2048];
        while running.load(Ordering::Relaxed) {
            match socket.recv_from(&mut buf) {
                Ok((size, src)) => {
                    if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..size]) {
                        if msg.magic != MAGIC || msg.msg_type != "discover" {
                            continue;
                        }
                        // 忽略自己发的
                        if msg.device_id.as_deref() == Some(device_id.as_str()) {
                            continue;
                        }
                        if let Some(reply_port) = msg.reply_port {
                            let reply = DiscoveryMessage {
                                magic: MAGIC.to_string(),
                                msg_type: "announce".to_string(),
                                device_id: Some(device_id.clone()),
                                device_name: Some(device_name.clone()),
                                platform: Some(platform.clone()),
                                reply_port: None,
                            };
                            if let Ok(json) = serde_json::to_vec(&reply) {
                                let addr = format!("{}:{}", src.ip(), reply_port);
                                let _ = socket.send_to(&json, addr);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 超时正常，继续循环
                }
                Err(_) => break,
            }
        }
        // 循环退出后重置标志
        let mut started = LISTENER_STARTED.lock().unwrap();
        *started = false;
        println!("[Discovery] 监听线程已停止");
    });

    println!("[Discovery] 监听线程已启动，端口 {}", DISCOVERY_PORT);
    Ok(())
}

/// 停止 UDP 监听
pub fn stop_listener() {
    LISTENER_RUNNING.store(false, Ordering::Relaxed);
    println!("[Discovery] 已发送停止信号");
}

/// 广播 discover 消息，收集局域网内打开的设备的回应
pub fn discover_devices(
    timeout_ms: u64,
    self_device_id: String,
) -> Result<Vec<DeviceInfo>, String> {
    let socket =
        UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| format!("绑定 UDP 套接字失败: {}", e))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("设置广播失败: {}", e))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;

    let reply_port = socket.local_addr().map_err(|e| e.to_string())?.port();

    let request = DiscoveryMessage {
        magic: MAGIC.to_string(),
        msg_type: "discover".to_string(),
        device_id: Some(self_device_id.clone()),
        device_name: None,
        platform: None,
        reply_port: Some(reply_port),
    };

    let request_json = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let broadcast_addr = SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), DISCOVERY_PORT);

    // 广播两次，提高可靠性
    for _ in 0..2 {
        let _ = socket.send_to(&request_json, broadcast_addr);
        thread::sleep(Duration::from_millis(50));
    }

    let start = Instant::now();
    let mut devices: Vec<DeviceInfo> = Vec::new();
    let mut buf = [0u8; 2048];

    while start.elapsed() < Duration::from_millis(timeout_ms) {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                if let Ok(msg) = serde_json::from_slice::<DiscoveryMessage>(&buf[..size]) {
                    if msg.magic != MAGIC || msg.msg_type != "announce" {
                        continue;
                    }
                    if let (Some(id), Some(name), Some(platform)) =
                        (msg.device_id, msg.device_name, msg.platform)
                    {
                        // 过滤自己
                        if id == self_device_id {
                            continue;
                        }
                        // 去重
                        if devices.iter().any(|d| d.device_id == id) {
                            continue;
                        }
                        devices.push(DeviceInfo {
                            device_id: id,
                            device_name: name,
                            platform,
                            ip_address: src.ip().to_string(),
                        });
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
    }

    Ok(devices)
}

// ===== Tauri 命令 =====

#[tauri::command]
pub fn start_discovery_listener(
    device_id: String,
    device_name: String,
    platform: String,
) -> Result<(), String> {
    start_listener(device_id, device_name, platform)
}

#[tauri::command]
pub fn stop_discovery_listener() -> Result<(), String> {
    stop_listener();
    Ok(())
}

#[tauri::command]
pub async fn scan_devices(
    timeout_ms: Option<u64>,
    self_device_id: String,
) -> Result<Vec<DeviceInfo>, String> {
    let timeout = timeout_ms.unwrap_or(2000);
    // 放到阻塞线程里执行，避免卡住 Tauri 主线程
    tauri::async_runtime::spawn_blocking(move || discover_devices(timeout, self_device_id))
        .await
        .map_err(|e| format!("任务失败: {}", e))?
}
