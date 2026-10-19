use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub protocol_version: u32,
}

#[tauri::command]
pub async fn check_remote_device(ip_address: String) -> Result<RemoteDeviceInfo, String> {
    let url = format!("http://{}:{}/api/info", ip_address, 54322);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("对方返回错误状态码: {}", resp.status()));
    }

    let info: RemoteDeviceInfo = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    crate::log_info!(
        "[HTTP Client] 握手成功: {} ({}) @ {}",
        info.device_name, info.platform, ip_address
    );

    Ok(info)
}

use crate::platform::sync_payload::{SyncPayload, SyncResponse};

#[tauri::command]
pub async fn exchange_sync_data(
    ip_address: String,
    payload: SyncPayload,
) -> Result<SyncResponse, String> {
    let url = format!("http://{}:{}/api/sync/exchange", ip_address, 54322);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("对方返回错误状态码: {}", resp.status()));
    }

    let response: SyncResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    crate::log_info!(
        "[HTTP Client] 同步完成: 新增 {}，合并 {}，拒绝 {}",
        response.accepted, response.merged, response.rejected
    );

    Ok(response)
}
