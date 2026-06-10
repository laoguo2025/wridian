use crate::cocreation::read_model_response_text;
use crate::runtime::{ensure_workspace, model_accounts_path, wridian_data_dir};
use keyring_core::{set_default_store, Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;
use windows_native_keyring_store::Store;

const KEYRING_SERVICE: &str = "ai.wridian.app";
const CUSTOM_API_KEYRING_USER: &str = "custom-api-key";

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCustomApiSettingsFile {
    base_url: String,
    model: String,
    #[serde(default)]
    key_stored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
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

#[tauri::command]
pub(crate) fn wridian_clear_custom_api_settings() -> Result<CustomApiSettingsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    clear_api_key()?;
    let path = model_accounts_path(&data_dir);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("模型账户配置清除失败：{error}"))?;
    }
    Ok(CustomApiSettingsStatus {
        configured: false,
        base_url: None,
        model: None,
        masked_key: None,
    })
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
    validate_base_url(&base_url)?;
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
    let stored: StoredCustomApiSettingsFile = serde_json::from_value(custom_api.clone())
        .map_err(|error| format!("自定义 API 配置格式损坏：{error}"))?;
    let api_key = if let Some(legacy_key) = stored
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        store_api_key(legacy_key)?;
        write_custom_api_settings_file(
            data_dir,
            &StoredCustomApiSettingsFile {
                base_url: stored.base_url.clone(),
                model: stored.model.clone(),
                key_stored: true,
                api_key: None,
            },
        )?;
        legacy_key.to_string()
    } else if stored.key_stored {
        read_api_key()?
    } else {
        String::new()
    };
    if api_key.is_empty() {
        return Ok(None);
    }
    Ok(Some(StoredCustomApiSettings {
        base_url: stored.base_url,
        api_key,
        model: stored.model,
    }))
}

fn write_custom_api_settings(
    data_dir: &Path,
    settings: &StoredCustomApiSettings,
) -> Result<(), String> {
    store_api_key(&settings.api_key)?;
    write_custom_api_settings_file(
        data_dir,
        &StoredCustomApiSettingsFile {
            base_url: settings.base_url.clone(),
            model: settings.model.clone(),
            key_stored: true,
            api_key: None,
        },
    )
}

fn write_custom_api_settings_file(
    data_dir: &Path,
    settings: &StoredCustomApiSettingsFile,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "customApi": settings
    }))
    .map_err(|error| error.to_string())?;
    fs::write(model_accounts_path(data_dir), content)
        .map_err(|error| format!("模型账户配置写入失败：{error}"))
}

fn api_key_entry() -> Result<Entry, String> {
    set_default_store(Store::new().map_err(|error| format!("系统凭据存储初始化失败：{error}"))?);
    Entry::new(KEYRING_SERVICE, CUSTOM_API_KEYRING_USER)
        .map_err(|error| format!("系统凭据项创建失败：{error}"))
}

fn store_api_key(api_key: &str) -> Result<(), String> {
    api_key_entry()?
        .set_password(api_key)
        .map_err(|error| format!("API Key 写入系统凭据失败：{error}"))
}

fn clear_api_key() -> Result<(), String> {
    match api_key_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("API Key 清除失败：{error}")),
    }
}

fn read_api_key() -> Result<String, String> {
    match api_key_entry()?.get_password() {
        Ok(api_key) => Ok(api_key),
        Err(KeyringError::NoEntry) => Ok(String::new()),
        Err(error) => Err(format!("API Key 读取系统凭据失败：{error}")),
    }
}

fn validate_base_url(base_url: &str) -> Result<(), String> {
    if base_url.starts_with("https://") {
        return Ok(());
    }
    if let Some(authority) = base_url.strip_prefix("http://") {
        let authority = authority.split('/').next().unwrap_or("");
        let host = if authority.starts_with('[') {
            authority
                .find(']')
                .map(|end| &authority[..=end])
                .unwrap_or(authority)
        } else {
            authority.split(':').next().unwrap_or("")
        };
        if matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]") {
            return Ok(());
        }
    }
    Err("Base URL 必须使用 https；只有 localhost/127.0.0.1 允许 http 本地调试。".to_string())
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
    if status.is_success() {
        let body = response
            .text()
            .await
            .map_err(|error| format!("自定义 API 响应读取失败：{error}"))?;
        let content = read_model_response_text(&body)?;
        if content.trim().is_empty() {
            return Err("自定义 API 返回了空文本，无法用于 Wridian 对话。".to_string());
        }
        Ok(TestCustomApiResponse {
            ok: true,
            message: "连接成功，且响应格式可用于 Wridian 对话。".to_string(),
        })
    } else {
        let body = response
            .text()
            .await
            .map_err(|error| format!("自定义 API 响应读取失败：{error}"))?;
        Err(format!(
            "自定义 API 测试失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::unique_test_suffix;
    use std::env;

    #[test]
    fn custom_api_base_url_requires_https_except_localhost() {
        assert!(validate_base_url("https://api.example.com/v1").is_ok());
        assert!(validate_base_url("http://localhost:8080/v1").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(validate_base_url("http://[::1]:8080/v1").is_ok());
        assert!(validate_base_url("http://api.example.com/v1").is_err());
    }

    #[test]
    fn custom_api_settings_file_omits_api_key_field() {
        let data_dir =
            env::temp_dir().join(format!("wridian-model-settings-{}", unique_test_suffix()));
        ensure_workspace(&data_dir).expect("workspace");
        write_custom_api_settings_file(
            &data_dir,
            &StoredCustomApiSettingsFile {
                base_url: "https://api.example.com/v1".to_string(),
                model: "example-model".to_string(),
                key_stored: true,
                api_key: None,
            },
        )
        .expect("write settings");

        let content = fs::read_to_string(model_accounts_path(&data_dir)).expect("read settings");
        let value: serde_json::Value = serde_json::from_str(&content).expect("json");
        let custom_api = value
            .get("customApi")
            .and_then(serde_json::Value::as_object)
            .expect("customApi object");

        assert_eq!(
            custom_api
                .get("keyStored")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(!custom_api.contains_key("apiKey"));

        let _ = fs::remove_dir_all(data_dir);
    }
}
