use crate::memory::{read_relevant_memory_snippets, write_memory_leaves, MemoryLeafDraft};
use crate::model_accounts::{
    anthropic_messages_url, apply_anthropic_auth_headers, ensure_supported_protocol,
    gemini_generate_content_url, is_anthropic_compatible_parse_error, is_openai_oauth_settings,
    openai_chat_completions_url, openai_oauth_account_id, read_active_model_settings,
    read_anthropic_response_text, read_gemini_response_text, response_body_summary,
    ActiveModelSettings, GEMINI_DEFAULT_MAX_OUTPUT_TOKENS,
};
use crate::projects::{active_project_model, read_active_project_context};
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{path::BaseDirectory, AppHandle, Manager};

const BUDGET_ACTIVE_CONTEXT_CHARS: usize = 1200;
const BUDGET_CURRENT_DRAFT_CHARS: usize = 7000;
const BUDGET_EXPLICIT_ITEM_CHARS: usize = 1400;
const BUDGET_EXPLICIT_TOTAL_CHARS: usize = 4200;
const BUDGET_MEMORY_TOTAL_CHARS: usize = 2200;
const BUDGET_PROJECT_CONTEXT_CHARS: usize = 2600;
const BUDGET_SELECTION_CHARS: usize = 3000;
const BUDGET_TOOL_ITEM_CHARS: usize = 1800;
const BUDGET_TOOL_TOTAL_CHARS: usize = 2600;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateInput {
    request_id: Option<String>,
    source_path: String,
    title: String,
    content: String,
    draft_kind: Option<String>,
    user_input: String,
    selected_text: Option<String>,
    selected_model_id: Option<String>,
    #[serde(default)]
    context_items: Vec<DialogueContextItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AbortCoCreateInput {
    request_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AbortCoCreateResponse {
    aborted: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DialogueContextItem {
    kind: String,
    label: String,
    value: String,
    source_path: Option<String>,
    relative_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateResponse {
    context_load_status: Vec<ContextLoadStatus>,
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories_used: Vec<String>,
    memories_written: Vec<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextLoadStatus {
    key: String,
    label: String,
    loaded: bool,
    item_count: usize,
    included_chars: usize,
    budget_chars: usize,
    truncated: bool,
    note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateEdit {
    target: String,
    replacement: String,
    rationale: Option<String>,
}

#[tauri::command]
pub(crate) async fn wridian_cocreate(
    app: AppHandle,
    input: CoCreateInput,
) -> Result<CoCreateResponse, String> {
    let request_id = input
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned);
    if let Some(id) = request_id.as_deref() {
        begin_cocreation_request(id);
    }
    let result = run_cocreation(app, input, request_id.as_deref()).await;
    if let Some(id) = request_id.as_deref() {
        finish_cocreation_request(id);
    }
    result
}

#[tauri::command]
pub(crate) fn wridian_abort_cocreate(
    input: AbortCoCreateInput,
) -> Result<AbortCoCreateResponse, String> {
    let request_id = input.request_id.trim();
    if request_id.is_empty() {
        return Ok(AbortCoCreateResponse { aborted: false });
    }
    cancel_cocreation_request(request_id);
    Ok(AbortCoCreateResponse { aborted: true })
}

async fn run_cocreation(
    app: AppHandle,
    mut input: CoCreateInput,
    request_id: Option<&str>,
) -> Result<CoCreateResponse, String> {
    let user_input = input.user_input.trim();
    if user_input.is_empty() {
        return Err("对话输入不能为空。".to_string());
    }
    check_cocreation_cancelled(request_id)?;

    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let skill_resource_root = app
        .path()
        .resolve("resources/skills", BaseDirectory::Resource)
        .ok();
    input.context_items = expand_context_items(
        &data_dir,
        skill_resource_root.as_deref(),
        &input.context_items,
    )?;
    check_cocreation_cancelled(request_id)?;
    let settings = read_active_model_settings(&data_dir, input.selected_model_id.as_deref())?
        .ok_or_else(|| "请先在模型设置里保存模型账户。".to_string())?;
    let memories_used =
        read_relevant_memory_snippets(&data_dir, &input.source_path, &input.title, 8)?;
    let active_context = read_active_context(&data_dir);
    let active_project_context = read_active_project_context(&data_dir)?;
    let project_model = active_project_model(&data_dir)?;
    let context_load_status = build_context_load_status(
        &input,
        &memories_used,
        &active_context,
        &active_project_context,
    );
    let model_output = await_cancellable(
        request_id,
        cocreate_with_model(
            &settings,
            project_model.as_deref(),
            &input,
            &memories_used,
            &active_context,
            &active_project_context,
        ),
    )
    .await?;
    check_cocreation_cancelled(request_id)?;

    let memories_written = write_memory_leaves(&data_dir, &model_output.memories)?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    Ok(CoCreateResponse {
        context_load_status,
        reply: model_output.reply,
        edits: model_output.edits,
        memories_used,
        memories_written,
    })
}

static ACTIVE_COCREATION_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static CANCELLED_COCREATION_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn active_cocreation_requests() -> &'static Mutex<HashSet<String>> {
    ACTIVE_COCREATION_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn cancelled_cocreation_requests() -> &'static Mutex<HashSet<String>> {
    CANCELLED_COCREATION_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn begin_cocreation_request(request_id: &str) {
    if let Ok(mut cancelled) = cancelled_cocreation_requests().lock() {
        cancelled.remove(request_id);
    }
    if let Ok(mut active) = active_cocreation_requests().lock() {
        active.insert(request_id.to_string());
    }
}

fn finish_cocreation_request(request_id: &str) {
    if let Ok(mut active) = active_cocreation_requests().lock() {
        active.remove(request_id);
    }
    if let Ok(mut cancelled) = cancelled_cocreation_requests().lock() {
        cancelled.remove(request_id);
    }
}

fn cancel_cocreation_request(request_id: &str) {
    if let Ok(mut cancelled) = cancelled_cocreation_requests().lock() {
        cancelled.insert(request_id.to_string());
    }
}

fn cocreation_request_cancelled(request_id: &str) -> bool {
    cancelled_cocreation_requests()
        .lock()
        .map(|cancelled| cancelled.contains(request_id))
        .unwrap_or(false)
}

fn check_cocreation_cancelled(request_id: Option<&str>) -> Result<(), String> {
    if request_id.is_some_and(cocreation_request_cancelled) {
        Err("对话已停止。".to_string())
    } else {
        Ok(())
    }
}

async fn await_cancellable<T, Fut>(request_id: Option<&str>, future: Fut) -> Result<T, String>
where
    Fut: Future<Output = Result<T, String>>,
{
    let Some(request_id) = request_id else {
        return future.await;
    };
    tokio::select! {
        result = future => result,
        _ = wait_for_cocreation_cancel(request_id.to_string()) => Err("对话已停止。".to_string()),
    }
}

async fn wait_for_cocreation_cancel(request_id: String) {
    loop {
        if cocreation_request_cancelled(&request_id) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[derive(Debug, Deserialize)]
struct ModelCoCreateResponse {
    reply: Option<String>,
    #[serde(default)]
    edits: Vec<CoCreateEdit>,
    #[serde(default)]
    memories: Vec<MemoryLeafDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories: Vec<MemoryLeafDraft>,
}

async fn cocreate_with_model(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    ensure_supported_protocol(&settings.protocol)?;
    if is_openai_oauth_settings(settings) {
        return cocreate_with_openai_oauth(
            settings,
            project_model,
            input,
            memories,
            active_context,
            active_project_context,
        )
        .await;
    }
    match settings.protocol.as_str() {
        "anthropic" => {
            cocreate_with_anthropic(
                settings,
                project_model,
                input,
                memories,
                active_context,
                active_project_context,
            )
            .await
        }
        "google" => {
            cocreate_with_gemini(
                settings,
                project_model,
                input,
                memories,
                active_context,
                active_project_context,
            )
            .await
        }
        "openai-compatible" => {
            cocreate_with_openai_compatible(
                settings,
                project_model,
                input,
                memories,
                active_context,
                active_project_context,
            )
            .await
        }
        _ => unreachable!("protocol checked before dispatch"),
    }
}

async fn cocreate_with_openai_oauth(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let url = format!("{}/responses", settings.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let mut request = client.post(url).bearer_auth(&settings.api_key);
    if let Some(account_id) = openai_oauth_account_id()? {
        request = request.header("chatgpt-account-id", account_id);
    }
    let response = request
        .json(&json!({
            "model": project_model.unwrap_or(&settings.model),
            "instructions": cocreation_system_prompt(),
            "input": build_cocreation_prompt(input, memories, active_context, active_project_context),
            "store": false,
            "temperature": 0.7
        }))
        .send()
        .await
        .map_err(|error| format!("对话请求失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("对话响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "对话请求失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ));
    }
    let content = read_model_response_text(&body)?;
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

async fn cocreate_with_openai_compatible(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let url = openai_chat_completions_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let model = project_model.unwrap_or(&settings.model);
    let prompt = build_cocreation_prompt(input, memories, active_context, active_project_context);
    let response = client
        .post(url.clone())
        .bearer_auth(&settings.api_key)
        .json(&openai_compatible_cocreation_body(model, &prompt, true))
        .send()
        .await
        .map_err(|error| format!("对话请求失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("对话响应读取失败：{error}"))?;
    let body = if !status.is_success()
        && should_retry_without_response_format(status.as_u16(), &body)
        && body_mentions_response_format(&body)
    {
        let retry = client
            .post(url)
            .bearer_auth(&settings.api_key)
            .json(&openai_compatible_cocreation_body(model, &prompt, false))
            .send()
            .await
            .map_err(|error| format!("对话请求重试失败：{error}"))?;
        let retry_status = retry.status();
        let retry_body = retry
            .text()
            .await
            .map_err(|error| format!("对话重试响应读取失败：{error}"))?;
        if !retry_status.is_success() {
            return Err(format!(
                "对话请求失败：HTTP {} {}",
                retry_status.as_u16(),
                response_body_summary(&retry_body)
            ));
        }
        retry_body
    } else if !status.is_success() {
        return Err(format!(
            "对话请求失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ));
    } else {
        body
    };
    let content = read_model_response_text(&body)?;
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

fn openai_compatible_cocreation_body(model: &str, prompt: &str, strict_json: bool) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": cocreation_system_prompt()
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.7
    });
    if strict_json {
        body["response_format"] = json!({ "type": "json_object" });
    }
    body
}

fn should_retry_without_response_format(status: u16, body: &str) -> bool {
    matches!(status, 400 | 404 | 422)
        && (body.to_ascii_lowercase().contains("unsupported")
            || body.to_ascii_lowercase().contains("invalid")
            || body_mentions_response_format(body))
}

fn body_mentions_response_format(body: &str) -> bool {
    let body = body.to_ascii_lowercase();
    body.contains("response_format") || body.contains("json_object")
}

async fn cocreate_with_anthropic(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let body = send_anthropic_cocreation_request(
        settings,
        project_model,
        input,
        memories,
        active_context,
        active_project_context,
        false,
    )
    .await?;
    let content = match read_anthropic_response_text(&body) {
        Err(error) if is_anthropic_compatible_parse_error(&error) => {
            let body = send_anthropic_cocreation_request(
                settings,
                project_model,
                input,
                memories,
                active_context,
                active_project_context,
                true,
            )
            .await?;
            read_anthropic_response_text(&body)?
        }
        result => result?,
    };
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

async fn send_anthropic_cocreation_request(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    stream: bool,
) -> Result<String, String> {
    let url = anthropic_messages_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let request = apply_anthropic_auth_headers(
        client.post(url).header("anthropic-version", "2023-06-01"),
        settings,
    );
    let response = request
        .json(&json!({
            "model": project_model.unwrap_or(&settings.model),
            "system": cocreation_system_prompt(),
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": build_cocreation_prompt(input, memories, active_context, active_project_context)
                        }
                    ]
                }
            ],
            "max_tokens": 2048,
            "temperature": 0.7,
            "stream": stream
        }))
        .send()
        .await
        .map_err(|error| format!("对话请求失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("对话响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "对话请求失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ));
    }
    Ok(body)
}

async fn cocreate_with_gemini(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let model = project_model.unwrap_or(&settings.model);
    let url = gemini_generate_content_url(&settings.base_url, model);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let mut request = client.post(url);
    if settings.auth_style == "oauth_external" {
        request = request.bearer_auth(&settings.api_key);
    } else {
        request = request.header("x-goog-api-key", &settings.api_key);
    }
    let response = request
        .json(&json!({
            "systemInstruction": {
                "parts": [{ "text": cocreation_system_prompt() }]
            },
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        { "text": build_cocreation_prompt(input, memories, active_context, active_project_context) }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
                "maxOutputTokens": GEMINI_DEFAULT_MAX_OUTPUT_TOKENS,
                "temperature": 0.7
            }
        }))
        .send()
        .await
        .map_err(|error| format!("对话请求失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("对话响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "对话请求失败：HTTP {} {}",
            status.as_u16(),
            response_body_summary(&body)
        ));
    }
    let content = read_gemini_response_text(&body)?;
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

fn read_active_context(data_dir: &Path) -> String {
    let path = runtime_root(data_dir).join("active-context.json");
    fs::read_to_string(path)
        .map(|content| compact_text(&content, BUDGET_ACTIVE_CONTEXT_CHARS))
        .unwrap_or_default()
}

fn expand_context_items(
    data_dir: &Path,
    skill_resource_root: Option<&Path>,
    items: &[DialogueContextItem],
) -> Result<Vec<DialogueContextItem>, String> {
    items
        .iter()
        .map(|item| expand_context_item(data_dir, skill_resource_root, item))
        .collect()
}

fn expand_context_item(
    data_dir: &Path,
    skill_resource_root: Option<&Path>,
    item: &DialogueContextItem,
) -> Result<DialogueContextItem, String> {
    let Some(raw_path) = referenced_context_path(item) else {
        return Ok(item.clone());
    };
    let (path, relative_path) =
        resolve_allowed_context_file(data_dir, skill_resource_root, item, &raw_path)?;
    let content =
        fs::read_to_string(&path).map_err(|error| format!("上下文文件读取失败：{error}"))?;
    let mut expanded = item.clone();
    expanded.source_path = Some(path.to_string_lossy().into_owned());
    if expanded
        .relative_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        expanded.relative_path = Some(relative_path);
    }
    expanded.value = content;
    Ok(expanded)
}

fn referenced_context_path(item: &DialogueContextItem) -> Option<PathBuf> {
    let source_path = item.source_path.as_deref().unwrap_or("").trim();
    if !source_path.is_empty() {
        return Some(PathBuf::from(source_path));
    }
    let value = item.value.trim();
    if let Some(path) = value.strip_prefix("path:") {
        let trimmed = path.trim();
        return (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
    }
    if let Some(path) = value.strip_prefix("路径：") {
        let trimmed = path.lines().next().unwrap_or("").trim();
        return (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
    }
    None
}

fn resolve_allowed_context_file(
    data_dir: &Path,
    skill_resource_root: Option<&Path>,
    item: &DialogueContextItem,
    raw_path: &Path,
) -> Result<(PathBuf, String), String> {
    let canonical = raw_path
        .canonicalize()
        .map_err(|error| format!("上下文文件不存在：{error}"))?;
    if !canonical.is_file() {
        return Err("上下文引用必须指向文件。".to_string());
    }
    for root in selected_context_roots(data_dir)? {
        if canonical.starts_with(&root) {
            let relative = canonical
                .strip_prefix(&root)
                .unwrap_or(&canonical)
                .to_string_lossy()
                .replace('\\', "/");
            return Ok((canonical, relative));
        }
    }
    if item.kind.trim() == "tool" {
        if let Some(root) = skill_resource_root {
            let canonical_root = root
                .canonicalize()
                .map_err(|error| format!("内置技能目录解析失败：{error}"))?;
            if canonical.starts_with(&canonical_root) {
                let relative = canonical
                    .strip_prefix(&canonical_root)
                    .unwrap_or(&canonical)
                    .to_string_lossy()
                    .replace('\\', "/");
                return Ok((canonical, format!("skills/{relative}")));
            }
        }
    }
    Err("上下文文件不在已选择的作品库或知识库中。".to_string())
}

fn selected_context_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            roots.push(
                path.canonicalize()
                    .map_err(|error| format!("作品库目录解析失败：{error}"))?,
            );
        }
    }
    let knowledge = resolved_knowledge_root(data_dir)?;
    if knowledge.is_dir() {
        roots.push(
            knowledge
                .canonicalize()
                .map_err(|error| format!("知识库目录解析失败：{error}"))?,
        );
    }
    Ok(roots)
}

fn build_cocreation_prompt(
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> String {
    let current_draft_slot = build_current_draft_slot(input);
    let project_mode_slot = build_text_slot(
        "project-mode",
        "项目记忆",
        active_project_context,
        BUDGET_PROJECT_CONTEXT_CHARS,
        "未启用项目记忆。",
    );
    let active_context_slot = build_text_slot(
        "active-context",
        "当前现场",
        active_context,
        BUDGET_ACTIVE_CONTEXT_CHARS,
        "暂无当前现场。",
    );
    let memory_slot = build_memory_slot(memories);
    let knowledge_slot = build_context_items_slot(
        "explicit-knowledge-cards",
        "已选知识卡",
        &filter_context_items(input, ContextItemSlot::Knowledge),
        BUDGET_EXPLICIT_ITEM_CHARS,
        BUDGET_EXPLICIT_TOTAL_CHARS,
    );
    let relevant_slot = build_context_items_slot(
        "relevant-notes",
        "相关稿件",
        &filter_context_items(input, ContextItemSlot::Relevant),
        BUDGET_EXPLICIT_ITEM_CHARS,
        BUDGET_EXPLICIT_TOTAL_CHARS,
    );
    let tool_slot = build_context_items_slot(
        "skill-protocol",
        "技能规则",
        &filter_context_items(input, ContextItemSlot::Tool),
        BUDGET_TOOL_ITEM_CHARS,
        BUDGET_TOOL_TOTAL_CHARS,
    );
    let draft_kind = match input.draft_kind.as_deref() {
        Some("screenplay") => "短剧/剧本稿件",
        _ => "小说/散文稿件",
    };

    let source_label = prompt_source_label(&input.source_path, &input.title);
    format!(
        "稿件类型：{}\n当前文件：{}\n来源路径：{}\n\n上下文编译顺序：当前稿件/选区 → 项目记忆 → 最近对话现场 → 压缩记忆 → 已选知识卡 → 相关稿件 → 技能规则 → 用户请求。每个槽位独立预算，超预算时按此优先级裁剪，不把作品记忆、知识卡和技能规则混写。\n\n输出格式：必须返回 json object，字段为 reply、edits、memories。\n\n[1 当前稿件与选区]\n{}\n\n[2 项目记忆]\n{}\n\n[3 最近对话现场]\n{}\n\n[4 压缩记忆]\n{}\n\n[5 已选知识卡]\n{}\n\n[6 相关稿件]\n{}\n\n[7 技能规则]\n{}\n\n[8 用户请求]\n{}",
        draft_kind,
        input.title,
        source_label,
        current_draft_slot.block,
        project_mode_slot.block,
        active_context_slot.block,
        memory_slot.block,
        knowledge_slot.block,
        relevant_slot.block,
        tool_slot.block,
        input.user_input.trim()
    )
}

fn build_context_load_status(
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Vec<ContextLoadStatus> {
    vec![
        build_current_draft_slot(input).status,
        build_text_slot(
            "project-mode",
            "项目记忆",
            active_project_context,
            BUDGET_PROJECT_CONTEXT_CHARS,
            "未启用项目记忆。",
        )
        .status,
        build_text_slot(
            "active-context",
            "当前现场",
            active_context,
            BUDGET_ACTIVE_CONTEXT_CHARS,
            "暂无当前现场。",
        )
        .status,
        build_memory_slot(memories).status,
        build_context_items_slot(
            "explicit-knowledge-cards",
            "已选知识卡",
            &filter_context_items(input, ContextItemSlot::Knowledge),
            BUDGET_EXPLICIT_ITEM_CHARS,
            BUDGET_EXPLICIT_TOTAL_CHARS,
        )
        .status,
        build_context_items_slot(
            "relevant-notes",
            "相关稿件",
            &filter_context_items(input, ContextItemSlot::Relevant),
            BUDGET_EXPLICIT_ITEM_CHARS,
            BUDGET_EXPLICIT_TOTAL_CHARS,
        )
        .status,
        build_context_items_slot(
            "skill-protocol",
            "技能规则",
            &filter_context_items(input, ContextItemSlot::Tool),
            BUDGET_TOOL_ITEM_CHARS,
            BUDGET_TOOL_TOTAL_CHARS,
        )
        .status,
        build_text_slot(
            "user-request",
            "用户请求",
            input.user_input.trim(),
            input.user_input.chars().count(),
            "无。",
        )
        .status,
    ]
    .into_iter()
    .filter(|status| status.loaded && status.key != "user-request")
    .collect()
}

#[derive(Debug, Clone, Copy)]
enum ContextItemSlot {
    Knowledge,
    Relevant,
    Tool,
}

#[derive(Debug)]
struct ContextSlotBuild {
    block: String,
    status: ContextLoadStatus,
}

fn build_current_draft_slot(input: &CoCreateInput) -> ContextSlotBuild {
    let selected_text = input
        .selected_text
        .as_deref()
        .map(|text| compact_text_with_status(text, BUDGET_SELECTION_CHARS))
        .filter(|text| !text.output.is_empty());
    let draft_text = compact_text_with_status(&input.content, BUDGET_CURRENT_DRAFT_CHARS);
    let selection_block = selected_text
        .as_ref()
        .map(|text| text.output.clone())
        .unwrap_or_else(|| "未选择片段。".to_string());
    let draft_block = if draft_text.output.is_empty() {
        "当前稿件为空。".to_string()
    } else {
        draft_text.output.clone()
    };
    let included_chars = selected_text
        .as_ref()
        .map(|text| text.included_chars)
        .unwrap_or(0)
        + draft_text.included_chars;
    let truncated =
        selected_text.as_ref().is_some_and(|text| text.truncated) || draft_text.truncated;
    let has_selected_text = input
        .selected_text
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty());
    let has_draft_text = !input.content.trim().is_empty() && !input.source_path.trim().is_empty();
    ContextSlotBuild {
        block: format!(
            "用户选中的片段：\n{}\n\n稿件内容：\n{}",
            selection_block, draft_block
        ),
        status: ContextLoadStatus {
            key: "current-draft-selection".to_string(),
            label: "当前稿件/选区".to_string(),
            loaded: has_draft_text || has_selected_text,
            item_count: usize::from(has_draft_text) + usize::from(has_selected_text),
            included_chars,
            budget_chars: BUDGET_CURRENT_DRAFT_CHARS + BUDGET_SELECTION_CHARS,
            truncated,
            note: truncated.then(|| "当前稿件或选区已按预算裁剪。".to_string()),
        },
    }
}

fn build_text_slot(
    key: &str,
    label: &str,
    text: &str,
    budget_chars: usize,
    empty_block: &str,
) -> ContextSlotBuild {
    let compact = compact_text_with_status(text, budget_chars);
    let loaded = !compact.output.trim().is_empty();
    ContextSlotBuild {
        block: if loaded {
            compact.output
        } else {
            empty_block.to_string()
        },
        status: ContextLoadStatus {
            key: key.to_string(),
            label: label.to_string(),
            loaded,
            item_count: usize::from(loaded),
            included_chars: compact.included_chars,
            budget_chars,
            truncated: compact.truncated,
            note: compact.truncated.then(|| format!("{label} 已按预算裁剪。")),
        },
    }
}

fn build_memory_slot(memories: &[String]) -> ContextSlotBuild {
    if memories.is_empty() {
        return ContextSlotBuild {
            block: "暂无压缩记忆。".to_string(),
            status: ContextLoadStatus {
                key: "compressed-memory".to_string(),
                label: "压缩记忆".to_string(),
                loaded: false,
                item_count: 0,
                included_chars: 0,
                budget_chars: BUDGET_MEMORY_TOTAL_CHARS,
                truncated: false,
                note: None,
            },
        };
    }

    let mut rendered = String::new();
    let mut included_chars = 0;
    let mut item_count = 0;
    let mut truncated = false;
    for memory in memories {
        let compact = compact_text_with_status(memory, 420);
        let item = format!("- {}\n", compact.output);
        if rendered.chars().count() + item.chars().count() > BUDGET_MEMORY_TOTAL_CHARS {
            rendered.push_str("- 其余压缩记忆因篇幅限制未展开。\n");
            truncated = true;
            break;
        }
        included_chars += compact.included_chars;
        item_count += 1;
        truncated = truncated || compact.truncated;
        rendered.push_str(&item);
    }

    ContextSlotBuild {
        block: rendered.trim().to_string(),
        status: ContextLoadStatus {
            key: "compressed-memory".to_string(),
            label: "压缩记忆".to_string(),
            loaded: item_count > 0,
            item_count,
            included_chars,
            budget_chars: BUDGET_MEMORY_TOTAL_CHARS,
            truncated,
            note: truncated.then(|| "压缩记忆已保留关键部分。".to_string()),
        },
    }
}

fn filter_context_items<'a>(
    input: &'a CoCreateInput,
    slot: ContextItemSlot,
) -> Vec<&'a DialogueContextItem> {
    input
        .context_items
        .iter()
        .filter(|item| match slot {
            ContextItemSlot::Tool => item.kind.trim() == "tool",
            ContextItemSlot::Knowledge => item.kind.trim() == "memory",
            ContextItemSlot::Relevant => {
                let kind = item.kind.trim();
                kind != "tool" && kind != "memory"
            }
        })
        .collect()
}

fn prompt_source_label(source_path: &str, title: &str) -> String {
    Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| (!title.trim().is_empty()).then(|| title.trim().to_string()))
        .unwrap_or_else(|| "当前稿件".to_string())
}

fn build_context_items_slot(
    key: &str,
    label: &str,
    items: &[&DialogueContextItem],
    per_item_budget: usize,
    total_budget: usize,
) -> ContextSlotBuild {
    if items.is_empty() {
        return ContextSlotBuild {
            block: "无。".to_string(),
            status: ContextLoadStatus {
                key: key.to_string(),
                label: label.to_string(),
                loaded: false,
                item_count: 0,
                included_chars: 0,
                budget_chars: total_budget,
                truncated: false,
                note: None,
            },
        };
    }
    let mut rendered = String::new();
    let mut included_chars = 0;
    let mut item_count = 0;
    let mut truncated = false;
    for item in items {
        let value = compact_text_with_status(&item.value, per_item_budget);
        if value.output.trim().is_empty() {
            continue;
        }
        let source = item
            .relative_path
            .as_deref()
            .or(item.source_path.as_deref())
            .unwrap_or("")
            .trim();
        let header = if source.is_empty() {
            format!("【{}｜{}】", item.kind.trim(), item.label.trim())
        } else {
            format!(
                "【{}｜{}｜{}】",
                item.kind.trim(),
                item.label.trim(),
                source
            )
        };
        let block = format!("{header}\n{}\n", value.output);
        if rendered.chars().count() + block.chars().count() > total_budget {
            rendered.push_str("【预算】其余上下文因预算限制未展开。\n");
            truncated = true;
            break;
        }
        included_chars += value.included_chars;
        item_count += 1;
        truncated = truncated || value.truncated;
        rendered.push_str(&block);
    }
    let block = if rendered.trim().is_empty() {
        "无。".to_string()
    } else {
        rendered.trim().to_string()
    };
    ContextSlotBuild {
        block,
        status: ContextLoadStatus {
            key: key.to_string(),
            label: label.to_string(),
            loaded: item_count > 0,
            item_count,
            included_chars,
            budget_chars: total_budget,
            truncated,
            note: truncated.then(|| format!("{label} 已按预算裁剪。")),
        },
    }
}

pub(crate) fn read_model_response_text(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("对话响应格式损坏：{error}"))?;
    if let Some(content) = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(read_content_value)
    {
        return Ok(content);
    }
    if let Some(content) = value
        .get("output_text")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
    {
        return Ok(content);
    }
    if let Some(content) = value
        .get("output")
        .and_then(serde_json::Value::as_array)
        .and_then(|output| read_responses_output_text(output))
    {
        return Ok(content);
    }
    Err("对话响应缺少可解析文本内容。".to_string())
}

fn read_content_value(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    let content = value.as_array()?;
    let mut parts = Vec::new();
    for item in content {
        if let Some(text) = item.get("text").and_then(serde_json::Value::as_str) {
            parts.push(text.to_string());
        } else if let Some(text) = item
            .get("text")
            .and_then(|text| text.get("value"))
            .and_then(serde_json::Value::as_str)
        {
            parts.push(text.to_string());
        }
    }
    (!parts.is_empty()).then(|| parts.join(""))
}

fn read_responses_output_text(output: &[serde_json::Value]) -> Option<String> {
    let mut parts = Vec::new();
    for item in output {
        if let Some(content) = item.get("content").and_then(serde_json::Value::as_array) {
            for content_item in content {
                if let Some(text) = content_item
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        content_item
                            .get("output_text")
                            .and_then(serde_json::Value::as_str)
                    })
                {
                    parts.push(text.to_string());
                }
            }
        }
    }
    (!parts.is_empty()).then(|| parts.join(""))
}

fn compact_text(text: &str, max_chars: usize) -> String {
    compact_text_with_status(text, max_chars).output
}

#[derive(Debug, Clone)]
struct CompactText {
    output: String,
    included_chars: usize,
    truncated: bool,
}

fn compact_text_with_status(text: &str, max_chars: usize) -> CompactText {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let total_chars = compact.chars().count();
    let output = compact.chars().take(max_chars).collect::<String>();
    CompactText {
        included_chars: output.chars().count(),
        output,
        truncated: total_chars > max_chars,
    }
}

fn parse_cocreation_model_output(output: &str) -> Result<ParsedCoCreateResponse, String> {
    let trimmed = output.trim();
    let parsed: ModelCoCreateResponse = match serde_json::from_str(trimmed) {
        Ok(parsed) => parsed,
        Err(error) => {
            if !trimmed.starts_with('{') && !trimmed.starts_with('[') && !trimmed.is_empty() {
                return Ok(ParsedCoCreateResponse {
                    reply: trimmed.to_string(),
                    edits: Vec::new(),
                    memories: Vec::new(),
                });
            }
            return Err(format!("对话结果不是有效 JSON：{error}"));
        }
    };
    let reply = parsed.reply.unwrap_or_default().trim().to_string();
    let edits = parsed
        .edits
        .into_iter()
        .filter_map(|edit| {
            let target = edit.target.trim().to_string();
            let replacement = edit.replacement.trim().to_string();
            if target.is_empty() || replacement.is_empty() || target == replacement {
                return None;
            }
            Some(CoCreateEdit {
                target,
                replacement,
                rationale: edit
                    .rationale
                    .map(|text| text.trim().to_string())
                    .filter(|text| !text.is_empty()),
            })
        })
        .collect();
    let memories = parsed
        .memories
        .into_iter()
        .filter_map(normalize_model_memory_leaf)
        .collect();
    Ok(ParsedCoCreateResponse {
        reply,
        edits,
        memories,
    })
}

fn normalize_model_memory_leaf(mut leaf: MemoryLeafDraft) -> Option<MemoryLeafDraft> {
    leaf.branch = leaf.branch.trim().to_lowercase();
    leaf.title = leaf.title.trim().to_string();
    leaf.summary = leaf.summary.trim().to_string();
    leaf.reason = leaf
        .reason
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    leaf.source_path = leaf
        .source_path
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    if leaf.branch.is_empty() || leaf.title.is_empty() || leaf.summary.is_empty() {
        None
    } else {
        Some(leaf)
    }
}

fn cocreation_system_prompt() -> &'static str {
    r#"你是 Wridian 的写作对话助手。
你的任务是围绕当前稿件给出可执行的写作建议、局部改写方案或结构判断。
你会同时服务小说和短剧/剧本创作：小说关注章节、人物动机、叙述节奏、伏笔和设定一致性；短剧/剧本关注对白、场景冲突、钩子、角色口吻和分集节奏。
当稿件类型是短剧/剧本时，优先关注场次、对白可表演性、结尾钩子、分集节奏和低成本拍摄约束。
不要写成通用聊天回复；不要自动声称已经修改正文。
你需要判断本轮是否产生值得长期保留的创作记忆。如果没有，memories 输出空数组；如果有，只提取稳定、可复用、对后续写作有约束或参考价值的事实，不记录一次性闲聊。
必须输出 JSON 对象（json object）：
{"reply":"给用户看的正常回复","edits":[{"target":"需要被替换的原文片段，必须从稿件内容或用户选中片段中逐字复制","replacement":"替换后的新文本","rationale":"简短理由"}],"memories":[{"branch":"novel|drama|knowledge|skill|user|relationship|journey|awareness|sense","title":"短标题","summary":"要沉淀的长期记忆正文","reason":"为什么值得沉淀","sourcePath":"当前来源路径或空"}]}
如果只是聊天、讨论、解释，edits 输出空数组。
如果用户要求修改、润色、批量替换角色名、调整对白或重写片段，必须尽量给 edits。
target 必须是原文中存在的精确片段；不要用行号、摘要或正则；不能确定精确原文时只给 reply，不给 edits。"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-cocreation-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn build_prompt_keeps_draft_memories_and_user_request_separate() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "强化她进门前的动机".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let prompt = build_cocreation_prompt(
            &input,
            &["【剧情线】雨夜场景不能提前暴露凶手。".to_string()],
            "{\"currentChapter\":\"第三章\"}",
            "项目记忆：短剧项目",
        );

        assert!(prompt.contains("稿件内容"));
        assert!(prompt.contains("压缩记忆"));
        assert!(prompt.contains("强化她进门前的动机"));
        assert!(prompt.contains("json object"));
    }

    #[test]
    fn build_prompt_separates_selection_from_explicit_context_items() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "结合人物卡改写".to_string(),
            selected_text: Some("她推开门".to_string()),
            selected_model_id: None,
            context_items: vec![DialogueContextItem {
                kind: "memory".to_string(),
                label: "人物卡".to_string(),
                value: "她怕黑，但不承认。".to_string(),
                source_path: Some("D:/vault/knowledge/人物卡.md".to_string()),
                relative_path: Some("人物卡.md".to_string()),
            }],
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "");

        assert!(prompt.contains("用户选中的片段：\n她推开门"));
        assert!(prompt.contains("[5 已选知识卡]"));
        assert!(prompt.contains("【memory｜人物卡｜人物卡.md】\n她怕黑，但不承认。"));
    }

    #[test]
    fn build_prompt_separates_tool_protocol_from_explicit_context_items() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "体检知识库".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: vec![
                DialogueContextItem {
                    kind: "tool".to_string(),
                    label: "知识库运维".to_string(),
                    value: "Wridian 技能协议：知识库运维".to_string(),
                    source_path: None,
                    relative_path: None,
                },
                DialogueContextItem {
                    kind: "memory".to_string(),
                    label: "知识卡".to_string(),
                    value: "一条显式知识卡".to_string(),
                    source_path: None,
                    relative_path: Some("03故事模型/知识卡.md".to_string()),
                },
            ],
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "");

        assert!(prompt
            .contains("[5 已选知识卡]\n【memory｜知识卡｜03故事模型/知识卡.md】\n一条显式知识卡"));
        assert!(
            prompt.contains("[7 技能规则]\n【tool｜知识库运维】\nWridian 技能协议：知识库运维")
        );
    }

    #[test]
    fn context_load_status_reports_loaded_slots_and_budget_truncation() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门。".repeat(3000),
            draft_kind: Some("prose".to_string()),
            user_input: "结合人物卡改写".to_string(),
            selected_text: Some("她推开门".to_string()),
            selected_model_id: None,
            context_items: vec![
                DialogueContextItem {
                    kind: "memory".to_string(),
                    label: "人物卡".to_string(),
                    value: "她怕黑。".to_string(),
                    source_path: None,
                    relative_path: Some("人物卡.md".to_string()),
                },
                DialogueContextItem {
                    kind: "tool".to_string(),
                    label: "作品拆解".to_string(),
                    value: "按作品拆解技能执行。".to_string(),
                    source_path: None,
                    relative_path: None,
                },
            ],
        };

        let status = build_context_load_status(
            &input,
            &["长期记忆：雨夜不能提前暴露凶手。".to_string()],
            "",
            "项目记忆：短剧项目",
        );

        let current = status
            .iter()
            .find(|item| item.key == "current-draft-selection")
            .expect("current draft slot");
        assert!(current.loaded);
        assert!(current.truncated);
        assert_eq!(
            current.budget_chars,
            BUDGET_CURRENT_DRAFT_CHARS + BUDGET_SELECTION_CHARS
        );

        let knowledge = status
            .iter()
            .find(|item| item.key == "explicit-knowledge-cards")
            .expect("knowledge slot");
        assert!(knowledge.loaded);
        assert_eq!(knowledge.item_count, 1);

        let tool = status
            .iter()
            .find(|item| item.key == "skill-protocol")
            .expect("tool slot");
        assert!(tool.loaded);
        assert_eq!(tool.item_count, 1);
    }

    #[test]
    fn build_prompt_does_not_send_absolute_source_path() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "D:/private/vault/works/第一章.md".to_string(),
            title: "第一章.md".to_string(),
            content: "她推开门。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "润色".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "");

        assert!(prompt.contains("来源路径：第一章.md"));
        assert!(!prompt.contains("D:/private"));
    }

    #[test]
    fn prompt_source_label_falls_back_to_title() {
        assert_eq!(prompt_source_label("", "未保存稿件"), "未保存稿件");
        assert_eq!(prompt_source_label("", ""), "当前稿件");
    }

    #[test]
    fn expands_path_context_items_from_selected_library() {
        let data_dir = temp_data_dir("path-context");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        let card_path = knowledge_root.join("人物.md");
        fs::write(&card_path, "她怕黑，但不承认。").expect("write card");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            crate::runtime::workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let expanded = expand_context_items(
            &data_dir,
            None,
            &[DialogueContextItem {
                kind: "memory".to_string(),
                label: "人物".to_string(),
                value: format!("path:{}", card_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("expand context");

        assert_eq!(expanded[0].value, "她怕黑，但不承认。");
        assert_eq!(expanded[0].relative_path.as_deref(), Some("人物.md"));
    }

    #[test]
    fn accepts_default_knowledge_context_without_user_selection() {
        let data_dir = temp_data_dir("default-knowledge-context");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let default_knowledge = crate::runtime::default_knowledge_root(&data_dir);
        let card_path = default_knowledge.join("03故事模型").join("默认人物.md");
        fs::write(&card_path, "默认知识").expect("write default card");

        let expanded = expand_context_items(
            &data_dir,
            None,
            &[DialogueContextItem {
                kind: "memory".to_string(),
                label: "默认人物".to_string(),
                value: format!("path:{}", card_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("default knowledge context should be accepted");

        assert_eq!(expanded[0].value, "默认知识");
        assert_eq!(
            expanded[0].relative_path.as_deref(),
            Some("03故事模型/默认人物.md")
        );
    }

    #[test]
    fn expands_builtin_skill_resource_for_tool_context() {
        let data_dir = temp_data_dir("builtin-skill-context");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let skill_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("skills");
        let skill_path = skill_root.join("zhishiku-skill").join("SKILL.md");

        let expanded = expand_context_items(
            &data_dir,
            Some(&skill_root),
            &[DialogueContextItem {
                kind: "tool".to_string(),
                label: "知识库运维".to_string(),
                value: format!("path:{}", skill_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("builtin skill context should be accepted");

        assert!(expanded[0].value.contains("# 知识库总控流程"));
        assert_eq!(
            expanded[0].relative_path.as_deref(),
            Some("skills/zhishiku-skill/SKILL.md")
        );
    }

    #[test]
    fn read_model_response_text_reads_first_choice_message() {
        let body = r#"{
            "choices": [
                { "message": { "content": "可以先补一段动作。" } }
            ]
        }"#;

        let content = read_model_response_text(body).expect("content exists");

        assert_eq!(content, "可以先补一段动作。");
    }

    #[test]
    fn read_model_response_text_reads_content_parts() {
        let body = r#"{
            "choices": [
                { "message": { "content": [
                    { "type": "text", "text": "第一段" },
                    { "type": "text", "text": "第二段" }
                ] } }
            ]
        }"#;

        let content = read_model_response_text(body).expect("content exists");

        assert_eq!(content, "第一段第二段");
    }

    #[test]
    fn read_model_response_text_reads_responses_output_text() {
        let body = r#"{
            "output": [
                { "content": [
                    { "type": "output_text", "text": "{\"reply\":\"好\",\"edits\":[],\"memories\":[]}" }
                ] }
            ]
        }"#;

        let content = read_model_response_text(body).expect("content exists");

        assert!(content.contains("\"reply\":\"好\""));
    }

    #[test]
    fn parse_cocreation_model_output_reads_reply_and_edits() {
        let output = r#"{
            "reply": "我会把动机提前到进门动作里。",
            "edits": [
                {
                    "target": "她没有立刻喊人。",
                    "replacement": "她没有立刻喊人，而是先摸了摸口袋里那把旧钥匙。",
                    "rationale": "让动机从动作里出现。"
                }
            ]
        }"#;

        let parsed = parse_cocreation_model_output(output).expect("valid output");

        assert_eq!(parsed.reply, "我会把动机提前到进门动作里。");
        assert_eq!(parsed.edits.len(), 1);
        assert_eq!(parsed.edits[0].target, "她没有立刻喊人。");
    }

    #[test]
    fn parse_cocreation_model_output_reads_memory_leaves() {
        let output = r#"{
            "reply": "这个人物禁区我会记住。",
            "edits": [],
            "memories": [
                {
                    "branch": "novel",
                    "title": "人物禁区",
                    "summary": "女主不能主动说出真相。",
                    "reason": "这是后续章节约束。",
                    "sourcePath": "chapter.md"
                }
            ]
        }"#;

        let parsed = parse_cocreation_model_output(output).expect("valid output");

        assert_eq!(parsed.memories.len(), 1);
        assert_eq!(parsed.memories[0].branch, "novel");
        assert_eq!(parsed.memories[0].title, "人物禁区");
    }

    #[test]
    fn parse_cocreation_model_output_uses_plain_text_as_reply() {
        let parsed = parse_cocreation_model_output("现在的年月日时间是 2026-06-11。")
            .expect("plain text reply");

        assert_eq!(parsed.reply, "现在的年月日时间是 2026-06-11。");
        assert!(parsed.edits.is_empty());
        assert!(parsed.memories.is_empty());
    }

    #[test]
    fn openai_compatible_body_can_omit_response_format_for_legacy_gateways() {
        let strict = openai_compatible_cocreation_body("model-a", "prompt", true);
        assert_eq!(
            strict
                .get("response_format")
                .and_then(|value| value.get("type"))
                .and_then(serde_json::Value::as_str),
            Some("json_object")
        );

        let fallback = openai_compatible_cocreation_body("model-a", "prompt", false);
        assert!(fallback.get("response_format").is_none());
    }

    #[test]
    fn response_format_retry_gate_only_fires_for_relevant_errors() {
        assert!(should_retry_without_response_format(
            400,
            "unsupported response_format"
        ));
        assert!(body_mentions_response_format(
            "json_object is not supported"
        ));
        assert!(!should_retry_without_response_format(
            401,
            "response_format unsupported"
        ));
    }

    #[test]
    fn gemini_default_output_limit_is_high_enough_for_json_reply() {
        assert_eq!(GEMINI_DEFAULT_MAX_OUTPUT_TOKENS, 65535);
    }
}
