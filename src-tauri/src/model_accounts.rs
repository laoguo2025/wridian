use crate::cocreation::read_model_response_text;
use crate::runtime::{ensure_workspace, model_accounts_path, wridian_data_dir};
use base64::Engine;
use keyring_core::{set_default_store, Entry, Error as KeyringError};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_native_keyring_store::Store;

const KEYRING_SERVICE: &str = "ai.wridian.app";
const LEGACY_CUSTOM_API_KEYRING_USER: &str = "custom-api-key";
const DEFAULT_CUSTOM_PROVIDER_ID: &str = "custom-openai-compatible";
const ANTHROPIC_OAUTH_PROVIDER_ID: &str = "anthropic-official";
const ANTHROPIC_OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const ANTHROPIC_OAUTH_AUTH_ENDPOINT: &str = "https://claude.ai/oauth/authorize";
const ANTHROPIC_OAUTH_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";
const ANTHROPIC_OAUTH_REFRESH_ENDPOINT: &str = "https://platform.claude.com/v1/oauth/token";
const ANTHROPIC_OAUTH_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
const ANTHROPIC_OAUTH_SCOPES: &str = "org:create_api_key user:profile user:inference";
const ANTHROPIC_OAUTH_REFRESH_SKEW_SECONDS: u64 = 300;
const OPENAI_OAUTH_PROVIDER_ID: &str = "openai-official";
const OPENAI_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_OAUTH_AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_OAUTH_TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const OPENAI_OAUTH_REDIRECT_BIND_HOST: &str = "127.0.0.1";
const OPENAI_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const OPENAI_OAUTH_CALLBACK_PATH: &str = "/auth/callback";
const OPENAI_OAUTH_REFRESH_SKEW_SECONDS: u64 = 300;
const GOOGLE_GEMINI_OAUTH_PROVIDER_ID: &str = "google-gemini-cli";
const GOOGLE_OAUTH_CLIENT_ID_ENV: &str = "WRIDIAN_GOOGLE_OAUTH_CLIENT_ID";
const GOOGLE_OAUTH_CLIENT_SECRET_ENV: &str = "WRIDIAN_GOOGLE_OAUTH_CLIENT_SECRET";
const GOOGLE_OAUTH_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_OAUTH_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_OAUTH_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v1/userinfo";
const GOOGLE_OAUTH_SCOPES: &str =
    "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email";
const GOOGLE_OAUTH_REDIRECT_HOST: &str = "127.0.0.1";
const GOOGLE_OAUTH_CALLBACK_PATH: &str = "/oauth2callback";
const GOOGLE_OAUTH_REFRESH_SKEW_SECONDS: u64 = 300;
pub(crate) const GEMINI_DEFAULT_MAX_OUTPUT_TOKENS: u32 = 65535;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestModelProviderResponse {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoogleGeminiOauthResponse {
    email: Option<String>,
    status: ModelAccountsStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnthropicOauthStartResponse {
    session_id: String,
    auth_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnthropicOauthCompleteInput {
    session_id: String,
    code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderOauthResponse {
    email: Option<String>,
    status: ModelAccountsStatus,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfiguredModelStatus {
    id: String,
    label: String,
    provider_id: String,
    provider_name: String,
    protocol: String,
    model: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelProviderStatus {
    id: String,
    preset_key: Option<String>,
    provider_name: String,
    provider_type: Option<String>,
    protocol: String,
    auth_style: String,
    configured: bool,
    base_url: Option<String>,
    models: Vec<String>,
    masked_key: Option<String>,
    extra_env: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelAccountsStatus {
    active_model_id: Option<String>,
    active_model_label: Option<String>,
    configured_models: Vec<ConfiguredModelStatus>,
    providers: Vec<ModelProviderStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveModelProviderInput {
    #[serde(default)]
    preset_key: String,
    provider_id: String,
    provider_name: String,
    #[serde(default)]
    provider_type: String,
    protocol: String,
    #[serde(default)]
    auth_style: String,
    base_url: String,
    api_key: String,
    models: Vec<String>,
    #[serde(default)]
    extra_env: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectActiveModelInput {
    model_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestModelProviderInput {
    provider_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestModelProviderConfigInput {
    #[serde(default)]
    provider_id: String,
    provider_name: String,
    protocol: String,
    #[serde(default)]
    auth_style: String,
    base_url: String,
    api_key: String,
    models: Vec<String>,
    #[serde(default)]
    extra_env: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteModelProviderInput {
    provider_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveModelSettings {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) protocol: String,
    pub(crate) auth_style: String,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) model_id: String,
    pub(crate) extra_env: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredModelAccountsFile {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_model_id: Option<String>,
    #[serde(default)]
    providers: Vec<StoredModelProviderFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredModelProviderFile {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preset_key: Option<String>,
    provider_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_type: Option<String>,
    protocol: String,
    #[serde(default)]
    auth_style: String,
    base_url: String,
    models: Vec<String>,
    #[serde(default)]
    extra_env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    key_stored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredGoogleOauthCredentials {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    access_token: String,
    expires_at: u64,
    email: Option<String>,
}

struct GoogleOauthClientConfig {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredAnthropicOauthCredentials {
    client_id: String,
    refresh_token: String,
    access_token: String,
    expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredOpenAiOauthCredentials {
    client_id: String,
    refresh_token: String,
    access_token: String,
    id_token: Option<String>,
    expires_at: u64,
    email: Option<String>,
    account_id: Option<String>,
    plan: Option<String>,
}

#[tauri::command]
pub(crate) fn wridian_get_model_accounts() -> Result<ModelAccountsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    model_accounts_status(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_save_model_provider(
    input: SaveModelProviderInput,
) -> Result<ModelAccountsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut file = read_model_accounts_file(&data_dir)?;
    let existing = file
        .providers
        .iter()
        .find(|provider| provider.id == input.provider_id.trim())
        .cloned();
    let provider = normalize_model_provider(input, existing)?;
    if let Some(position) = file
        .providers
        .iter()
        .position(|item| item.id == provider.id)
    {
        file.providers[position] = provider.clone();
    } else {
        file.providers.push(provider.clone());
    }
    if file.active_model_id.is_none() {
        file.active_model_id = provider
            .models
            .first()
            .map(|model| model_config_id(&provider.id, model));
    }
    write_model_accounts_file(&data_dir, &file)?;
    model_accounts_status(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_select_active_model(
    input: SelectActiveModelInput,
) -> Result<ModelAccountsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut file = read_model_accounts_file(&data_dir)?;
    let model_id = input.model_id.trim();
    if model_id.is_empty() {
        file.active_model_id = None;
    } else if configured_models(&file)
        .iter()
        .any(|model| model.id == model_id)
    {
        file.active_model_id = Some(model_id.to_string());
    } else {
        return Err("选择的模型还没有配置。".to_string());
    }
    write_model_accounts_file(&data_dir, &file)?;
    model_accounts_status(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_delete_model_provider(
    input: DeleteModelProviderInput,
) -> Result<ModelAccountsStatus, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut file = read_model_accounts_file(&data_dir)?;
    let provider_id = sanitize_provider_id(&input.provider_id);
    if provider_id.is_empty() {
        return Err("供应商 ID 不能为空。".to_string());
    }
    let before = file.providers.len();
    file.providers.retain(|provider| provider.id != provider_id);
    if file.providers.len() == before {
        return Err("供应商配置不存在。".to_string());
    }
    delete_provider_api_key(&provider_id)?;
    let configured = configured_models(&file);
    if file
        .active_model_id
        .as_deref()
        .is_some_and(|id| id.starts_with(&format!("{provider_id}::")))
    {
        file.active_model_id = configured.first().map(|model| model.id.clone());
    }
    write_model_accounts_file(&data_dir, &file)?;
    model_accounts_status(&data_dir)
}

#[tauri::command]
pub(crate) async fn wridian_test_model_provider(
    input: TestModelProviderInput,
) -> Result<TestModelProviderResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let file = read_model_accounts_file(&data_dir)?;
    let provider = file
        .providers
        .iter()
        .find(|provider| provider.id == input.provider_id)
        .ok_or_else(|| "供应商配置不存在。".to_string())?;
    let model = provider
        .models
        .first()
        .ok_or_else(|| "请先为该供应商填写至少一个模型。".to_string())?;
    let api_key = read_provider_secret(provider)?;
    if api_key.trim().is_empty() {
        return Err("请先保存该供应商的 API Key 或访问令牌。".to_string());
    }
    let settings = ActiveModelSettings {
        provider_id: provider.id.clone(),
        provider_name: provider.provider_name.clone(),
        protocol: provider.protocol.clone(),
        auth_style: provider.auth_style.clone(),
        base_url: provider.base_url.clone(),
        api_key,
        model: resolve_provider_model(provider, model),
        model_id: model_config_id(&provider.id, model),
        extra_env: provider.extra_env.clone(),
    };
    test_model_chat(&settings).await
}

#[tauri::command]
pub(crate) async fn wridian_test_model_provider_config(
    input: TestModelProviderConfigInput,
) -> Result<TestModelProviderResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let file = read_model_accounts_file(&data_dir)?;
    let provider_id = sanitize_provider_id(&input.provider_id);
    let existing = file
        .providers
        .iter()
        .find(|provider| provider.id == provider_id);
    let protocol = normalize_protocol(&input.protocol)?;
    let auth_style = normalize_auth_style(&input.auth_style);
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    validate_base_url(&base_url)?;
    let model = normalize_models(input.models)
        .into_iter()
        .next()
        .ok_or_else(|| "至少需要配置一个模型。".to_string())?;
    let mut api_key = input.api_key.trim().to_string();
    if api_key.is_empty() {
        api_key = existing
            .map(read_provider_secret)
            .transpose()?
            .unwrap_or_default();
    }
    if api_key.trim().is_empty() {
        return Err("请先填写或保存该服务的 API Key / Token。".to_string());
    }
    let provider_name = input.provider_name.trim();
    let provider_for_model_resolution = StoredModelProviderFile {
        id: provider_id.clone(),
        preset_key: None,
        provider_name: provider_name.to_string(),
        provider_type: None,
        protocol: protocol.clone(),
        auth_style: auth_style.clone(),
        base_url: base_url.clone(),
        models: vec![model.clone()],
        extra_env: input.extra_env.clone(),
        key_stored: false,
        api_key: None,
    };
    let resolved_model = resolve_provider_model(&provider_for_model_resolution, &model);
    test_model_chat(&ActiveModelSettings {
        provider_id,
        provider_name: if provider_name.is_empty() {
            "未命名服务".to_string()
        } else {
            provider_name.to_string()
        },
        protocol,
        auth_style,
        base_url,
        api_key,
        model: resolved_model,
        model_id: model,
        extra_env: input.extra_env,
    })
    .await
}

pub(crate) fn is_anthropic_compatible_parse_error(error: &str) -> bool {
    error == "Anthropic 响应中没有可用文本。"
        || error.starts_with("Anthropic 响应 JSON 解析失败：")
}

pub(crate) fn apply_anthropic_auth_headers(
    request: reqwest::RequestBuilder,
    settings: &ActiveModelSettings,
) -> reqwest::RequestBuilder {
    if settings.auth_style == "oauth_external" {
        request
            .bearer_auth(&settings.api_key)
            .header("anthropic-beta", "claude-code-20250219,oauth-2025-04-20")
            .header("user-agent", "claude-cli/2.1.74 (external, cli)")
            .header("x-app", "cli")
    } else if uses_anthropic_api_key_header(settings) {
        request.header("api-key", &settings.api_key)
    } else if settings.auth_style == "auth_token" {
        request.bearer_auth(&settings.api_key)
    } else {
        request.header("x-api-key", &settings.api_key)
    }
}

fn uses_anthropic_api_key_header(settings: &ActiveModelSettings) -> bool {
    settings.provider_id.contains("xiaomi-mimo")
        || settings
            .base_url
            .to_ascii_lowercase()
            .contains("xiaomimimo.com")
}

#[tauri::command]
pub(crate) fn wridian_anthropic_oauth_start() -> Result<AnthropicOauthStartResponse, String> {
    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    let session_id = random_urlsafe(24);
    store_provider_api_key(&anthropic_oauth_session_key(&session_id), &verifier)?;
    let auth_url = anthropic_oauth_auth_url(&verifier, &challenge);
    open_browser_url(&auth_url)?;
    Ok(AnthropicOauthStartResponse {
        session_id,
        auth_url,
    })
}

#[tauri::command]
pub(crate) async fn wridian_anthropic_oauth_complete(
    input: AnthropicOauthCompleteInput,
) -> Result<ProviderOauthResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let session_key = anthropic_oauth_session_key(&input.session_id);
    let verifier = read_provider_api_key(&session_key)?;
    if verifier.trim().is_empty() {
        return Err("Anthropic OAuth 会话不存在或已过期，请重新登录。".to_string());
    }
    let code_input = input.code.trim();
    let (code, state) = code_input
        .split_once('#')
        .map(|(code, state)| (code.trim(), state.trim()))
        .unwrap_or((code_input, verifier.as_str()));
    if code.is_empty() {
        return Err("Anthropic OAuth code 不能为空。".to_string());
    }
    let token_response = exchange_anthropic_oauth_code(code, state, &verifier).await?;
    let access_token = token_response
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let refresh_token = token_response
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Err("Anthropic OAuth token 响应缺少 access_token 或 refresh_token。".to_string());
    }
    let expires_in = token_response
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    let credentials = StoredAnthropicOauthCredentials {
        client_id: ANTHROPIC_OAUTH_CLIENT_ID.to_string(),
        refresh_token,
        access_token,
        expires_at: unix_timestamp() + expires_in,
    };
    store_anthropic_oauth_credentials(&credentials)?;
    let _ = delete_provider_api_key(&session_key);
    upsert_oauth_provider(
        &data_dir,
        StoredModelProviderFile {
            id: ANTHROPIC_OAUTH_PROVIDER_ID.to_string(),
            preset_key: Some(ANTHROPIC_OAUTH_PROVIDER_ID.to_string()),
            provider_name: "Anthropic".to_string(),
            provider_type: Some(ANTHROPIC_OAUTH_PROVIDER_ID.to_string()),
            protocol: "anthropic".to_string(),
            auth_style: "oauth_external".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            models: vec![
                "sonnet".to_string(),
                "opus".to_string(),
                "haiku".to_string(),
            ],
            extra_env: std::collections::BTreeMap::new(),
            key_stored: true,
            api_key: None,
        },
    )?;
    Ok(ProviderOauthResponse {
        email: None,
        status: model_accounts_status(&data_dir)?,
    })
}

#[tauri::command]
pub(crate) async fn wridian_openai_oauth_login() -> Result<ProviderOauthResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    let state = random_urlsafe(24);
    let listener = TcpListener::bind((OPENAI_OAUTH_REDIRECT_BIND_HOST, 1455))
        .map_err(|error| format!("OpenAI OAuth 回调端口监听失败：{error}"))?;
    let auth_url = openai_oauth_auth_url(OPENAI_OAUTH_REDIRECT_URI, &state, &challenge);
    open_browser_url(&auth_url)?;
    let captured = tauri::async_runtime::spawn_blocking(move || {
        capture_oauth_code(listener, OPENAI_OAUTH_CALLBACK_PATH, &state, "OpenAI")
    })
    .await
    .map_err(|error| format!("OpenAI OAuth 回调任务失败：{error}"))??;
    let token_response =
        exchange_openai_oauth_code(&captured, OPENAI_OAUTH_REDIRECT_URI, &verifier).await?;
    let access_token = token_response
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let refresh_token = token_response
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Err("OpenAI OAuth token 响应缺少 access_token 或 refresh_token。".to_string());
    }
    let id_token = token_response
        .get("id_token")
        .and_then(Value::as_str)
        .map(str::to_string);
    let claims = id_token
        .as_deref()
        .and_then(parse_jwt_claims)
        .unwrap_or(Value::Null);
    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_string);
    let auth_claims = claims.get("https://api.openai.com/auth");
    let account_id = auth_claims
        .and_then(|value| value.get("chatgpt_account_id"))
        .or_else(|| claims.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            claims
                .get("organizations")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let plan = auth_claims
        .and_then(|value| value.get("chatgpt_plan_type"))
        .or_else(|| claims.get("chatgpt_plan_type"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let expires_in = token_response
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    let credentials = StoredOpenAiOauthCredentials {
        client_id: OPENAI_OAUTH_CLIENT_ID.to_string(),
        refresh_token,
        access_token,
        id_token,
        expires_at: unix_timestamp() + expires_in,
        email: email.clone(),
        account_id,
        plan,
    };
    store_openai_oauth_credentials(&credentials)?;
    upsert_oauth_provider(
        &data_dir,
        StoredModelProviderFile {
            id: OPENAI_OAUTH_PROVIDER_ID.to_string(),
            preset_key: Some(OPENAI_OAUTH_PROVIDER_ID.to_string()),
            provider_name: "OpenAI".to_string(),
            provider_type: Some(OPENAI_OAUTH_PROVIDER_ID.to_string()),
            protocol: "openai-compatible".to_string(),
            auth_style: "oauth_external".to_string(),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            models: vec![
                "gpt-5.5".to_string(),
                "gpt-5.4".to_string(),
                "gpt-5.4-mini".to_string(),
                "gpt-5.3-codex".to_string(),
                "gpt-5.3-codex-spark".to_string(),
            ],
            extra_env: std::collections::BTreeMap::new(),
            key_stored: true,
            api_key: None,
        },
    )?;
    Ok(ProviderOauthResponse {
        email,
        status: model_accounts_status(&data_dir)?,
    })
}

#[tauri::command]
pub(crate) async fn wridian_google_gemini_oauth_login() -> Result<GoogleGeminiOauthResponse, String>
{
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let oauth_client = google_oauth_client_config()?;
    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    let state = random_urlsafe(24);
    let listener = TcpListener::bind((GOOGLE_OAUTH_REDIRECT_HOST, 8085))
        .or_else(|_| TcpListener::bind((GOOGLE_OAUTH_REDIRECT_HOST, 0)))
        .map_err(|error| format!("Gemini OAuth 回调端口监听失败：{error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Gemini OAuth 回调端口读取失败：{error}"))?
        .port();
    let redirect_uri = format!(
        "http://{}:{}{}",
        GOOGLE_OAUTH_REDIRECT_HOST, port, GOOGLE_OAUTH_CALLBACK_PATH
    );
    let auth_url =
        google_oauth_auth_url(&oauth_client.client_id, &redirect_uri, &state, &challenge);
    open_browser_url(&auth_url)?;
    let captured =
        tauri::async_runtime::spawn_blocking(move || capture_google_oauth_code(listener, &state))
            .await
            .map_err(|error| format!("Gemini OAuth 回调任务失败：{error}"))??;
    let token_response = exchange_google_oauth_code(
        &oauth_client.client_id,
        &oauth_client.client_secret,
        &captured,
        &redirect_uri,
        &verifier,
    )
    .await?;
    let access_token = token_response
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let refresh_token = token_response
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Err("Google OAuth token 响应缺少 access_token 或 refresh_token。".to_string());
    }
    let expires_in = token_response
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    let email = fetch_google_oauth_email(&access_token).await.ok();
    let credentials = StoredGoogleOauthCredentials {
        client_id: oauth_client.client_id,
        client_secret: oauth_client.client_secret,
        refresh_token,
        access_token,
        expires_at: unix_timestamp() + expires_in,
        email: email.clone(),
    };
    store_google_oauth_credentials(&credentials)?;
    upsert_google_gemini_oauth_provider(&data_dir)?;
    Ok(GoogleGeminiOauthResponse {
        email,
        status: model_accounts_status(&data_dir)?,
    })
}

pub(crate) fn read_active_model_settings(
    data_dir: &Path,
    requested_model_id: Option<&str>,
) -> Result<Option<ActiveModelSettings>, String> {
    let file = read_model_accounts_file(data_dir)?;
    let configured = configured_models(&file);
    if configured.is_empty() {
        return Ok(None);
    }
    let selected_id = requested_model_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .or(file.active_model_id.as_deref());
    let selected = selected_id
        .and_then(|id| configured.iter().find(|model| model.id == id))
        .unwrap_or(&configured[0]);
    let provider = file
        .providers
        .iter()
        .find(|provider| provider.id == selected.provider_id)
        .ok_or_else(|| "模型供应商配置损坏。".to_string())?;
    let api_key = read_provider_secret(provider)?;
    if api_key.trim().is_empty() {
        return Err(format!(
            "{} 还没有保存 API Key 或访问令牌。",
            provider.provider_name
        ));
    }
    Ok(Some(ActiveModelSettings {
        provider_id: provider.id.clone(),
        provider_name: provider.provider_name.clone(),
        protocol: provider.protocol.clone(),
        auth_style: provider.auth_style.clone(),
        base_url: provider.base_url.clone(),
        api_key,
        model: resolve_provider_model(provider, &selected.model),
        model_id: selected.id.clone(),
        extra_env: provider.extra_env.clone(),
    }))
}

fn normalize_model_provider(
    input: SaveModelProviderInput,
    existing: Option<StoredModelProviderFile>,
) -> Result<StoredModelProviderFile, String> {
    let id = sanitize_provider_id(&input.provider_id);
    if id.is_empty() {
        return Err("供应商 ID 不能为空。".to_string());
    }
    let provider_name = input.provider_name.trim().to_string();
    if provider_name.is_empty() {
        return Err("供应商名称不能为空。".to_string());
    }
    let protocol = normalize_protocol(&input.protocol)?;
    let auth_style = normalize_auth_style(&input.auth_style);
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    validate_base_url(&base_url)?;
    let models = normalize_models(input.models);
    if models.is_empty() {
        return Err("至少需要配置一个模型。".to_string());
    }
    let mut api_key = input.api_key.trim().to_string();
    if api_key.is_empty() {
        api_key = existing
            .as_ref()
            .and_then(|provider| read_provider_api_key(&provider.id).ok())
            .unwrap_or_default();
    }
    let key_stored = !api_key.is_empty();
    if key_stored {
        store_provider_api_key(&id, &api_key)?;
    }
    Ok(StoredModelProviderFile {
        id,
        preset_key: Some(clean_identifier(&input.preset_key)).filter(|value| !value.is_empty()),
        provider_name,
        provider_type: Some(clean_identifier(&input.provider_type))
            .filter(|value| !value.is_empty()),
        protocol,
        auth_style,
        base_url,
        models,
        extra_env: input.extra_env,
        key_stored,
        api_key: None,
    })
}

fn model_accounts_status(data_dir: &Path) -> Result<ModelAccountsStatus, String> {
    let file = read_model_accounts_file(data_dir)?;
    let configured_models = configured_models(&file);
    let active_model_id = file
        .active_model_id
        .filter(|id| configured_models.iter().any(|model| model.id == *id))
        .or_else(|| configured_models.first().map(|model| model.id.clone()));
    let active_model_label = active_model_id
        .as_deref()
        .and_then(|id| configured_models.iter().find(|model| model.id == id))
        .map(|model| model.label.clone());
    let providers = file
        .providers
        .iter()
        .map(provider_status)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ModelAccountsStatus {
        active_model_id,
        active_model_label,
        configured_models,
        providers,
    })
}

fn provider_status(provider: &StoredModelProviderFile) -> Result<ModelProviderStatus, String> {
    let api_key = read_provider_secret(provider)?;
    Ok(ModelProviderStatus {
        id: provider.id.clone(),
        preset_key: provider.preset_key.clone(),
        provider_name: provider.provider_name.clone(),
        provider_type: provider.provider_type.clone(),
        protocol: provider.protocol.clone(),
        auth_style: normalize_auth_style(&provider.auth_style),
        configured: !api_key.trim().is_empty() && !provider.models.is_empty(),
        base_url: Some(provider.base_url.clone()),
        models: provider.models.clone(),
        masked_key: if api_key.trim().is_empty() {
            None
        } else {
            Some(mask_api_key(&api_key))
        },
        extra_env: provider.extra_env.clone(),
    })
}

fn configured_models(file: &StoredModelAccountsFile) -> Vec<ConfiguredModelStatus> {
    file.providers
        .iter()
        .filter(|provider| {
            provider.key_stored
                && !provider.models.is_empty()
                && ensure_supported_protocol(&provider.protocol).is_ok()
        })
        .flat_map(|provider| {
            provider.models.iter().map(|model| ConfiguredModelStatus {
                id: model_config_id(&provider.id, model),
                label: format!("{} / {}", provider.provider_name, model),
                provider_id: provider.id.clone(),
                provider_name: provider.provider_name.clone(),
                protocol: provider.protocol.clone(),
                model: model.clone(),
            })
        })
        .collect()
}

fn read_model_accounts_file(data_dir: &Path) -> Result<StoredModelAccountsFile, String> {
    let path = model_accounts_path(data_dir);
    if !path.exists() {
        return Ok(empty_accounts_file());
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("模型账户配置读取失败：{error}"))?;
    let value: Value =
        serde_json::from_str(&content).map_err(|error| format!("模型账户配置格式损坏：{error}"))?;
    if value.get("providers").is_some() {
        let mut file: StoredModelAccountsFile = serde_json::from_value(value)
            .map_err(|error| format!("模型账户配置格式损坏：{error}"))?;
        for provider in &mut file.providers {
            if let Some(legacy_key) = provider
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|key| !key.is_empty())
            {
                store_provider_api_key(&provider.id, legacy_key)?;
                provider.key_stored = true;
                provider.api_key = None;
            }
        }
        return Ok(file);
    }
    migrate_legacy_custom_api(data_dir, &value)
}

fn migrate_legacy_custom_api(
    data_dir: &Path,
    value: &Value,
) -> Result<StoredModelAccountsFile, String> {
    let Some(custom_api) = value.get("customApi") else {
        return Ok(empty_accounts_file());
    };
    let stored: StoredCustomApiSettingsFile = serde_json::from_value(custom_api.clone())
        .map_err(|error| format!("自定义 API 配置格式损坏：{error}"))?;
    let api_key = if let Some(legacy_key) = stored
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        legacy_key.to_string()
    } else if stored.key_stored {
        read_legacy_custom_api_key()?
    } else {
        String::new()
    };
    if !api_key.trim().is_empty() {
        store_provider_api_key(DEFAULT_CUSTOM_PROVIDER_ID, &api_key)?;
    }
    let provider = StoredModelProviderFile {
        id: DEFAULT_CUSTOM_PROVIDER_ID.to_string(),
        preset_key: Some(DEFAULT_CUSTOM_PROVIDER_ID.to_string()),
        provider_name: "自定义 OpenAI 兼容".to_string(),
        provider_type: Some(DEFAULT_CUSTOM_PROVIDER_ID.to_string()),
        protocol: "openai-compatible".to_string(),
        auth_style: "api_key".to_string(),
        base_url: stored.base_url.trim().trim_end_matches('/').to_string(),
        models: normalize_models(vec![stored.model]),
        extra_env: std::collections::BTreeMap::new(),
        key_stored: !api_key.trim().is_empty(),
        api_key: None,
    };
    let active_model_id = provider
        .models
        .first()
        .map(|model| model_config_id(&provider.id, model));
    let migrated = StoredModelAccountsFile {
        schema_version: 2,
        active_model_id,
        providers: vec![provider],
    };
    write_model_accounts_file(data_dir, &migrated)?;
    Ok(migrated)
}

fn write_model_accounts_file(
    data_dir: &Path,
    file: &StoredModelAccountsFile,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(file).map_err(|error| error.to_string())?;
    fs::write(model_accounts_path(data_dir), content)
        .map_err(|error| format!("模型账户配置写入失败：{error}"))
}

fn empty_accounts_file() -> StoredModelAccountsFile {
    StoredModelAccountsFile {
        schema_version: 2,
        active_model_id: None,
        providers: Vec::new(),
    }
}

fn keyring_entry(user: &str) -> Result<Entry, String> {
    set_default_store(Store::new().map_err(|error| format!("系统凭据存储初始化失败：{error}"))?);
    Entry::new(KEYRING_SERVICE, user).map_err(|error| format!("系统凭据项创建失败：{error}"))
}

fn provider_keyring_user(provider_id: &str) -> String {
    format!("provider:{provider_id}")
}

fn store_provider_api_key(provider_id: &str, api_key: &str) -> Result<(), String> {
    keyring_entry(&provider_keyring_user(provider_id))?
        .set_password(api_key)
        .map_err(|error| format!("API Key 写入系统凭据失败：{error}"))
}

fn read_provider_api_key(provider_id: &str) -> Result<String, String> {
    match keyring_entry(&provider_keyring_user(provider_id))?.get_password() {
        Ok(api_key) => Ok(api_key),
        Err(KeyringError::NoEntry) => Ok(String::new()),
        Err(error) => Err(format!("API Key 读取系统凭据失败：{error}")),
    }
}

fn delete_provider_api_key(provider_id: &str) -> Result<(), String> {
    match keyring_entry(&provider_keyring_user(provider_id))?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("API Key 删除失败：{error}")),
    }
}

fn read_provider_secret(provider: &StoredModelProviderFile) -> Result<String, String> {
    if provider.id == ANTHROPIC_OAUTH_PROVIDER_ID
        && normalize_auth_style(&provider.auth_style) == "oauth_external"
    {
        return get_valid_anthropic_oauth_access_token();
    }
    if provider.id == OPENAI_OAUTH_PROVIDER_ID
        && normalize_auth_style(&provider.auth_style) == "oauth_external"
    {
        return get_valid_openai_oauth_access_token();
    }
    if provider.id == GOOGLE_GEMINI_OAUTH_PROVIDER_ID
        && normalize_auth_style(&provider.auth_style) == "oauth_external"
    {
        return get_valid_google_oauth_access_token();
    }
    read_provider_api_key(&provider.id)
}

pub(crate) fn is_openai_oauth_settings(settings: &ActiveModelSettings) -> bool {
    settings.provider_id == OPENAI_OAUTH_PROVIDER_ID
        && normalize_auth_style(&settings.auth_style) == "oauth_external"
}

pub(crate) fn openai_oauth_account_id() -> Result<Option<String>, String> {
    Ok(read_openai_oauth_credentials()?.and_then(|credentials| credentials.account_id))
}

fn store_anthropic_oauth_credentials(
    credentials: &StoredAnthropicOauthCredentials,
) -> Result<(), String> {
    let content = serde_json::to_string(credentials).map_err(|error| error.to_string())?;
    store_provider_api_key(ANTHROPIC_OAUTH_PROVIDER_ID, &content)
}

fn read_anthropic_oauth_credentials() -> Result<Option<StoredAnthropicOauthCredentials>, String> {
    let content = read_provider_api_key(ANTHROPIC_OAUTH_PROVIDER_ID)?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Anthropic OAuth 凭据格式损坏：{error}"))
}

fn get_valid_anthropic_oauth_access_token() -> Result<String, String> {
    let Some(mut credentials) = read_anthropic_oauth_credentials()? else {
        return Ok(String::new());
    };
    if credentials.expires_at > unix_timestamp() + ANTHROPIC_OAUTH_REFRESH_SKEW_SECONDS {
        return Ok(credentials.access_token);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Anthropic OAuth 刷新客户端创建失败：{error}"))?;
    let response = client
        .post(ANTHROPIC_OAUTH_REFRESH_ENDPOINT)
        .form(&[
            ("client_id", credentials.client_id.as_str()),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .header("User-Agent", "claude-cli/2.1.74 (external, cli)")
        .send()
        .map_err(|error| format!("Anthropic OAuth token 刷新失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("Anthropic OAuth token 刷新响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Anthropic OAuth token 刷新失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Anthropic OAuth token 刷新 JSON 解析失败：{error}"))?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() {
        return Err("Anthropic OAuth token 刷新响应缺少 access_token。".to_string());
    }
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    credentials.access_token = access_token.clone();
    credentials.expires_at = unix_timestamp() + expires_in;
    if let Some(refresh_token) = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        credentials.refresh_token = refresh_token.to_string();
    }
    store_anthropic_oauth_credentials(&credentials)?;
    Ok(access_token)
}

fn store_openai_oauth_credentials(
    credentials: &StoredOpenAiOauthCredentials,
) -> Result<(), String> {
    let content = serde_json::to_string(credentials).map_err(|error| error.to_string())?;
    store_provider_api_key(OPENAI_OAUTH_PROVIDER_ID, &content)
}

fn read_openai_oauth_credentials() -> Result<Option<StoredOpenAiOauthCredentials>, String> {
    let content = read_provider_api_key(OPENAI_OAUTH_PROVIDER_ID)?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("OpenAI OAuth 凭据格式损坏：{error}"))
}

fn get_valid_openai_oauth_access_token() -> Result<String, String> {
    let Some(mut credentials) = read_openai_oauth_credentials()? else {
        return Ok(String::new());
    };
    if credentials.expires_at > unix_timestamp() + OPENAI_OAUTH_REFRESH_SKEW_SECONDS {
        return Ok(credentials.access_token);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("OpenAI OAuth 刷新客户端创建失败：{error}"))?;
    let response = client
        .post(OPENAI_OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", credentials.client_id.as_str()),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .map_err(|error| format!("OpenAI OAuth token 刷新失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("OpenAI OAuth token 刷新响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "OpenAI OAuth token 刷新失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("OpenAI OAuth token 刷新 JSON 解析失败：{error}"))?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() {
        return Err("OpenAI OAuth token 刷新响应缺少 access_token。".to_string());
    }
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    credentials.access_token = access_token.clone();
    credentials.expires_at = unix_timestamp() + expires_in;
    if let Some(refresh_token) = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        credentials.refresh_token = refresh_token.to_string();
    }
    if let Some(id_token) = value
        .get("id_token")
        .and_then(Value::as_str)
        .map(str::to_string)
    {
        credentials.id_token = Some(id_token);
    }
    store_openai_oauth_credentials(&credentials)?;
    Ok(access_token)
}

fn store_google_oauth_credentials(
    credentials: &StoredGoogleOauthCredentials,
) -> Result<(), String> {
    let content = serde_json::to_string(credentials).map_err(|error| error.to_string())?;
    store_provider_api_key(GOOGLE_GEMINI_OAUTH_PROVIDER_ID, &content)
}

fn read_google_oauth_credentials() -> Result<Option<StoredGoogleOauthCredentials>, String> {
    let content = read_provider_api_key(GOOGLE_GEMINI_OAUTH_PROVIDER_ID)?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Gemini OAuth 凭据格式损坏：{error}"))
}

fn get_valid_google_oauth_access_token() -> Result<String, String> {
    let Some(mut credentials) = read_google_oauth_credentials()? else {
        return Ok(String::new());
    };
    if credentials.expires_at > unix_timestamp() + GOOGLE_OAUTH_REFRESH_SKEW_SECONDS {
        return Ok(credentials.access_token);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Gemini OAuth 刷新客户端创建失败：{error}"))?;
    let response = client
        .post(GOOGLE_OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("refresh_token", credentials.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .map_err(|error| format!("Gemini OAuth token 刷新失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("Gemini OAuth token 刷新响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Gemini OAuth token 刷新失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Gemini OAuth token 刷新 JSON 解析失败：{error}"))?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if access_token.is_empty() {
        return Err("Gemini OAuth token 刷新响应缺少 access_token。".to_string());
    }
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    credentials.access_token = access_token.clone();
    credentials.expires_at = unix_timestamp() + expires_in;
    if let Some(refresh_token) = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        credentials.refresh_token = refresh_token.to_string();
    }
    store_google_oauth_credentials(&credentials)?;
    Ok(access_token)
}

fn read_legacy_custom_api_key() -> Result<String, String> {
    match keyring_entry(LEGACY_CUSTOM_API_KEYRING_USER)?.get_password() {
        Ok(api_key) => Ok(api_key),
        Err(KeyringError::NoEntry) => Ok(String::new()),
        Err(error) => Err(format!("API Key 读取系统凭据失败：{error}")),
    }
}

fn upsert_oauth_provider(data_dir: &Path, provider: StoredModelProviderFile) -> Result<(), String> {
    let mut file = read_model_accounts_file(data_dir)?;
    if let Some(position) = file
        .providers
        .iter()
        .position(|item| item.id == provider.id)
    {
        file.providers[position] = provider.clone();
    } else {
        file.providers.push(provider.clone());
    }
    if file.active_model_id.is_none() {
        file.active_model_id = provider
            .models
            .first()
            .map(|model| model_config_id(&provider.id, model));
    }
    write_model_accounts_file(data_dir, &file)
}

fn upsert_google_gemini_oauth_provider(data_dir: &Path) -> Result<(), String> {
    let provider = StoredModelProviderFile {
        id: GOOGLE_GEMINI_OAUTH_PROVIDER_ID.to_string(),
        preset_key: Some(GOOGLE_GEMINI_OAUTH_PROVIDER_ID.to_string()),
        provider_name: "Gemini".to_string(),
        provider_type: Some(GOOGLE_GEMINI_OAUTH_PROVIDER_ID.to_string()),
        protocol: "google".to_string(),
        auth_style: "oauth_external".to_string(),
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        models: vec!["gemini-2.5-pro".to_string(), "gemini-2.5-flash".to_string()],
        extra_env: std::collections::BTreeMap::new(),
        key_stored: true,
        api_key: None,
    };
    upsert_oauth_provider(data_dir, provider)
}

fn random_urlsafe(bytes: usize) -> String {
    let mut data = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut data);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn anthropic_oauth_session_key(session_id: &str) -> String {
    format!(
        "oauth-session:anthropic:{}",
        sanitize_provider_id(session_id)
    )
}

fn anthropic_oauth_auth_url(verifier: &str, challenge: &str) -> String {
    let params = [
        ("code", "true"),
        ("client_id", ANTHROPIC_OAUTH_CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", ANTHROPIC_OAUTH_REDIRECT_URI),
        ("scope", ANTHROPIC_OAUTH_SCOPES),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", verifier),
    ];
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{ANTHROPIC_OAUTH_AUTH_ENDPOINT}?{query}")
}

fn openai_oauth_auth_url(redirect_uri: &str, state: &str, challenge: &str) -> String {
    let params = [
        ("client_id", OPENAI_OAUTH_CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", redirect_uri),
        ("scope", "openid profile email offline_access"),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("codex_cli_simplified_flow", "true"),
        ("id_token_add_organizations", "true"),
    ];
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{OPENAI_OAUTH_AUTH_ENDPOINT}?{query}")
}

fn google_oauth_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> String {
    let params = [
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", GOOGLE_OAUTH_SCOPES),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("state", state),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
    ];
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{GOOGLE_OAUTH_AUTH_ENDPOINT}?{query}")
}

fn google_oauth_client_config() -> Result<GoogleOauthClientConfig, String> {
    let client_id = std::env::var(GOOGLE_OAUTH_CLIENT_ID_ENV)
        .unwrap_or_default()
        .trim()
        .to_string();
    let client_secret = std::env::var(GOOGLE_OAUTH_CLIENT_SECRET_ENV)
        .unwrap_or_default()
        .trim()
        .to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err(format!(
            "Google Gemini OAuth 需要先配置环境变量 {GOOGLE_OAUTH_CLIENT_ID_ENV} 和 {GOOGLE_OAUTH_CLIENT_SECRET_ENV}。"
        ));
    }
    Ok(GoogleOauthClientConfig {
        client_id,
        client_secret,
    })
}

fn open_browser_url(url: &str) -> Result<(), String> {
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map_err(|error| format!("打开浏览器失败：{error}"))?;
    Ok(())
}

fn capture_oauth_code(
    listener: TcpListener,
    expected_path: &str,
    expected_state: &str,
    label: &str,
) -> Result<String, String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|error| format!("{label} OAuth 回调接收失败：{error}"))?;
    let mut buffer = [0u8; 4096];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("{label} OAuth 回调读取失败：{error}"))?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or("");
    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("{label} OAuth 回调请求格式无效。"))?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != expected_path {
        let _ = write_oauth_callback_page(
            &mut stream,
            404,
            &format!("Wridian {label} OAuth callback path mismatch."),
        );
        return Err(format!("{label} OAuth 回调路径不匹配。"));
    }
    let params = parse_query(query);
    if let Some(error) = params.get("error") {
        let _ = write_oauth_callback_page(&mut stream, 400, &format!("{label} OAuth denied."));
        return Err(format!("{label} OAuth 登录失败：{error}"));
    }
    if params.get("state").map(String::as_str) != Some(expected_state) {
        let _ = write_oauth_callback_page(
            &mut stream,
            400,
            &format!("Wridian {label} OAuth state mismatch."),
        );
        return Err(format!("{label} OAuth state 校验失败。"));
    }
    let code = params
        .get("code")
        .map(String::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} OAuth 回调缺少 code。"))?;
    let _ = write_oauth_callback_page(
        &mut stream,
        200,
        &format!("Wridian {label} OAuth login complete. You can return to Wridian."),
    );
    Ok(code)
}

fn capture_google_oauth_code(
    listener: TcpListener,
    expected_state: &str,
) -> Result<String, String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|error| format!("Gemini OAuth 回调接收失败：{error}"))?;
    let mut buffer = [0u8; 4096];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("Gemini OAuth 回调读取失败：{error}"))?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or("");
    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "Gemini OAuth 回调请求格式无效。".to_string())?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != GOOGLE_OAUTH_CALLBACK_PATH {
        let _ = write_oauth_callback_page(
            &mut stream,
            404,
            "Wridian Gemini OAuth callback path mismatch.",
        );
        return Err("Gemini OAuth 回调路径不匹配。".to_string());
    }
    let params = parse_query(query);
    if let Some(error) = params.get("error") {
        let _ = write_oauth_callback_page(&mut stream, 400, "Google OAuth denied.");
        return Err(format!("Google OAuth 登录失败：{error}"));
    }
    if params.get("state").map(String::as_str) != Some(expected_state) {
        let _ = write_oauth_callback_page(&mut stream, 400, "Wridian Gemini OAuth state mismatch.");
        return Err("Gemini OAuth state 校验失败。".to_string());
    }
    let code = params
        .get("code")
        .map(String::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Gemini OAuth 回调缺少 code。".to_string())?;
    let _ = write_oauth_callback_page(
        &mut stream,
        200,
        "Wridian Gemini OAuth login complete. You can return to Wridian.",
    );
    Ok(code)
}

fn parse_query(query: &str) -> std::collections::BTreeMap<String, String> {
    query
        .split('&')
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((
                urlencoding::decode(key).ok()?.into_owned(),
                urlencoding::decode(value).ok()?.into_owned(),
            ))
        })
        .collect()
}

fn write_oauth_callback_page(
    stream: &mut std::net::TcpStream,
    status: u16,
    message: &str,
) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Wridian Gemini OAuth</title><body style=\"font-family:system-ui;padding:32px\"><h1>{}</h1><p>{}</p></body>",
        if status == 200 { "登录完成" } else { "登录失败" },
        message
    );
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.as_bytes().len(),
        html
    )
}

async fn exchange_anthropic_oauth_code(
    code: &str,
    state: &str,
    verifier: &str,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Anthropic OAuth token 客户端创建失败：{error}"))?;
    let response = client
        .post(ANTHROPIC_OAUTH_TOKEN_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("User-Agent", "claude-cli/2.1.74 (external, cli)")
        .json(&json!({
            "grant_type": "authorization_code",
            "client_id": ANTHROPIC_OAUTH_CLIENT_ID,
            "code": code,
            "state": state,
            "redirect_uri": ANTHROPIC_OAUTH_REDIRECT_URI,
            "code_verifier": verifier,
        }))
        .send()
        .await
        .map_err(|error| format!("Anthropic OAuth token 交换失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Anthropic OAuth token 响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Anthropic OAuth token 交换失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    serde_json::from_str(&body)
        .map_err(|error| format!("Anthropic OAuth token JSON 解析失败：{error}"))
}

async fn exchange_openai_oauth_code(
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("OpenAI OAuth token 客户端创建失败：{error}"))?;
    let response = client
        .post(OPENAI_OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", OPENAI_OAUTH_CLIENT_ID),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|error| format!("OpenAI OAuth token 交换失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("OpenAI OAuth token 响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "OpenAI OAuth token 交换失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    serde_json::from_str(&body)
        .map_err(|error| format!("OpenAI OAuth token JSON 解析失败：{error}"))
}

async fn exchange_google_oauth_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Gemini OAuth token 客户端创建失败：{error}"))?;
    let response = client
        .post(GOOGLE_OAUTH_TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|error| format!("Gemini OAuth token 交换失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Gemini OAuth token 响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Gemini OAuth token 交换失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    serde_json::from_str(&body)
        .map_err(|error| format!("Gemini OAuth token JSON 解析失败：{error}"))
}

async fn fetch_google_oauth_email(access_token: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Google userinfo 客户端创建失败：{error}"))?;
    let response = client
        .get(GOOGLE_OAUTH_USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("Google userinfo 请求失败：{error}"))?;
    let value: Value = response
        .json()
        .await
        .map_err(|error| format!("Google userinfo 响应解析失败：{error}"))?;
    value
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Google userinfo 响应缺少 email。".to_string())
}

fn parse_jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn normalize_protocol(protocol: &str) -> Result<String, String> {
    match protocol.trim().to_ascii_lowercase().as_str() {
        "openai-compatible" => Ok("openai-compatible".to_string()),
        "anthropic" => Ok("anthropic".to_string()),
        "google" => Ok("google".to_string()),
        _ => Err("协议必须是 openai-compatible、anthropic 或 google。".to_string()),
    }
}

pub(crate) fn ensure_supported_protocol(protocol: &str) -> Result<(), String> {
    match protocol {
        "openai-compatible" | "anthropic" | "google" => Ok(()),
        _ => Err(format!(
            "不支持的模型协议：{protocol}。请重新保存模型服务。"
        )),
    }
}

fn normalize_auth_style(auth_style: &str) -> String {
    match auth_style.trim().to_ascii_lowercase().as_str() {
        "auth_token" | "auth-token" | "token" | "bearer" => "auth_token".to_string(),
        "oauth_external" | "oauth" | "oauth-external" => "oauth_external".to_string(),
        _ => "api_key".to_string(),
    }
}

fn clean_identifier(input: &str) -> String {
    input.trim().chars().filter(|ch| !ch.is_control()).collect()
}

fn normalize_models(models: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for model in models {
        let trimmed = model.trim();
        if trimmed.is_empty() || normalized.iter().any(|item: &String| item == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    normalized
}

fn sanitize_provider_id(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn model_config_id(provider_id: &str, model: &str) -> String {
    format!("{provider_id}::{}", model.trim())
}

fn resolve_provider_model(provider: &StoredModelProviderFile, model: &str) -> String {
    let model = model.trim();
    if model.eq_ignore_ascii_case("haiku") {
        if is_first_party_anthropic_provider(provider) {
            return "claude-haiku-4-5-20251001".to_string();
        }
        if let Some(mapped) = provider.extra_env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL") {
            return mapped.trim().to_string();
        }
    }
    if model.eq_ignore_ascii_case("sonnet") {
        if is_first_party_anthropic_provider(provider) {
            return "claude-sonnet-4-6".to_string();
        }
        if let Some(mapped) = provider.extra_env.get("ANTHROPIC_DEFAULT_SONNET_MODEL") {
            return mapped.trim().to_string();
        }
    }
    if model.eq_ignore_ascii_case("opus") {
        if is_first_party_anthropic_provider(provider) {
            return "claude-opus-4-7".to_string();
        }
        if let Some(mapped) = provider.extra_env.get("ANTHROPIC_DEFAULT_OPUS_MODEL") {
            return mapped.trim().to_string();
        }
    }
    model.to_string()
}

fn is_first_party_anthropic_provider(provider: &StoredModelProviderFile) -> bool {
    provider.id == ANTHROPIC_OAUTH_PROVIDER_ID
        || provider.preset_key.as_deref() == Some(ANTHROPIC_OAUTH_PROVIDER_ID)
        || (provider.protocol == "anthropic"
            && provider
                .base_url
                .trim()
                .trim_end_matches('/')
                .eq_ignore_ascii_case("https://api.anthropic.com"))
}

fn validate_base_url(base_url: &str) -> Result<(), String> {
    if base_url.contains('?') || base_url.contains('#') {
        return Err(
            "Base URL 不能包含查询参数或片段，请只填写服务根地址或完整 endpoint 路径。".to_string(),
        );
    }
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

pub(crate) fn anthropic_messages_url(base_url: &str) -> String {
    let cleaned = base_url.trim().trim_end_matches('/');
    if cleaned.ends_with("/v1/messages") || cleaned.ends_with("/messages") {
        return cleaned.to_string();
    }
    if cleaned.ends_with("/v1") {
        return format!("{cleaned}/messages");
    }
    format!("{cleaned}/v1/messages")
}

pub(crate) fn openai_chat_completions_url(base_url: &str) -> String {
    let cleaned = base_url.trim().trim_end_matches('/');
    if cleaned.ends_with("/chat/completions") {
        return cleaned.to_string();
    }
    if cleaned.ends_with("/v1") {
        return format!("{cleaned}/chat/completions");
    }
    format!("{cleaned}/v1/chat/completions")
}

pub(crate) fn response_body_summary(body: &str) -> String {
    let mut text = body
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    for tag in ["html", "head", "title", "body", "center", "h1"] {
        text = text.replace(&format!("<{tag}>"), " ");
        text = text.replace(&format!("</{tag}>"), " ");
    }
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(240).collect()
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

async fn test_model_chat(
    settings: &ActiveModelSettings,
) -> Result<TestModelProviderResponse, String> {
    ensure_supported_protocol(&settings.protocol)?;
    if is_openai_oauth_settings(settings) {
        return test_openai_oauth_chat(settings).await;
    }
    match settings.protocol.as_str() {
        "anthropic" => test_anthropic_chat(settings).await,
        "google" => test_gemini_chat(settings).await,
        "openai-compatible" => test_openai_compatible_chat(settings).await,
        _ => unreachable!("protocol checked before dispatch"),
    }
}

async fn test_openai_oauth_chat(
    settings: &ActiveModelSettings,
) -> Result<TestModelProviderResponse, String> {
    let url = format!("{}/responses", settings.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("OpenAI OAuth 客户端创建失败：{error}"))?;
    let mut request = client.post(url).bearer_auth(&settings.api_key);
    if let Some(account_id) = openai_oauth_account_id()? {
        request = request.header("chatgpt-account-id", account_id);
    }
    let response = request
        .json(&json!({
            "model": settings.model,
            "instructions": "Reply with OK.",
            "input": "Reply with OK.",
            "store": false
        }))
        .send()
        .await
        .map_err(|error| format!("OpenAI OAuth 连接失败：{error}"))?;
    read_text_test_response(response, "OpenAI OAuth").await
}

async fn test_openai_compatible_chat(
    settings: &ActiveModelSettings,
) -> Result<TestModelProviderResponse, String> {
    let url = openai_chat_completions_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("模型客户端创建失败：{error}"))?;
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
        .map_err(|error| format!("模型连接失败：{error}"))?;
    read_text_test_response(response, "OpenAI-compatible").await
}

async fn test_anthropic_chat(
    settings: &ActiveModelSettings,
) -> Result<TestModelProviderResponse, String> {
    let response = send_anthropic_test_request(settings, false).await?;
    match read_anthropic_test_response(response).await {
        Err(error) if is_anthropic_compatible_parse_error(&error) => {
            let response = send_anthropic_test_request(settings, true).await?;
            read_anthropic_test_response(response).await
        }
        result => result,
    }
}

async fn send_anthropic_test_request(
    settings: &ActiveModelSettings,
    stream: bool,
) -> Result<reqwest::Response, String> {
    let url = anthropic_messages_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Anthropic 客户端创建失败：{error}"))?;
    let request = apply_anthropic_auth_headers(
        client.post(url).header("anthropic-version", "2023-06-01"),
        settings,
    );
    let response = request
        .json(&json!({
            "model": settings.model,
            "messages": [
                { "role": "user", "content": [{ "type": "text", "text": "Reply with OK." }] }
            ],
            "max_tokens": 8,
            "temperature": 0,
            "stream": stream
        }))
        .send()
        .await
        .map_err(|error| format!("Anthropic 连接失败：{error}"))?;
    Ok(response)
}

async fn test_gemini_chat(
    settings: &ActiveModelSettings,
) -> Result<TestModelProviderResponse, String> {
    let url = gemini_generate_content_url(&settings.base_url, &settings.model);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Gemini 客户端创建失败：{error}"))?;
    let mut request = client.post(url);
    if settings.auth_style == "oauth_external" {
        request = request.bearer_auth(&settings.api_key);
    } else {
        request = request.header("x-goog-api-key", &settings.api_key);
    }
    let response = request
        .json(&json!({
            "contents": [
                { "role": "user", "parts": [{ "text": "Reply with OK." }] }
            ],
            "generationConfig": {
                "maxOutputTokens": 8,
                "temperature": 0
            }
        }))
        .send()
        .await
        .map_err(|error| format!("Gemini 连接失败：{error}"))?;
    read_gemini_test_response(response).await
}

pub(crate) fn gemini_generate_content_url(base_url: &str, model: &str) -> String {
    format!(
        "{}/models/{}:generateContent",
        base_url.trim().trim_end_matches('/'),
        model.trim()
    )
}

async fn read_text_test_response(
    response: reqwest::Response,
    label: &str,
) -> Result<TestModelProviderResponse, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("{label} 响应读取失败：{error}"))?;
    if status.is_success() {
        let content = read_model_response_text(&body)?;
        if content.trim().is_empty() {
            return Err(format!("{label} 返回了空文本，无法用于 Wridian 对话。"));
        }
        Ok(TestModelProviderResponse {
            ok: true,
            message: "连接成功，且响应格式可用于 Wridian 对话。".to_string(),
        })
    } else {
        Err(format!(
            "{label} 测试失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ))
    }
}

async fn read_anthropic_test_response(
    response: reqwest::Response,
) -> Result<TestModelProviderResponse, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Anthropic 响应读取失败：{error}"))?;
    if status.is_success() {
        if read_anthropic_response_text(&body)?.trim().is_empty() {
            return Err("Anthropic 返回了空文本，无法用于 Wridian 对话。".to_string());
        }
        Ok(TestModelProviderResponse {
            ok: true,
            message: "连接成功，且 Anthropic 响应格式可用于 Wridian 对话。".to_string(),
        })
    } else {
        Err(format!(
            "Anthropic 测试失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ))
    }
}

async fn read_gemini_test_response(
    response: reqwest::Response,
) -> Result<TestModelProviderResponse, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Gemini 响应读取失败：{error}"))?;
    if status.is_success() {
        if read_gemini_response_text(&body)?.trim().is_empty() {
            return Err("Gemini 返回了空文本，无法用于 Wridian 对话。".to_string());
        }
        Ok(TestModelProviderResponse {
            ok: true,
            message: "连接成功，且 Gemini 响应格式可用于 Wridian 对话。".to_string(),
        })
    } else {
        Err(format!(
            "Gemini 测试失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ))
    }
}

pub(crate) fn read_anthropic_response_text(body: &str) -> Result<String, String> {
    if let Some(text) = read_sse_response_text(body) {
        return Ok(text);
    }
    if let Some(text) = read_plain_text_response(body) {
        return Ok(text);
    }
    let value: Value = serde_json::from_str(body)
        .map_err(|error| format!("Anthropic 响应 JSON 解析失败：{error}"))?;
    if let Some(text) = read_anthropic_value_text(&value) {
        Ok(text)
    } else {
        read_model_response_text(body)
            .map_err(|_| "Anthropic 响应中没有可用文本。".to_string())
    }
}

fn read_plain_text_response(body: &str) -> Option<String> {
    let text = body.trim();
    if text.is_empty() || text.starts_with('{') || text.starts_with('[') {
        return None;
    }
    Some(text.to_string())
}

fn read_anthropic_value_text(value: &Value) -> Option<String> {
    let mut parts = Vec::new();
    collect_anthropic_text(value, &mut parts);
    let text = parts.join("\n");
    (!text.trim().is_empty()).then_some(text)
}

fn collect_anthropic_text(value: &Value, parts: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                parts.push(text.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_anthropic_text(item, parts);
            }
        }
        Value::Object(map) => {
            for key in [
                "text",
                "content",
                "message",
                "output_text",
                "completion",
                "response",
            ] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    if !text.trim().is_empty() {
                        parts.push(text.to_string());
                    }
                }
            }
            for key in [
                "content",
                "delta",
                "content_block",
                "message",
                "data",
                "output",
            ] {
                if let Some(nested) = map.get(key).filter(|value| !value.is_string()) {
                    collect_anthropic_text(nested, parts);
                }
            }
        }
        _ => {}
    }
}

fn read_sse_response_text(body: &str) -> Option<String> {
    if !body.lines().any(|line| line.trim_start().starts_with("data:")) {
        return None;
    }
    let mut parts = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(data) {
            if let Some(text) = read_sse_event_text(&value) {
                parts.push(text);
            }
        }
    }
    let text = parts.join("");
    (!text.trim().is_empty()).then_some(text)
}

fn read_sse_event_text(value: &Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(text) = value
        .get("delta")
        .and_then(|delta| {
            delta
                .get("text")
                .or_else(|| delta.get("content"))
                .or_else(|| delta.get("output_text"))
        })
        .and_then(Value::as_str)
    {
        parts.push(text.to_string());
    }
    if let Some(text) = value
        .get("content_block")
        .and_then(|block| block.get("text").or_else(|| block.get("content")))
        .and_then(Value::as_str)
    {
        parts.push(text.to_string());
    }
    if let Some(choices) = value.get("choices").and_then(Value::as_array) {
        for choice in choices {
            if let Some(text) = choice
                .get("delta")
                .and_then(|delta| delta.get("content").or_else(|| delta.get("text")))
                .and_then(Value::as_str)
            {
                parts.push(text.to_string());
            }
        }
    }
    let text = parts.join("");
    (!text.trim().is_empty()).then_some(text)
}

pub(crate) fn read_gemini_response_text(body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| format!("Gemini 响应 JSON 解析失败：{error}"))?;
    let text = value
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|candidate| {
            candidate
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        Err("Gemini 响应中没有可用文本。".to_string())
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_requires_https_except_localhost() {
        assert!(validate_base_url("https://api.example.com/v1").is_ok());
        assert!(validate_base_url("http://localhost:8080/v1").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(validate_base_url("http://[::1]:8080/v1").is_ok());
        assert!(validate_base_url("http://api.example.com/v1").is_err());
        assert!(validate_base_url("https://api.example.com/v1?key=secret").is_err());
        assert!(validate_base_url("https://api.example.com/v1#token").is_err());
    }

    #[test]
    fn protocol_normalization_rejects_legacy_aliases_and_unknown_values() {
        assert_eq!(
            normalize_protocol("openai-compatible").expect("openai-compatible"),
            "openai-compatible"
        );
        assert_eq!(
            normalize_protocol("anthropic").expect("anthropic"),
            "anthropic"
        );
        assert_eq!(normalize_protocol("google").expect("google"), "google");
        assert!(normalize_protocol("openai").is_err());
        assert!(normalize_protocol("gemini").is_err());
        assert!(normalize_protocol("unknown").is_err());
    }

    #[test]
    fn supported_protocol_check_rejects_runtime_fallback() {
        assert!(ensure_supported_protocol("openai-compatible").is_ok());
        assert!(ensure_supported_protocol("anthropic").is_ok());
        assert!(ensure_supported_protocol("google").is_ok());
        assert!(ensure_supported_protocol("openai").is_err());
        assert!(ensure_supported_protocol("custom").is_err());
    }

    #[test]
    fn anthropic_messages_url_matches_sdk_base_url_shape() {
        assert_eq!(
            anthropic_messages_url("https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://token-plan-cn.xiaomimimo.com/anthropic"),
            "https://token-plan-cn.xiaomimimo.com/anthropic/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://api.kimi.com/coding/v1"),
            "https://api.kimi.com/coding/v1/messages"
        );
        assert_eq!(
            anthropic_messages_url("https://proxy.example.com/v1/messages"),
            "https://proxy.example.com/v1/messages"
        );
    }

    #[test]
    fn openai_chat_completions_url_accepts_base_or_full_endpoint() {
        assert_eq!(
            openai_chat_completions_url("https://api.example.com"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            openai_chat_completions_url("https://api.example.com/v1"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            openai_chat_completions_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn gemini_generate_content_url_does_not_embed_api_key() {
        let url = gemini_generate_content_url(
            "https://generativelanguage.googleapis.com/v1beta/",
            "gemini-2.5-pro",
        );

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent"
        );
        assert!(!url.contains("?key="));
    }

    #[test]
    fn gemini_default_output_limit_matches_native_api_ceiling() {
        assert_eq!(GEMINI_DEFAULT_MAX_OUTPUT_TOKENS, 65535);
    }

    #[test]
    fn xiaomi_mimo_anthropic_uses_api_key_header_even_for_saved_auth_token() {
        let settings = ActiveModelSettings {
            provider_id: "xiaomi-mimo-token-plan".to_string(),
            provider_name: "Xiaomi MiMo Token Plan".to_string(),
            protocol: "anthropic".to_string(),
            auth_style: "auth_token".to_string(),
            base_url: "https://token-plan-cn.xiaomimimo.com/anthropic".to_string(),
            api_key: "secret".to_string(),
            model: "mimo-v2.5-pro".to_string(),
            model_id: "mimo-v2.5-pro".to_string(),
            extra_env: std::collections::BTreeMap::new(),
        };

        assert!(uses_anthropic_api_key_header(&settings));
    }

    #[test]
    fn response_body_summary_strips_simple_html_error() {
        let body = "<html><head><title>404 Not Found</title></head><body><center><h1>404 Not Found</h1></center></body></html>";
        assert_eq!(response_body_summary(body), "404 Not Found 404 Not Found");
    }

    #[test]
    fn model_provider_file_omits_plain_api_key() {
        let provider = StoredModelProviderFile {
            id: "deepseek".to_string(),
            preset_key: Some("deepseek".to_string()),
            provider_name: "DeepSeek".to_string(),
            provider_type: Some("deepseek".to_string()),
            protocol: "anthropic".to_string(),
            auth_style: "auth_token".to_string(),
            base_url: "https://api.deepseek.com/anthropic".to_string(),
            models: vec!["deepseek-chat".to_string()],
            extra_env: std::collections::BTreeMap::new(),
            key_stored: true,
            api_key: None,
        };
        let content = serde_json::to_string_pretty(&json!({
            "schemaVersion": 2,
            "providers": [provider]
        }))
        .expect("serialize settings");

        assert!(content.contains("\"keyStored\": true"));
        assert!(!content.contains("secret-key"));
    }

    #[test]
    fn provider_model_resolution_maps_first_party_anthropic_aliases() {
        let provider = StoredModelProviderFile {
            id: ANTHROPIC_OAUTH_PROVIDER_ID.to_string(),
            preset_key: Some(ANTHROPIC_OAUTH_PROVIDER_ID.to_string()),
            provider_name: "Anthropic".to_string(),
            provider_type: Some(ANTHROPIC_OAUTH_PROVIDER_ID.to_string()),
            protocol: "anthropic".to_string(),
            auth_style: "oauth_external".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            models: vec!["sonnet".to_string()],
            extra_env: std::collections::BTreeMap::new(),
            key_stored: true,
            api_key: None,
        };

        assert_eq!(
            resolve_provider_model(&provider, "sonnet"),
            "claude-sonnet-4-6"
        );
        assert_eq!(resolve_provider_model(&provider, "opus"), "claude-opus-4-7");
        assert_eq!(
            resolve_provider_model(&provider, "haiku"),
            "claude-haiku-4-5-20251001"
        );
    }

    #[test]
    fn provider_model_resolution_uses_catalog_env_role_mapping() {
        let mut extra_env = std::collections::BTreeMap::new();
        extra_env.insert(
            "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
            "glm-5-turbo".to_string(),
        );
        let provider = StoredModelProviderFile {
            id: "glm-cn".to_string(),
            preset_key: Some("glm-cn".to_string()),
            provider_name: "GLM".to_string(),
            provider_type: Some("glm-cn".to_string()),
            protocol: "anthropic".to_string(),
            auth_style: "auth_token".to_string(),
            base_url: "https://open.bigmodel.cn/api/anthropic".to_string(),
            models: vec!["sonnet".to_string()],
            extra_env,
            key_stored: true,
            api_key: None,
        };

        assert_eq!(resolve_provider_model(&provider, "sonnet"), "glm-5-turbo");
        assert_eq!(resolve_provider_model(&provider, "glm-5"), "glm-5");
    }

    #[test]
    fn anthropic_response_text_reads_content_parts() {
        let body = r#"{"content":[{"type":"text","text":"OK"}]}"#;
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_reads_content_string() {
        let body = r#"{"content":"OK"}"#;
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_falls_back_to_openai_compatible_shape() {
        let body = r#"{"choices":[{"message":{"content":"OK"}}]}"#;
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_reads_nested_data_message() {
        let body = r#"{"data":{"message":"OK"}}"#;
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_reads_anthropic_sse_delta() {
        let body = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"O\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"K\"}}\n\n";
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_reads_openai_style_sse_delta() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"O\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"K\"}}]}\n\ndata: [DONE]\n\n";
        assert_eq!(read_anthropic_response_text(body).expect("text"), "OK");
    }

    #[test]
    fn anthropic_response_text_reads_plain_text_body() {
        assert_eq!(read_anthropic_response_text("OK").expect("text"), "OK");
    }

    #[test]
    fn anthropic_compatible_parse_error_allows_json_parse_retry() {
        assert!(is_anthropic_compatible_parse_error(
            "Anthropic 响应 JSON 解析失败：expected value at line 1 column 1"
        ));
        assert!(is_anthropic_compatible_parse_error(
            "Anthropic 响应中没有可用文本。"
        ));
        assert!(!is_anthropic_compatible_parse_error(
            "Anthropic 测试失败：HTTP 401 unauthorized"
        ));
    }

    #[test]
    fn gemini_response_text_reads_candidate_parts() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"OK"}]}}]}"#;
        assert_eq!(read_gemini_response_text(body).expect("text"), "OK");
    }
}
