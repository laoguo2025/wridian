use crate::runtime::{ensure_workspace, model_accounts_path, wridian_data_dir};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomApiSettingsInput {
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomApiSettingsStatus {
    configured: bool,
    base_url: Option<String>,
    model: Option<String>,
    masked_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestCustomApiResponse {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredCustomApiSettings {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
}

#[tauri::command]
pub(crate) fn wridian_get_custom_api_settings() -> Result<CustomApiSettingsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    Ok(read_custom_api_settings(&data_dir)?
        .map(|settings| CustomApiSettingsStatus {
            configured: true,
            base_url: Some(settings.base_url),
            model: Some(settings.model),
            masked_key: Some(mask_api_key(&settings.api_key)),
        })
        .unwrap_or(CustomApiSettingsStatus {
            configured: false,
            base_url: None,
            model: None,
            masked_key: None,
        }))
}

#[tauri::command]
pub(crate) fn wridian_save_custom_api_settings(
    input: CustomApiSettingsInput,
) -> Result<CustomApiSettingsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let settings = normalize_custom_api_settings(input, read_custom_api_settings(&data_dir)?)?;
    write_custom_api_settings(&data_dir, &settings)?;
    Ok(CustomApiSettingsStatus {
        configured: true,
        base_url: Some(settings.base_url),
        model: Some(settings.model),
        masked_key: Some(mask_api_key(&settings.api_key)),
    })
}

#[tauri::command]
pub(crate) async fn wridian_test_custom_api() -> Result<TestCustomApiResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let settings = read_custom_api_settings(&data_dir)?
        .ok_or_else(|| "请先保存自定义 API 配置。".to_string())?;
    test_openai_compatible_chat(&settings).await
}

fn normalize_custom_api_settings(
    input: CustomApiSettingsInput,
    existing: Option<StoredCustomApiSettings>,
) -> Result<StoredCustomApiSettings, String> {
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let mut api_key = input.api_key.trim().to_string();
    if api_key.is_empty() {
        api_key = existing
            .as_ref()
            .map(|settings| settings.api_key.clone())
            .unwrap_or_default();
    }
    let model = input.model.trim().to_string();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("Base URL 必须是 http 或 https 地址。".to_string());
    }
    if api_key.is_empty() {
        return Err("API Key 不能为空。".to_string());
    }
    if model.is_empty() {
        return Err("模型名不能为空。".to_string());
    }
    Ok(StoredCustomApiSettings {
        base_url,
        api_key,
        model,
    })
}

pub(crate) fn read_custom_api_settings(
    data_dir: &Path,
) -> Result<Option<StoredCustomApiSettings>, String> {
    let path = model_accounts_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("模型账户配置读取失败：{error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("模型账户配置格式损坏：{error}"))?;
    let Some(custom_api) = value.get("customApi") else {
        return Ok(None);
    };
    serde_json::from_value(custom_api.clone())
        .map(Some)
        .map_err(|error| format!("自定义 API 配置格式损坏：{error}"))
}

fn write_custom_api_settings(
    data_dir: &Path,
    settings: &StoredCustomApiSettings,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "customApi": settings
    }))
    .map_err(|error| error.to_string())?;
    fs::write(model_accounts_path(data_dir), content)
        .map_err(|error| format!("模型账户配置写入失败：{error}"))
}

fn mask_api_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    if trimmed.chars().count() <= 8 {
        return "********".to_string();
    }
    let prefix: String = trimmed.chars().take(4).collect();
    let suffix: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}...{suffix}")
}

async fn test_openai_compatible_chat(
    settings: &StoredCustomApiSettings,
) -> Result<TestCustomApiResponse, String> {
    let url = format!("{}/chat/completions", settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("自定义 API 客户端创建失败：{error}"))?;
    let response = client
        .post(url)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": settings.model,
            "messages": [
                { "role": "user", "content": "Reply with OK." }
            ],
            "max_tokens": 8,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|error| format!("自定义 API 连接失败：{error}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(TestCustomApiResponse {
            ok: true,
            message: "连接成功。".to_string(),
        })
    } else {
        Err(format!(
            "自定义 API 测试失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ))
    }
}
