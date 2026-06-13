use crate::memory::{read_relevant_memory_snippets, write_memory_leaves, MemoryLeafDraft};
use crate::model_accounts::{
    anthropic_messages_url, apply_anthropic_auth_headers, ensure_google_gemini_oauth_project_id,
    ensure_supported_protocol, gemini_generate_content_url,
    google_gemini_cloudcode_generate_content_url, google_gemini_cloudcode_request_body,
    is_anthropic_compatible_parse_error, is_google_gemini_oauth_settings, is_openai_oauth_settings,
    openai_chat_completions_url, openai_compatible_chat_body, openai_oauth_account_id,
    post_google_code_assist_json, read_active_model_settings, read_anthropic_response_text,
    read_gemini_response_text, response_body_summary, ActiveModelSettings,
    GEMINI_DEFAULT_MAX_OUTPUT_TOKENS,
};
use crate::projects::{active_project_model, read_active_project_context};
use crate::rule_router::RuleRouteContext;
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::workspace::{
    apply_workspace_create_folder, apply_workspace_rename_node, apply_workspace_trash_node,
    apply_workspace_write_file, read_workspace_file_trees, WorkFileNode,
};
use crate::workspace::{
    is_supported_text_preview_file, read_active_work_root, read_workspace_text_content,
    resolved_knowledge_root,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::io::Write;
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
const BUDGET_FILE_TREE_CHARS: usize = 2200;
const BUDGET_MENTIONED_FILE_ITEM_CHARS: usize = 1800;
const BUDGET_MENTIONED_FILE_TOTAL_CHARS: usize = 5200;
const BUDGET_RULE_ROUTE_CHARS: usize = 5200;

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
    file_operations: Vec<AppliedFileOperation>,
    memories_used: Vec<String>,
    memories_written: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyChatFileOperationsInput {
    source_path: Option<String>,
    operations: Vec<ModelFileOperation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyChatFileOperationsResponse {
    file_operations: Vec<AppliedFileOperation>,
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

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppliedFileOperation {
    action: String,
    library: String,
    path: String,
    ok: bool,
    message: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ModelFileOperation {
    action: String,
    library: String,
    path: String,
    #[serde(default)]
    new_name: Option<String>,
    #[serde(default)]
    content: Option<String>,
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
pub(crate) async fn wridian_apply_chat_file_operations(
    _app: AppHandle,
    input: ApplyChatFileOperationsInput,
) -> Result<ApplyChatFileOperationsResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut operations = input.operations;
    route_new_work_file_operations_to_current_folder(
        &data_dir,
        input.source_path.as_deref().unwrap_or(""),
        &mut operations,
    );
    Ok(ApplyChatFileOperationsResponse {
        file_operations: apply_model_file_operations(&data_dir, &operations),
    })
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
    let selected_model_id = input.selected_model_id.clone();
    let settings_data_dir = data_dir.clone();
    let settings = tokio::task::spawn_blocking(move || {
        read_active_model_settings(&settings_data_dir, selected_model_id.as_deref())
    })
    .await
    .map_err(|error| format!("模型设置读取任务失败：{error}"))??
    .ok_or_else(|| "请先在模型设置里保存模型账户。".to_string())?;
    let memories_used =
        read_relevant_memory_snippets(&data_dir, &input.source_path, &input.title, 8)?;
    let active_context = read_active_context(&data_dir);
    let active_project_context = read_active_project_context(&data_dir)?;
    let rule_route_context = crate::rule_router::read_rule_route_context(&data_dir)?;
    let project_model = active_project_model(&data_dir)?;
    let file_tree = read_workspace_file_trees(&data_dir)?;
    let file_tree_slot = build_file_tree_slot(&file_tree);
    let mentioned_files = read_user_mentioned_workspace_files(&input, &file_tree);
    let context_load_status = build_context_load_status(
        &input,
        &memories_used,
        &active_context,
        &active_project_context,
        &rule_route_context,
        &file_tree_slot,
        &mentioned_files,
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
            &rule_route_context,
            &file_tree_slot.block,
            &mentioned_files,
        ),
    )
    .await?;
    check_cocreation_cancelled(request_id)?;
    let model_output = await_cancellable(
        request_id,
        repair_missing_file_operations_for_file_requests(
            &settings,
            project_model.as_deref(),
            &input,
            &memories_used,
            &active_context,
            &active_project_context,
            &rule_route_context,
            &file_tree_slot.block,
            &mentioned_files,
            model_output,
        ),
    )
    .await?;
    check_cocreation_cancelled(request_id)?;
    let model_output = route_current_file_writes_to_edits(&data_dir, &input, model_output);
    let mut model_output = route_new_work_files_to_current_folder(&data_dir, &input, model_output);

    let memories_written = write_memory_leaves(&data_dir, &model_output.memories)?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let file_operations = apply_model_file_operations(&data_dir, &model_output.file_operations);
    model_output.reply = model_output.reply.trim().to_string();

    Ok(CoCreateResponse {
        context_load_status,
        reply: model_output.reply,
        edits: model_output.edits,
        file_operations,
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
#[serde(rename_all = "camelCase")]
struct ModelCoCreateResponse {
    reply: Option<String>,
    #[serde(default)]
    edits: Vec<CoCreateEdit>,
    #[serde(default)]
    file_operations: Vec<ModelFileOperation>,
    #[serde(default)]
    memories: Vec<MemoryLeafDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
    file_operations: Vec<ModelFileOperation>,
    memories: Vec<MemoryLeafDraft>,
}

async fn cocreate_with_model(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
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
            rule_route_context,
            file_tree,
            mentioned_files,
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
                rule_route_context,
                file_tree,
                mentioned_files,
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
                rule_route_context,
                file_tree,
                mentioned_files,
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
                rule_route_context,
                file_tree,
                mentioned_files,
            )
            .await
        }
        _ => unreachable!("protocol checked before dispatch"),
    }
}

async fn repair_missing_file_operations_for_file_requests(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
    parsed: ParsedCoCreateResponse,
) -> Result<ParsedCoCreateResponse, String> {
    if !should_repair_missing_file_operations(input, &parsed) {
        return Ok(parsed);
    }
    let mut repair_input = clone_cocreation_input(input);
    repair_input.user_input = build_file_operation_repair_user_input(input, &parsed.reply);
    match cocreate_with_model(
        settings,
        project_model,
        &repair_input,
        memories,
        active_context,
        active_project_context,
        rule_route_context,
        file_tree,
        mentioned_files,
    )
    .await
    {
        Ok(repaired) if !repaired.file_operations.is_empty() => Ok(repaired),
        _ if reply_can_seed_local_file_operation(&parsed.reply) => Ok(parsed),
        _ => Ok(reject_missing_file_operations_for_file_requests(input, parsed)),
    }
}

fn clone_cocreation_input(input: &CoCreateInput) -> CoCreateInput {
    CoCreateInput {
        request_id: input.request_id.clone(),
        source_path: input.source_path.clone(),
        title: input.title.clone(),
        content: input.content.clone(),
        draft_kind: input.draft_kind.clone(),
        user_input: input.user_input.clone(),
        selected_text: input.selected_text.clone(),
        selected_model_id: input.selected_model_id.clone(),
        context_items: input.context_items.clone(),
    }
}

fn build_file_operation_repair_user_input(input: &CoCreateInput, previous_reply: &str) -> String {
    format!(
        "上一轮回复没有返回可执行的 fileOperations，Wridian 实际没有执行任何文件树操作。请基于同一当前稿件和同一用户请求，重新只返回可执行 JSON：reply 简短说明，edits 为空，memories 为空，fileOperations 必须包含实际需要的文件树操作。用户原请求：{}\n上一轮回复：{}\n如果用户没有指定扩展名，新建文档默认使用 .md；如果用户要求放在新建文档里，使用 writeFile 写入完整初始内容；重命名使用 rename；删除使用 trash；创建目录使用 createFolder。不要再只在 reply 里给正文或口头描述。",
        input.user_input.trim(),
        previous_reply.trim()
    )
}

async fn cocreate_with_openai_oauth(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
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
            "input": build_cocreation_prompt(input, memories, active_context, active_project_context, rule_route_context, file_tree, mentioned_files),
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
    ensure_parsed_cocreation_response(parse_cocreation_model_output(&content)?)
}

async fn cocreate_with_openai_compatible(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
) -> Result<ParsedCoCreateResponse, String> {
    let url = openai_chat_completions_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let model = project_model.unwrap_or(&settings.model);
    let prompt = build_cocreation_prompt(
        input,
        memories,
        active_context,
        active_project_context,
        rule_route_context,
        file_tree,
        mentioned_files,
    );
    let response = client
        .post(url.clone())
        .bearer_auth(&settings.api_key)
        .json(&openai_compatible_cocreation_body(
            settings, model, &prompt, true,
        ))
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
            .json(&openai_compatible_cocreation_body(
                settings, model, &prompt, false,
            ))
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
    ensure_parsed_cocreation_response(parse_cocreation_model_output(&content)?)
}

fn openai_compatible_cocreation_body(
    settings: &ActiveModelSettings,
    model: &str,
    prompt: &str,
    strict_json: bool,
) -> Value {
    let mut body = openai_compatible_chat_body(
        settings,
        model,
        json!([
            {
                "role": "system",
                "content": cocreation_system_prompt()
            },
            {
                "role": "user",
                "content": prompt
            }
        ]),
        2048,
        0.7,
    );
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
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
) -> Result<ParsedCoCreateResponse, String> {
    let body = send_anthropic_cocreation_request(
        settings,
        project_model,
        input,
        memories,
        active_context,
        active_project_context,
        rule_route_context,
        file_tree,
        mentioned_files,
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
                rule_route_context,
                file_tree,
                mentioned_files,
                true,
            )
            .await?;
            read_anthropic_response_text(&body)?
        }
        result => result?,
    };
    ensure_parsed_cocreation_response(parse_cocreation_model_output(&content)?)
}

async fn send_anthropic_cocreation_request(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
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
                            "text": build_cocreation_prompt(input, memories, active_context, active_project_context, rule_route_context, file_tree, mentioned_files)
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
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
) -> Result<ParsedCoCreateResponse, String> {
    let model = project_model.unwrap_or(&settings.model);
    if is_google_gemini_oauth_settings(settings) {
        return cocreate_with_google_gemini_cloudcode(
            settings,
            model,
            input,
            memories,
            active_context,
            active_project_context,
            rule_route_context,
            file_tree,
            mentioned_files,
        )
        .await;
    }
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
                        { "text": build_cocreation_prompt(input, memories, active_context, active_project_context, rule_route_context, file_tree, mentioned_files) }
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
    ensure_parsed_cocreation_response(parse_cocreation_model_output(&content)?)
}

async fn cocreate_with_google_gemini_cloudcode(
    settings: &ActiveModelSettings,
    model: &str,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
) -> Result<ParsedCoCreateResponse, String> {
    let project_id = ensure_google_gemini_oauth_project_id(&settings.api_key, model).await?;
    let inner_request = json!({
        "systemInstruction": {
            "parts": [{ "text": cocreation_system_prompt() }]
        },
        "contents": [
            {
                "role": "user",
                "parts": [
                    { "text": build_cocreation_prompt(input, memories, active_context, active_project_context, rule_route_context, file_tree, mentioned_files) }
                ]
            }
        ],
        "generationConfig": {
            "responseMimeType": "application/json",
            "maxOutputTokens": GEMINI_DEFAULT_MAX_OUTPUT_TOKENS,
            "temperature": 0.7
        }
    });
    let body = google_gemini_cloudcode_request_body(&project_id, model, inner_request);
    let response = post_google_code_assist_json(
        &google_gemini_cloudcode_generate_content_url(),
        &body,
        &settings.api_key,
        model,
    )
    .await?;
    let body = serde_json::to_string(&response).map_err(|error| error.to_string())?;
    let content = read_gemini_response_text(&body)?;
    ensure_parsed_cocreation_response(parse_cocreation_model_output(&content)?)
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
    let content = if crate::workspace::is_supported_text_preview_file(&path) {
        crate::workspace::read_workspace_text_content(&path)?
    } else {
        format!(
            "文件引用：{}\n当前格式暂不支持抽取为文本上下文，可在中间文件区查看或用本机程序打开。",
            relative_path
        )
    };
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
    rule_route_context: &RuleRouteContext,
    file_tree: &str,
    mentioned_files: &[DialogueContextItem],
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
    let file_tree_slot = build_text_slot(
        "workspace-file-tree",
        "作品库和知识库文件树",
        file_tree,
        BUDGET_FILE_TREE_CHARS,
        "暂无可读取的文件树。",
    );
    let mut rule_route_slot = build_text_slot(
        "rule-router",
        "规则路由",
        &rule_route_context.block,
        BUDGET_RULE_ROUTE_CHARS,
        "暂无 WRIDIAN.md / AGENT.md / index.md 规则路由。",
    );
    rule_route_slot.status.item_count = rule_route_context.item_count;
    rule_route_slot.status.truncated =
        rule_route_slot.status.truncated || rule_route_context.truncated;
    let knowledge_slot = build_context_items_slot(
        "explicit-knowledge-cards",
        "已选知识卡",
        &filter_context_items(input, ContextItemSlot::Knowledge),
        BUDGET_EXPLICIT_ITEM_CHARS,
        BUDGET_EXPLICIT_TOTAL_CHARS,
    );
    let explicit_file_items = filter_context_items(input, ContextItemSlot::File);
    let mentioned_file_refs: Vec<&DialogueContextItem> = mentioned_files.iter().collect();
    let file_context_slot = build_combined_context_items_slot(
        "mentioned-files",
        "点名文件",
        &explicit_file_items,
        &mentioned_file_refs,
        BUDGET_MENTIONED_FILE_ITEM_CHARS,
        BUDGET_MENTIONED_FILE_TOTAL_CHARS,
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
        "稿件类型：{}\n当前文件：{}\n来源路径：{}\n\n上下文编译顺序：当前稿件/选区 → 作品库和知识库文件树 → 规则路由 → 项目记忆 → 最近对话现场 → 压缩记忆 → 已选知识卡 → 点名文件 → 技能规则 → 用户请求。每个槽位独立预算，超预算时按此优先级裁剪，不把作品记忆、知识卡、规则路由和技能规则混写。\n\n规则路由协议：WRIDIAN.md、AGENT.md、AGENTS.md 定义当前库内的长期行动规则；index.md 和 hot.md 定义当前库的导航与近期上下文。规则路由只说明如何工作和如何定位资料，不等于把知识卡写入作品记忆。\n\n技能工作流协议：当 [9 技能规则] 非空时，所选技能必须按可执行工作流处理，不得只输出泛泛建议。先确认输入和扫描范围，再给出产物清单；需要落地文件时必须返回 fileOperations；完成后在 reply 中说明质量检查结果和回滚方式。技能产物优先写入当前库内相对路径：作品拆解进入知识库 02拆解报告，知识卡进入知识库 03-07，作者/大神 skill 进入知识库 08大神蒸馏，知识库体检进入 00知识库治理。不能验证来源、关联、frontmatter 或可分发性时，不得声称完成，只能输出待补缺口。\n\n编辑回复协议：只有当用户明确要求修改、重写、润色、替换、整理当前稿件正文、改成某版本或删除正文内容时，才返回 edits；普通聊天、询问原因、让给建议或比较方案时 edits 必须为空。用户明确要求改正文时，不要给多个候选方案让用户选择；直接给唯一改稿结果。Wridian 会自动写入能安全定位的 edits，并保留撤销。reply 只简短说明已整理的重点；不要说“我给你两个方向/挑一个/确认后再改”，不要把 replacement 长篇贴到 reply 里。\n\n文件树权限协议：用户在对话中提到作品库或知识库文件树内的文件时，Wridian 可以在当前库内读取该文件内容，并可通过 fileOperations 新建文件、创建文件夹、重命名或移到回收站。无需默认展示文件内容给用户；需要说明时只说明操作结果。\n\n文件树操作协议：如需操作文件树，必须在 fileOperations 数组里返回操作，不要只在 reply 里描述。只允许相对路径，不允许绝对路径或 ..。action 只能是 writeFile、createFolder、rename、trash；library 只能是 works 或 knowledge；writeFile 只能用于新建不存在的 md、txt、docx 文件并写入初始内容；对已有文件内容执行新增、修改、删除时，只有用户明确要求改正文才返回 edits，新建含内容文件除外；rename 需要 newName；writeFile 需要 content；trash 表示移到用户本机系统回收站，不创建库内 .wridian-trash。禁止在 fileOperations 为空时说“已新建/已创建/已写入/已保存为某文件”；这种回复会被 Wridian 视为未执行。\n\n输出格式：必须返回 json object，字段为 reply、edits、fileOperations、memories。\n\n[1 当前稿件与选区]\n{}\n\n[2 作品库和知识库文件树]\n{}\n\n[3 规则路由]\n{}\n\n[4 项目记忆]\n{}\n\n[5 最近对话现场]\n{}\n\n[6 压缩记忆]\n{}\n\n[7 已选知识卡]\n{}\n\n[8 点名文件]\n{}\n\n[9 技能规则]\n{}\n\n[10 用户请求]\n{}",
        draft_kind,
        input.title,
        source_label,
        current_draft_slot.block,
        file_tree_slot.block,
        rule_route_slot.block,
        project_mode_slot.block,
        active_context_slot.block,
        memory_slot.block,
        knowledge_slot.block,
        file_context_slot.block,
        tool_slot.block,
        input.user_input.trim()
    )
}

fn build_context_load_status(
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    rule_route_context: &RuleRouteContext,
    file_tree_slot: &ContextSlotBuild,
    mentioned_files: &[DialogueContextItem],
) -> Vec<ContextLoadStatus> {
    let explicit_file_items = filter_context_items(input, ContextItemSlot::File);
    let mentioned_file_refs: Vec<&DialogueContextItem> = mentioned_files.iter().collect();
    let mut rule_route_status = build_text_slot(
        "rule-router",
        "规则路由",
        &rule_route_context.block,
        BUDGET_RULE_ROUTE_CHARS,
        "暂无 WRIDIAN.md / AGENT.md / index.md 规则路由。",
    )
    .status;
    rule_route_status.item_count = rule_route_context.item_count;
    rule_route_status.truncated = rule_route_status.truncated || rule_route_context.truncated;
    vec![
        build_current_draft_slot(input).status,
        file_tree_slot.status.clone(),
        rule_route_status,
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
        build_combined_context_items_slot(
            "mentioned-files",
            "点名文件",
            &explicit_file_items,
            &mentioned_file_refs,
            BUDGET_MENTIONED_FILE_ITEM_CHARS,
            BUDGET_MENTIONED_FILE_TOTAL_CHARS,
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
    File,
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

fn build_file_tree_slot(nodes: &[WorkFileNode]) -> ContextSlotBuild {
    let mut lines = Vec::new();
    for node in nodes {
        render_file_tree_node(node, 0, &mut lines);
    }
    build_text_slot(
        "workspace-file-tree",
        "作品库和知识库文件树",
        &lines.join("\n"),
        BUDGET_FILE_TREE_CHARS,
        "暂无可读取的文件树。",
    )
}

fn render_file_tree_node(node: &WorkFileNode, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let kind = if node.folder { "folder" } else { "file" };
    let path = if node.relative_path.trim().is_empty() {
        node.name.as_str()
    } else {
        node.relative_path.as_str()
    };
    lines.push(format!(
        "{indent}- [{}] {}/{}",
        kind,
        node.library.trim(),
        path
    ));
    for child in &node.children {
        render_file_tree_node(child, depth + 1, lines);
    }
}

fn read_user_mentioned_workspace_files(
    input: &CoCreateInput,
    nodes: &[WorkFileNode],
) -> Vec<DialogueContextItem> {
    let query = normalize_match_text(&input.user_input);
    if query.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
    collect_user_mentioned_workspace_files(nodes, &query, &mut items);
    items
}

fn collect_user_mentioned_workspace_files(
    nodes: &[WorkFileNode],
    query: &str,
    items: &mut Vec<DialogueContextItem>,
) {
    for node in nodes {
        if node.folder {
            collect_user_mentioned_workspace_files(&node.children, query, items);
            continue;
        }
        if !file_node_is_mentioned(query, node) {
            continue;
        }
        let path = Path::new(&node.path);
        if !is_supported_text_preview_file(path) {
            continue;
        }
        let Ok(content) = read_workspace_text_content(path) else {
            continue;
        };
        items.push(DialogueContextItem {
            kind: "file".to_string(),
            label: node.name.clone(),
            value: content,
            source_path: None,
            relative_path: Some(format!(
                "{}/{}",
                node.library.trim(),
                node.relative_path.trim()
            )),
        });
    }
}

fn file_node_is_mentioned(query: &str, node: &WorkFileNode) -> bool {
    let relative = normalize_match_text(&node.relative_path);
    let library_relative =
        normalize_match_text(&format!("{}/{}", node.library, node.relative_path));
    let name = normalize_match_text(&node.name);
    if (!relative.is_empty() && query.contains(&relative))
        || (!library_relative.is_empty() && query.contains(&library_relative))
        || (!name.is_empty() && query.contains(&name))
    {
        return true;
    }
    let Some(stem) = Path::new(&node.name).file_stem() else {
        return false;
    };
    let stem = normalize_match_text(&stem.to_string_lossy());
    stem.chars().count() >= 3 && query.contains(&stem)
}

fn normalize_match_text(text: &str) -> String {
    text.trim()
        .replace('\\', "/")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
        .to_lowercase()
}

fn route_current_file_writes_to_edits(
    data_dir: &Path,
    input: &CoCreateInput,
    mut parsed: ParsedCoCreateResponse,
) -> ParsedCoCreateResponse {
    if input.source_path.trim().is_empty() {
        return parsed;
    }
    let Some(source_path) = canonical_existing_path(Path::new(&input.source_path)) else {
        return parsed;
    };
    let mut routed_operations = Vec::with_capacity(parsed.file_operations.len());
    for operation in parsed.file_operations {
        if operation.action.trim() == "writeFile"
            && operation
                .content
                .as_deref()
                .map(str::trim)
                .filter(|content| !content.is_empty())
                .is_some()
            && model_operation_targets_path(data_dir, &operation, &source_path)
        {
            parsed.edits.push(CoCreateEdit {
                target: input.content.clone(),
                replacement: operation.content.unwrap_or_default(),
                rationale: Some("根据当前打开文件的原文生成待确认修改。".to_string()),
            });
            continue;
        }
        routed_operations.push(operation);
    }
    parsed.file_operations = routed_operations;
    parsed
}

fn model_operation_targets_path(
    data_dir: &Path,
    operation: &ModelFileOperation,
    expected: &Path,
) -> bool {
    let relative_path =
        normalize_model_operation_relative_path(&operation.library, &operation.path);
    let Ok(root) = crate::workspace::workspace_library_root_for_audit(data_dir, &operation.library)
    else {
        return false;
    };
    let Ok(target) =
        crate::workspace::resolve_relative_workspace_target_for_audit(&root, &relative_path)
    else {
        return false;
    };
    canonical_existing_path(&target).as_deref() == Some(expected)
}

fn canonical_existing_path(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok()
}

fn reject_missing_file_operations_for_file_requests(
    input: &CoCreateInput,
    mut parsed: ParsedCoCreateResponse,
) -> ParsedCoCreateResponse {
    if !should_repair_missing_file_operations(input, &parsed) {
        return parsed;
    }
    parsed.reply =
        "这次模型没有返回可执行的文件树操作；Wridian 已拦截这条回复，没有新建、修改或删除任何文件。请重新发送一次文件树操作请求，最好带上目标文件名或目录。".to_string();
    parsed.edits.clear();
    parsed.memories.clear();
    parsed
}

fn should_repair_missing_file_operations(
    input: &CoCreateInput,
    parsed: &ParsedCoCreateResponse,
) -> bool {
    parsed.file_operations.is_empty()
        && user_requested_file_tree_write(input)
}

fn user_requested_file_tree_write(input: &CoCreateInput) -> bool {
    let text = normalize_match_text(&input.user_input);
    if text.is_empty() {
        return false;
    }
    let has_create_intent = [
        "新建",
        "创建",
        "新增",
        "生成",
        "写入",
        "放到",
        "放在",
        "保存",
        "重命名",
        "改名",
        "修改文件名",
        "改文件名",
        "删除",
        "删掉",
        "移除",
        "移到回收站",
        "移动到回收站",
    ]
    .iter()
    .any(|keyword| text.contains(keyword));
    let has_file_target = [
        "文件",
        "文档",
        "文件树",
        "作品库",
        "知识库",
        "文件夹",
        "目录",
        "稿件",
        "md",
        "markdown",
        "docx",
        "txt",
    ]
    .iter()
    .any(|keyword| text.contains(keyword));
    has_create_intent && has_file_target
}

fn reply_can_seed_local_file_operation(reply: &str) -> bool {
    let text = normalize_match_text(reply);
    if text.chars().count() < 20 {
        return false;
    }
    if reply_claims_file_tree_write(reply) {
        return false;
    }
    !text.contains("没有返回可执行的文件树操作")
}

fn reply_claims_file_tree_write(reply: &str) -> bool {
    let text = normalize_match_text(reply);
    if text.is_empty() {
        return false;
    }
    let claims_done = [
        "已新建",
        "已创建",
        "已写入",
        "已保存",
        "新建为",
        "创建为",
        "写入到",
        "保存到",
    ]
    .iter()
    .any(|keyword| text.contains(keyword));
    let mentions_file = [
        "works/",
        "knowledge/",
        ".md",
        ".markdown",
        ".docx",
        ".txt",
        "文件",
        "文档",
    ]
    .iter()
    .any(|keyword| text.contains(keyword));
    claims_done && mentions_file
}

fn route_new_work_files_to_current_folder(
    data_dir: &Path,
    input: &CoCreateInput,
    mut parsed: ParsedCoCreateResponse,
) -> ParsedCoCreateResponse {
    route_new_work_file_operations_to_current_folder(
        data_dir,
        &input.source_path,
        &mut parsed.file_operations,
    );
    parsed
}

fn route_new_work_file_operations_to_current_folder(
    data_dir: &Path,
    source_path: &str,
    operations: &mut [ModelFileOperation],
) {
    let current_relative_dir = current_work_file_relative_dir(data_dir, source_path);
    if current_relative_dir.is_none() {
        return;
    }
    let current_relative_dir = current_relative_dir.unwrap_or_default();
    for operation in operations {
        if operation.action.trim() != "writeFile" || operation.library.trim() != "works" {
            continue;
        }
        let normalized =
            normalize_model_operation_relative_path(&operation.library, &operation.path);
        if normalized.contains('/') {
            operation.path = normalized;
            continue;
        }
        operation.path = if current_relative_dir.is_empty() {
            normalized
        } else {
            format!("{}/{}", current_relative_dir, normalized)
        };
    }
}

fn current_work_file_relative_dir(data_dir: &Path, source_path: &str) -> Option<String> {
    if source_path.trim().is_empty() {
        return None;
    }
    let source_path = canonical_existing_path(Path::new(source_path))?;
    let root = crate::workspace::workspace_library_root_for_audit(data_dir, "works").ok()?;
    if !source_path.starts_with(&root) {
        return None;
    }
    let relative = source_path.strip_prefix(&root).ok()?;
    let parent = relative.parent()?;
    let relative_dir = parent.to_string_lossy().replace('\\', "/");
    Some(relative_dir.trim_matches('/').to_string())
}

fn apply_model_file_operations(
    data_dir: &Path,
    operations: &[ModelFileOperation],
) -> Vec<AppliedFileOperation> {
    operations
        .iter()
        .map(|operation| apply_model_file_operation(data_dir, operation))
        .collect()
}

fn apply_model_file_operation(
    data_dir: &Path,
    operation: &ModelFileOperation,
) -> AppliedFileOperation {
    let action = operation.action.trim().to_string();
    let library = operation.library.trim().to_string();
    let path = normalize_model_operation_relative_path(&library, &operation.path);
    let before = file_operation_snapshot(data_dir, &library, &path);
    let result = ensure_model_operation_library_configured(data_dir, &library).and_then(|_| {
        match action.as_str() {
            "writeFile" => apply_workspace_write_file(
                data_dir,
                &library,
                &path,
                operation.content.as_deref().unwrap_or(""),
            )
            .map(|path| format!("已写入 {}", path.to_string_lossy())),
            "createFolder" => apply_workspace_create_folder(data_dir, &library, &path)
                .map(|path| format!("已创建文件夹 {}", path.to_string_lossy())),
            "rename" => {
                let new_name = operation
                    .new_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "rename 操作缺少 newName。".to_string());
                new_name.and_then(|new_name| {
                    apply_workspace_rename_node(data_dir, &library, &path, new_name)
                        .map(|path| format!("已重命名为 {}", path.to_string_lossy()))
                })
            }
            "trash" => apply_workspace_trash_node(data_dir, &library, &path)
                .map(|path| format!("已移到系统回收站 {}", path.to_string_lossy())),
            _ => Err("未知文件操作 action。".to_string()),
        }
    });
    let applied = match result {
        Ok(message) => AppliedFileOperation {
            action,
            library,
            path,
            ok: true,
            message,
        },
        Err(error) => AppliedFileOperation {
            action,
            library,
            path,
            ok: false,
            message: error,
        },
    };
    audit_model_file_operation(data_dir, operation.new_name.as_deref(), &before, &applied);
    applied
}

fn ensure_model_operation_library_configured(data_dir: &Path, library: &str) -> Result<(), String> {
    match library.trim() {
        "works" => {
            let root = read_active_work_root(data_dir)?
                .map(PathBuf::from)
                .filter(|path| path.is_dir());
            if root.is_some() {
                Ok(())
            } else {
                Err("请先在作品库打开本地文件夹，再让 Wridian 操作文件树。".to_string())
            }
        }
        "knowledge" => Ok(()),
        _ => Err("文件操作 library 必须是 works 或 knowledge。".to_string()),
    }
}

fn normalize_model_operation_relative_path(library: &str, path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    let prefixes: &[&str] = match library.trim() {
        "works" => &["works/", "作品库/"],
        "knowledge" => &["knowledge/", "知识库/"],
        _ => &[],
    };
    for prefix in prefixes {
        if let Some(stripped) = normalized.strip_prefix(prefix) {
            return stripped.trim_start_matches('/').to_string();
        }
    }
    normalized
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelFileOperationAudit<'a> {
    timestamp: String,
    action: &'a str,
    library: &'a str,
    path: &'a str,
    new_name: Option<&'a str>,
    ok: bool,
    message: &'a str,
    before: FileOperationSnapshot,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileOperationSnapshot {
    exists: bool,
    kind: String,
    len: Option<u64>,
    sha256: Option<String>,
    note: Option<String>,
}

fn file_operation_snapshot(
    data_dir: &Path,
    library: &str,
    relative_path: &str,
) -> FileOperationSnapshot {
    let Ok(root) = crate::workspace::workspace_library_root_for_audit(data_dir, library) else {
        return FileOperationSnapshot {
            exists: false,
            kind: "unknown".to_string(),
            len: None,
            sha256: None,
            note: Some("库目录无法解析。".to_string()),
        };
    };
    let Ok(path) =
        crate::workspace::resolve_relative_workspace_target_for_audit(&root, relative_path)
    else {
        return FileOperationSnapshot {
            exists: false,
            kind: "unknown".to_string(),
            len: None,
            sha256: None,
            note: Some("相对路径无效。".to_string()),
        };
    };
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return FileOperationSnapshot {
                exists: false,
                kind: "missing".to_string(),
                len: None,
                sha256: None,
                note: None,
            };
        }
        Err(error) => {
            return FileOperationSnapshot {
                exists: false,
                kind: "unknown".to_string(),
                len: None,
                sha256: None,
                note: Some(format!("路径信息读取失败：{error}")),
            };
        }
    };
    if metadata.is_dir() {
        return FileOperationSnapshot {
            exists: true,
            kind: "directory".to_string(),
            len: None,
            sha256: None,
            note: None,
        };
    }
    if !metadata.is_file() {
        return FileOperationSnapshot {
            exists: true,
            kind: "other".to_string(),
            len: Some(metadata.len()),
            sha256: None,
            note: None,
        };
    }
    let sha256 = if metadata.len() <= 2 * 1024 * 1024 {
        fs::read(&path).ok().map(|bytes| sha256_bytes(&bytes))
    } else {
        None
    };
    FileOperationSnapshot {
        exists: true,
        kind: "file".to_string(),
        len: Some(metadata.len()),
        sha256,
        note: (metadata.len() > 2 * 1024 * 1024).then(|| "文件过大，审计只记录大小。".to_string()),
    }
}

fn audit_model_file_operation(
    data_dir: &Path,
    new_name: Option<&str>,
    before: &FileOperationSnapshot,
    applied: &AppliedFileOperation,
) {
    let dir = runtime_root(data_dir);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let audit = ModelFileOperationAudit {
        timestamp: crate::runtime::iso_timestamp(),
        action: applied.action.as_str(),
        library: applied.library.as_str(),
        path: applied.path.as_str(),
        new_name,
        ok: applied.ok,
        message: applied.message.as_str(),
        before: before.clone(),
    };
    let Ok(line) = serde_json::to_string(&audit) else {
        return;
    };
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("model-file-operations.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            ContextItemSlot::File => {
                item.kind.trim() == "file" || item.kind.trim() == "active-file"
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
    build_combined_context_items_slot(key, label, items, &[], per_item_budget, total_budget)
}

fn build_combined_context_items_slot(
    key: &str,
    label: &str,
    primary_items: &[&DialogueContextItem],
    secondary_items: &[&DialogueContextItem],
    per_item_budget: usize,
    total_budget: usize,
) -> ContextSlotBuild {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for item in primary_items.iter().chain(secondary_items.iter()) {
        let source_key = item
            .relative_path
            .as_deref()
            .or(item.source_path.as_deref())
            .unwrap_or("")
            .trim()
            .to_string();
        let key = if source_key.is_empty() {
            format!("{}:{}", item.kind.trim(), item.label.trim())
        } else {
            format!("{}:{}", item.kind.trim(), source_key)
        };
        if seen.insert(key) {
            items.push(*item);
        }
    }
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
        Err(_) => {
            let Some(payload) = extract_json_payload(trimmed) else {
                if looks_like_structured_cocreation_output(trimmed) {
                    return Ok(recover_malformed_cocreation_response(
                        trimmed, trimmed, None,
                    ));
                }
                return Ok(plain_text_cocreation_response(trimmed));
            };
            match serde_json::from_str(&payload) {
                Ok(parsed) => parsed,
                Err(parse_error) => {
                    return Ok(recover_malformed_cocreation_response(
                        trimmed,
                        &payload,
                        Some(&parse_error),
                    ));
                }
            }
        }
    };
    let reply = parsed.reply.unwrap_or_default().trim().to_string();
    let edits = parsed
        .edits
        .into_iter()
        .filter_map(|edit| {
            let target = edit.target.trim().to_string();
            let replacement = edit.replacement.trim().to_string();
            if replacement.is_empty() || target == replacement {
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
        file_operations: parsed.file_operations,
        memories,
    })
}

fn plain_text_cocreation_response(output: &str) -> ParsedCoCreateResponse {
    ParsedCoCreateResponse {
        reply: output.trim().to_string(),
        edits: Vec::new(),
        file_operations: Vec::new(),
        memories: Vec::new(),
    }
}

fn recover_malformed_cocreation_response(
    output: &str,
    payload: &str,
    parse_error: Option<&serde_json::Error>,
) -> ParsedCoCreateResponse {
    let file_operations = extract_cocreation_file_operations_lossy(payload)
        .or_else(|| extract_cocreation_file_operations_lossy(output))
        .unwrap_or_default();
    let reply = extract_json_string_field(payload, "reply")
        .or_else(|| extract_json_string_field(output, "reply"))
        .or_else(|| (!file_operations.is_empty()).then(|| "已按你的要求处理文件树。".to_string()))
        .or_else(|| extract_first_meaningful_text(output))
        .unwrap_or_else(|| {
            parse_error
                .map(|error| format!("模型回复格式不完整，已作为普通回复显示。原解析错误：{error}"))
                .unwrap_or_else(|| "模型回复格式不完整，已隐藏原始结构化内容。".to_string())
        });
    ParsedCoCreateResponse {
        reply,
        edits: extract_cocreation_edits_lossy(payload)
            .or_else(|| extract_cocreation_edits_lossy(output))
            .unwrap_or_default(),
        file_operations,
        memories: Vec::new(),
    }
}

fn looks_like_structured_cocreation_output(output: &str) -> bool {
    output.contains("\"reply\"")
        || output.contains("\"edits\"")
        || output.contains("\"fileOperations\"")
        || output.contains("\"memories\"")
}

fn extract_first_meaningful_text(output: &str) -> Option<String> {
    let cleaned = output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("```")
                && !line.starts_with('{')
                && !line.starts_with('}')
                && !line.starts_with('"')
                && !line.starts_with('[')
                && !line.starts_with(']')
        })
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.trim().is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn extract_json_string_field(payload: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let key_start = payload.find(&key)?;
    let after_key = &payload[key_start + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let value_start = after_colon.find('"')?;
    read_json_string_lossy(&after_colon[value_start..])
}

fn read_json_string_lossy(input: &str) -> Option<String> {
    let mut chars = input.char_indices();
    let (_, quote) = chars.next()?;
    if quote != '"' {
        return None;
    }
    let mut output = String::new();
    let mut escaped = false;
    for (index, char) in input[1..].char_indices() {
        if escaped {
            match char {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'u' => output.push_str("\\u"),
                other => output.push(other),
            }
            escaped = false;
            continue;
        }
        if char == '\\' {
            escaped = true;
            continue;
        }
        if char == '"' {
            let rest = input[index + 2..].trim_start();
            if rest.starts_with(',') || rest.starts_with('}') || rest.starts_with(']') {
                return Some(output.trim().to_string());
            }
        }
        output.push(char);
    }
    let text = output.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_cocreation_edits_lossy(payload: &str) -> Option<Vec<CoCreateEdit>> {
    let edits_payload = extract_json_array_field_lossy(payload, "edits")?;
    let mut edits = Vec::new();
    for object in extract_json_objects_lossy(&edits_payload) {
        let target = extract_json_string_field(&object, "target")?;
        let replacement = extract_json_string_field(&object, "replacement")?;
        if target.trim().is_empty()
            || replacement.trim().is_empty()
            || target.trim() == replacement.trim()
        {
            continue;
        }
        edits.push(CoCreateEdit {
            target: target.trim().to_string(),
            replacement: replacement.trim().to_string(),
            rationale: extract_json_string_field(&object, "rationale")
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty()),
        });
    }
    if edits.is_empty() {
        None
    } else {
        Some(edits)
    }
}

fn extract_cocreation_file_operations_lossy(payload: &str) -> Option<Vec<ModelFileOperation>> {
    let operations_payload = extract_json_array_field_lossy(payload, "fileOperations")?;
    let mut operations = Vec::new();
    for object in extract_json_objects_lossy(&operations_payload) {
        let Some(action) = extract_json_string_field(&object, "action") else {
            continue;
        };
        let Some(library) = extract_json_string_field(&object, "library") else {
            continue;
        };
        let Some(path) = extract_json_string_field(&object, "path") else {
            continue;
        };
        let action = action.trim().to_string();
        let library = library.trim().to_string();
        let path = path.trim().to_string();
        if action.is_empty() || library.is_empty() || path.is_empty() {
            continue;
        }
        operations.push(ModelFileOperation {
            action,
            library,
            path,
            new_name: extract_json_string_field(&object, "newName")
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty()),
            content: extract_json_string_field(&object, "content")
                .map(|text| text.trim().to_string()),
        });
    }
    if operations.is_empty() {
        None
    } else {
        Some(operations)
    }
}

fn extract_json_array_field_lossy(payload: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let key_start = payload.find(&key)?;
    let after_key = &payload[key_start + key.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let bracket_start = after_colon.find('[')?;
    let array = &after_colon[bracket_start..];
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, char) in array.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                let rest = array[index + char.len_utf8()..].trim_start();
                if rest.starts_with(',') || rest.starts_with('}') || rest.starts_with(']') {
                    in_string = false;
                }
            }
            continue;
        }
        match char {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(array[..=index].to_string());
                }
            }
            _ => {}
        }
    }
    (depth > 0).then(|| array.to_string())
}

fn extract_json_objects_lossy(array_payload: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut object_start: Option<usize> = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, char) in array_payload.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                let rest = array_payload[index + char.len_utf8()..].trim_start();
                if rest.starts_with(':')
                    || rest.starts_with(',')
                    || rest.starts_with('}')
                    || rest.starts_with(']')
                {
                    in_string = false;
                }
            }
            continue;
        }
        match char {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    object_start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(start) = object_start.take() {
                        objects.push(array_payload[start..=index].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    objects
}

fn extract_json_payload(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }
    extract_fenced_json_payload(trimmed)
        .or_else(|| extract_loose_fenced_json_payload(trimmed))
        .or_else(|| extract_balanced_json_payload(trimmed))
}

fn extract_fenced_json_payload(output: &str) -> Option<String> {
    let mut rest = output;
    while let Some(start) = rest.find("```") {
        let after_start = &rest[start + 3..];
        let Some(end) = after_start.find("```") else {
            return None;
        };
        let block = strip_fence_language(&after_start[..end]);
        let candidate = block.trim();
        if candidate.starts_with('{') || candidate.starts_with('[') {
            return Some(candidate.to_string());
        }
        rest = &after_start[end + 3..];
    }
    None
}

fn extract_loose_fenced_json_payload(output: &str) -> Option<String> {
    let trimmed = output.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    let after_fence = lower
        .strip_prefix("```json")
        .map(|_| &trimmed["```json".len()..])
        .or_else(|| {
            lower
                .strip_prefix("``` json")
                .map(|_| &trimmed["``` json".len()..])
        })?;
    extract_balanced_json_payload(after_fence)
}

fn strip_fence_language(block: &str) -> &str {
    let trimmed = block.trim_start();
    let Some(newline) = trimmed.find('\n') else {
        return trimmed;
    };
    let first_line = trimmed[..newline].trim();
    let rest = &trimmed[newline + 1..];
    if first_line.eq_ignore_ascii_case("json")
        || first_line.chars().all(|char| char.is_ascii_alphabetic())
    {
        rest
    } else {
        trimmed
    }
}

fn extract_balanced_json_payload(output: &str) -> Option<String> {
    let start = output
        .char_indices()
        .find(|(_, char)| matches!(char, '{' | '['))?
        .0;
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    for (offset, char) in output[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            continue;
        }
        match char {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if stack.pop() != Some(char) {
                    return None;
                }
                if stack.is_empty() {
                    let end = start + offset + char.len_utf8();
                    return Some(output[start..end].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn ensure_parsed_cocreation_response(
    mut parsed: ParsedCoCreateResponse,
) -> Result<ParsedCoCreateResponse, String> {
    if !parsed.reply.trim().is_empty() {
        return Ok(parsed);
    }
    if parsed.file_operations.is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        parsed.reply = "已按你的要求处理文件树。".to_string();
        Ok(parsed)
    }
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
不要写成通用聊天回复；用户要求改正文时不要只给建议或候选方案。
如果本轮带有技能规则，必须按技能工作流回复：确认输入与扫描范围、说明产物、需要写入时使用 fileOperations、说明质检结果和回滚方式。
你需要判断本轮是否产生值得长期保留的创作记忆。如果没有，memories 输出空数组；如果有，只提取稳定、可复用、对后续写作有约束或参考价值的事实，不记录一次性闲聊。
必须输出 JSON 对象（json object）：
{"reply":"给用户看的正常回复","edits":[{"target":"需要被替换的原文片段，必须从稿件内容或用户选中片段中逐字复制","replacement":"替换后的新文本","rationale":"简短理由"}],"fileOperations":[{"action":"writeFile|createFolder|rename|trash","library":"works|knowledge","path":"库内相对路径","newName":"rename 时的新名称","content":"writeFile 时的新内容"}],"memories":[{"branch":"novel|drama|knowledge|skill|user|relationship|journey|awareness|sense","title":"短标题","summary":"要沉淀的长期记忆正文","reason":"为什么值得沉淀","sourcePath":"当前来源路径或空"}]}
如果只是聊天、讨论、解释，edits 输出空数组。
只有当用户明确要求修改、重写、润色、替换、整理当前稿件正文、调整对白、删除正文内容或改成某版本时，才返回 edits；普通聊天、询问原因、让给建议或比较方案时 edits 输出空数组。
用户明确要求改正文时，必须给唯一改稿结果并尽量给 edits；不要给“两个方向/两个选择/你挑一个”。
对已有文件内容做新增、修改、删除时，只有用户明确要求改正文才给 edits；Wridian 会自动写入能安全定位的 edits；只有新建不存在的文件并写入初始内容时，才使用 fileOperations.writeFile。
返回 edits 时，reply 只简短说明已整理的重点和无法自动定位的风险；不要把 replacement 长篇贴到 reply 里。
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

    fn write_test_workspace_config(data_dir: &Path, work_root: &Path, knowledge_root: &Path) {
        fs::create_dir_all(crate::runtime::runtime_root(data_dir)).expect("create runtime");
        fs::write(
            crate::runtime::workspace_config_path(data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");
    }

    fn empty_rule_route_context() -> RuleRouteContext {
        RuleRouteContext {
            block: String::new(),
            item_count: 0,
            truncated: false,
        }
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
            &RuleRouteContext {
                block: "【works｜作品库｜规则路由｜WRIDIAN.md】\n作品规则。".to_string(),
                item_count: 1,
                truncated: false,
            },
            "works/第一章.md\nknowledge/人物卡.md",
            &[],
        );

        assert!(prompt.contains("稿件内容"));
        assert!(prompt.contains("作品库和知识库文件树"));
        assert!(prompt.contains("规则路由"));
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

        let prompt =
            build_cocreation_prompt(&input, &[], "", "", &empty_rule_route_context(), "", &[]);

        assert!(prompt.contains("用户选中的片段：\n她推开门"));
        assert!(prompt.contains("[7 已选知识卡]"));
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
                    label: "作品拆解".to_string(),
                    value: "Wridian 技能协议：作品拆解".to_string(),
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

        let prompt =
            build_cocreation_prompt(&input, &[], "", "", &empty_rule_route_context(), "", &[]);

        assert!(prompt
            .contains("[7 已选知识卡]\n【memory｜知识卡｜03故事模型/知识卡.md】\n一条显式知识卡"));
        assert!(prompt.contains("[9 技能规则]\n【tool｜作品拆解】\nWridian 技能协议：作品拆解"));
        assert!(prompt.contains("技能工作流协议：当 [9 技能规则] 非空时"));
        assert!(prompt.contains("需要落地文件时必须返回 fileOperations"));
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
            &RuleRouteContext {
                block: "【knowledge｜知识库｜索引｜index.md】\n知识索引。".to_string(),
                item_count: 1,
                truncated: false,
            },
            &build_text_slot(
                "workspace-file-tree",
                "作品库和知识库文件树",
                "works/第一章.md",
                BUDGET_FILE_TREE_CHARS,
                "暂无可读取的文件树。",
            ),
            &[],
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

        let rules = status
            .iter()
            .find(|item| item.key == "rule-router")
            .expect("rule router slot");
        assert!(rules.loaded);
        assert_eq!(rules.item_count, 1);
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

        let prompt =
            build_cocreation_prompt(&input, &[], "", "", &empty_rule_route_context(), "", &[]);

        assert!(prompt.contains("来源路径：第一章.md"));
        assert!(!prompt.contains("D:/private"));
    }

    #[test]
    fn file_tree_slot_lists_libraries_with_relative_paths() {
        let nodes = vec![WorkFileNode {
            name: "测试".to_string(),
            path: "D:/works/测试".to_string(),
            relative_path: "测试".to_string(),
            library: "works".to_string(),
            folder: true,
            children: vec![WorkFileNode {
                name: "第一集.docx".to_string(),
                path: "D:/works/测试/第一集.docx".to_string(),
                relative_path: "测试/第一集.docx".to_string(),
                library: "works".to_string(),
                folder: false,
                children: Vec::new(),
            }],
        }];

        let slot = build_file_tree_slot(&nodes);

        assert!(slot.status.loaded);
        assert!(slot.block.contains("[folder] works/测试"));
        assert!(slot.block.contains("[file] works/测试/第一集.docx"));
        assert!(!slot.block.contains("D:/works"));
    }

    #[test]
    fn user_mentioned_file_reads_text_content_from_workspace_tree() {
        let data_dir = temp_data_dir("mentioned-file-context");
        let work_root = data_dir.join("works");
        fs::create_dir_all(work_root.join("测试")).expect("create works");
        let file_path = work_root.join("测试").join("第1集.txt");
        fs::write(&file_path, "第一场：雨夜。").expect("write file");
        let input = CoCreateInput {
            request_id: None,
            source_path: String::new(),
            title: String::new(),
            content: String::new(),
            draft_kind: Some("prose".to_string()),
            user_input: "看看作品库里的第1集.txt，然后改名。".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let nodes = vec![WorkFileNode {
            name: "测试".to_string(),
            path: work_root.join("测试").to_string_lossy().into_owned(),
            relative_path: "测试".to_string(),
            library: "works".to_string(),
            folder: true,
            children: vec![WorkFileNode {
                name: "第1集.txt".to_string(),
                path: file_path.to_string_lossy().into_owned(),
                relative_path: "测试/第1集.txt".to_string(),
                library: "works".to_string(),
                folder: false,
                children: Vec::new(),
            }],
        }];

        let mentioned = read_user_mentioned_workspace_files(&input, &nodes);

        assert_eq!(mentioned.len(), 1);
        assert_eq!(mentioned[0].label, "第1集.txt");
        assert_eq!(
            mentioned[0].relative_path.as_deref(),
            Some("works/测试/第1集.txt")
        );
        assert_eq!(mentioned[0].value, "第一场：雨夜。");
    }

    #[test]
    fn user_mentioned_file_ignores_short_stem_and_non_text_files() {
        let data_dir = temp_data_dir("mentioned-file-ignore");
        let work_root = data_dir.join("works");
        fs::create_dir_all(&work_root).expect("create works");
        let short_file = work_root.join("人.txt");
        let image_file = work_root.join("海报.png");
        fs::write(&short_file, "短文件名不应被泛匹配。").expect("write short file");
        fs::write(&image_file, [0_u8, 1, 2]).expect("write image");
        let input = CoCreateInput {
            request_id: None,
            source_path: String::new(),
            title: String::new(),
            content: String::new(),
            draft_kind: Some("prose".to_string()),
            user_input: "人物要更狠，顺便看看海报.png".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let nodes = vec![
            WorkFileNode {
                name: "人.txt".to_string(),
                path: short_file.to_string_lossy().into_owned(),
                relative_path: "人.txt".to_string(),
                library: "works".to_string(),
                folder: false,
                children: Vec::new(),
            },
            WorkFileNode {
                name: "海报.png".to_string(),
                path: image_file.to_string_lossy().into_owned(),
                relative_path: "海报.png".to_string(),
                library: "works".to_string(),
                folder: false,
                children: Vec::new(),
            },
        ];

        let mentioned = read_user_mentioned_workspace_files(&input, &nodes);

        assert!(mentioned.is_empty());
    }

    #[test]
    fn parsed_file_operations_can_write_workspace_files() {
        let data_dir = temp_data_dir("file-operation-write");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        write_test_workspace_config(&data_dir, &work_root, &knowledge_root);
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已创建。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"测试/新场景.md","content":"第一场"}],"memories":[]}"#,
        )
        .expect("parse");

        let results = apply_model_file_operations(&data_dir, &parsed.file_operations);

        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert_eq!(
            fs::read_to_string(work_root.join("测试").join("新场景.md")).expect("read written"),
            "第一场"
        );
        let audit_path =
            crate::runtime::runtime_root(&data_dir).join("model-file-operations.jsonl");
        let audit = fs::read_to_string(audit_path).expect("read operation audit");
        assert!(audit.contains(r#""action":"writeFile""#));
        assert!(audit.contains(r#""library":"works""#));
        assert!(audit.contains(r#""path":"测试/新场景.md""#));
        assert!(audit.contains(r#""exists":false"#));
    }

    #[test]
    fn current_file_write_operation_routes_to_inline_edit() {
        let data_dir = temp_data_dir("current-write-routes-to-edit");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        let current_path = work_root.join("晚.md");
        fs::write(&current_path, "旧内容").expect("write current");
        write_test_workspace_config(&data_dir, &work_root, &knowledge_root);
        let input = CoCreateInput {
            request_id: None,
            source_path: current_path.to_string_lossy().into_owned(),
            title: "晚.md".to_string(),
            content: "旧内容".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "改写".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"给出修改方案。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"晚.md","content":"新内容"}],"memories":[]}"#,
        )
        .expect("parse");

        let routed = route_current_file_writes_to_edits(&data_dir, &input, parsed);

        assert!(routed.file_operations.is_empty());
        assert_eq!(
            routed.edits,
            vec![CoCreateEdit {
                target: "旧内容".to_string(),
                replacement: "新内容".to_string(),
                rationale: Some("根据当前打开文件的原文生成待确认修改。".to_string()),
            }]
        );
        assert_eq!(
            fs::read_to_string(current_path).expect("read current"),
            "旧内容"
        );
    }

    #[test]
    fn write_file_operation_rejects_existing_workspace_file() {
        let data_dir = temp_data_dir("file-ops-existing");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        fs::write(work_root.join("已有.md"), "原内容").expect("write existing");
        write_test_workspace_config(&data_dir, &work_root, &knowledge_root);
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已处理。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"已有.md","content":"新内容"}],"memories":[]}"#,
        )
        .expect("parse");

        let results = apply_model_file_operations(&data_dir, &parsed.file_operations);

        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
        assert!(results[0].message.contains("writeFile 只用于新建文件"));
        assert_eq!(
            fs::read_to_string(work_root.join("已有.md")).expect("read existing"),
            "原内容"
        );
    }

    #[test]
    fn file_operation_strips_library_prefix_from_relative_path() {
        let data_dir = temp_data_dir("file-ops-strip-library-prefix");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        write_test_workspace_config(&data_dir, &work_root, &knowledge_root);
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已创建。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"works/第2集.md","content":"第二集正文"}],"memories":[]}"#,
        )
        .expect("parse");

        let results = apply_model_file_operations(&data_dir, &parsed.file_operations);

        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert_eq!(results[0].path, "第2集.md");
        assert_eq!(
            fs::read_to_string(work_root.join("第2集.md")).expect("read written"),
            "第二集正文"
        );
        assert!(!work_root.join("works").join("第2集.md").exists());
        let audit_path =
            crate::runtime::runtime_root(&data_dir).join("model-file-operations.jsonl");
        let audit = fs::read_to_string(audit_path).expect("read operation audit");
        assert!(audit.contains(r#""path":"第2集.md""#));
    }

    #[test]
    fn new_work_file_routes_to_current_open_file_folder() {
        let data_dir = temp_data_dir("file-ops-current-folder");
        let work_root = data_dir.join("user-works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(work_root.join("测试")).expect("create work folder");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        let current_path = work_root.join("测试").join("第1集.docx");
        fs::write(&current_path, "第一集").expect("write current");
        write_test_workspace_config(&data_dir, &work_root, &knowledge_root);
        let input = CoCreateInput {
            request_id: None,
            source_path: current_path.to_string_lossy().into_owned(),
            title: "第1集.docx".to_string(),
            content: "第一集".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "新建一个md文件，根据第1集剧情，续写出第2集".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已写入 works/第2集.md。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"works/第2集.md","content":"第二集"}],"memories":[]}"#,
        )
        .expect("parse");

        let routed = route_new_work_files_to_current_folder(&data_dir, &input, parsed);
        let results = apply_model_file_operations(&data_dir, &routed.file_operations);

        assert_eq!(routed.file_operations[0].path, "测试/第2集.md");
        assert_eq!(routed.reply, "已写入 works/第2集.md。");
        assert!(results[0].ok);
        assert_eq!(results[0].path, "测试/第2集.md");
        assert_eq!(
            fs::read_to_string(work_root.join("测试").join("第2集.md")).expect("read written"),
            "第二集"
        );
        assert!(!work_root.join("第2集.md").exists());
    }

    #[test]
    fn fake_file_creation_reply_without_file_operations_is_rejected() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "D:/works/测试/第1集.docx".to_string(),
            title: "第1集.docx".to_string(),
            content: "第一集".to_string(),
            draft_kind: Some("screenplay".to_string()),
            user_input: "根据第1集剧情，续写第2集，放在新建文档里".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已根据第1集剧情续写第2集。剧本已新建为 `works/第2集.docx`。","edits":[],"fileOperations":[],"memories":[{"branch":"drama","title":"第2集","summary":"假成功","reason":"测试","sourcePath":"第1集.docx"}]}"#,
        )
        .expect("parse");

        assert!(should_repair_missing_file_operations(&input, &parsed));
        let rejected = reject_missing_file_operations_for_file_requests(&input, parsed);

        assert!(!rejected.reply.contains("works/第2集.docx"));
        assert!(rejected.reply.contains("没有新建、修改或删除任何文件"));
        assert!(rejected.file_operations.is_empty());
        assert!(rejected.edits.is_empty());
        assert!(rejected.memories.is_empty());
    }

    #[test]
    fn explicit_new_document_request_with_plain_reply_is_repaired_or_rejected() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "D:/works/测试/第1集.docx".to_string(),
            title: "第1集.docx".to_string(),
            content: "第一集".to_string(),
            draft_kind: Some("screenplay".to_string()),
            user_input: "新建一个文档，续写第2集".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r###"{"reply":"## 第2集\n\n这一集从上一集结尾继续。","edits":[],"fileOperations":[],"memories":[]}"###,
        )
        .expect("parse");

        assert!(should_repair_missing_file_operations(&input, &parsed));
        let rejected = reject_missing_file_operations_for_file_requests(&input, parsed);

        assert!(rejected.reply.contains("没有新建、修改或删除任何文件"));
        assert!(!rejected.reply.contains("## 第2集"));
        assert!(rejected.file_operations.is_empty());
        assert!(rejected.edits.is_empty());
        assert!(rejected.memories.is_empty());
    }

    #[test]
    fn plain_document_reply_can_seed_local_write_tool_fallback() {
        let parsed = parse_cocreation_model_output(
            r###"{"reply":"## 第2集\n\n这一集从上一集结尾继续，主角走进新的冲突。","edits":[],"fileOperations":[],"memories":[]}"###,
        )
        .expect("parse");

        assert!(reply_can_seed_local_file_operation(&parsed.reply));
        assert!(!reply_claims_file_tree_write(&parsed.reply));
    }

    #[test]
    fn fake_done_reply_cannot_seed_local_write_tool_fallback() {
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已根据第1集剧情续写第2集。剧本已新建为 `works/第2集.docx`。","edits":[],"fileOperations":[],"memories":[]}"#,
        )
        .expect("parse");

        assert!(reply_claims_file_tree_write(&parsed.reply));
        assert!(!reply_can_seed_local_file_operation(&parsed.reply));
    }

    #[test]
    fn normal_reply_without_file_operations_is_not_rejected() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "D:/works/测试/第1集.docx".to_string(),
            title: "第1集.docx".to_string(),
            content: "第一集".to_string(),
            draft_kind: Some("screenplay".to_string()),
            user_input: "第1集节奏怎么样".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"第1集节奏偏快，可以加强转场。","edits":[],"fileOperations":[],"memories":[]}"#,
        )
        .expect("parse");

        assert!(!should_repair_missing_file_operations(&input, &parsed));
        let kept = reject_missing_file_operations_for_file_requests(&input, parsed);

        assert_eq!(kept.reply, "第1集节奏偏快，可以加强转场。");
        assert!(kept.file_operations.is_empty());
    }

    #[test]
    fn file_operation_reply_is_not_rejected() {
        let input = CoCreateInput {
            request_id: None,
            source_path: "D:/works/测试/第1集.docx".to_string(),
            title: "第1集.docx".to_string(),
            content: "第一集".to_string(),
            draft_kind: Some("screenplay".to_string()),
            user_input: "根据第1集剧情，续写第2集，放在新建文档里".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };
        let parsed = parse_cocreation_model_output(
            r#"{"reply":"已新建第2集。","edits":[],"fileOperations":[{"action":"writeFile","library":"works","path":"第2集.md","content":"第二集"}],"memories":[]}"#,
        )
        .expect("parse");

        assert!(!should_repair_missing_file_operations(&input, &parsed));
        let kept = reject_missing_file_operations_for_file_requests(&input, parsed);

        assert_eq!(kept.file_operations.len(), 1);
        assert_eq!(kept.file_operations[0].path, "第2集.md");
    }

    #[test]
    fn model_file_operation_rejects_unconfigured_work_root() {
        let data_dir = temp_data_dir("file-ops-no-work-root");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let operation = ModelFileOperation {
            action: "writeFile".to_string(),
            library: "works".to_string(),
            path: "第2集.md".to_string(),
            new_name: None,
            content: Some("第二集".to_string()),
        };

        let result = apply_model_file_operation(&data_dir, &operation);

        assert!(!result.ok);
        assert!(result.message.contains("请先在作品库打开本地文件夹"));
        assert!(!crate::runtime::vault_root(&data_dir)
            .join("works")
            .join("第2集.md")
            .exists());
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
        let skill_path = skill_root.join("work-decompose").join("SKILL.md");

        let expanded = expand_context_items(
            &data_dir,
            Some(&skill_root),
            &[DialogueContextItem {
                kind: "tool".to_string(),
                label: "作品拆解".to_string(),
                value: format!("path:{}", skill_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("builtin skill context should be accepted");

        assert!(expanded[0].value.contains("# 拆解 Skill"));
        assert_eq!(
            expanded[0].relative_path.as_deref(),
            Some("skills/work-decompose/SKILL.md")
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
                    { "type": "output_text", "text": "{\"reply\":\"好\",\"edits\":[],\"fileOperations\":[],\"memories\":[]}" }
                ] }
            ]
        }"#;

        let content = read_model_response_text(body).expect("content exists");

        assert!(content.contains("\"reply\":\"好\""));
    }

    fn openai_compatible_test_settings(
        extra_env: std::collections::BTreeMap<String, String>,
    ) -> ActiveModelSettings {
        ActiveModelSettings {
            provider_id: "openai-compatible".to_string(),
            provider_name: "OpenAI-Compatible".to_string(),
            protocol: "openai-compatible".to_string(),
            auth_style: "api_key".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "secret".to_string(),
            model: "model-a".to_string(),
            model_id: "openai-compatible::model-a".to_string(),
            extra_env,
        }
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
    fn parse_cocreation_model_output_extracts_fenced_json() {
        let parsed = parse_cocreation_model_output(
            "好的。\n```json\n{\"reply\":\"现在的年月日时间是 2026-06-11。\",\"edits\":[],\"fileOperations\":[],\"memories\":[]}\n```",
        )
        .expect("fenced json reply");

        assert_eq!(parsed.reply, "现在的年月日时间是 2026-06-11。");
        assert!(parsed.edits.is_empty());
        assert!(parsed.memories.is_empty());
    }

    #[test]
    fn parse_cocreation_model_output_extracts_loose_fenced_json() {
        let parsed = parse_cocreation_model_output(
            "```json\n{\"reply\":\"这个替换涉及面比较大，我建议先逐处确认。\",\"edits\":[],\"fileOperations\":[],\"memories\":[]}\n",
        )
        .expect("loose fenced json reply");

        assert_eq!(parsed.reply, "这个替换涉及面比较大，我建议先逐处确认。");
        assert!(parsed.edits.is_empty());
        assert!(parsed.file_operations.is_empty());
        assert!(parsed.memories.is_empty());
    }

    #[test]
    fn parse_cocreation_model_output_accepts_plain_text_reply() {
        let parsed = parse_cocreation_model_output("现在的年月日时间是 2026-06-11。")
            .expect("plain text should be surfaced as reply");

        assert_eq!(parsed.reply, "现在的年月日时间是 2026-06-11。");
        assert!(parsed.edits.is_empty());
        assert!(parsed.file_operations.is_empty());
        assert!(parsed.memories.is_empty());
    }

    #[test]
    fn parse_cocreation_model_output_recovers_broken_json_payload_as_reply() {
        let parsed = parse_cocreation_model_output("```json\n{\"reply\":\"好\",\"edits\":[}\n```")
            .expect("broken structured output should not break chat");

        assert_eq!(parsed.reply, "好");
        assert!(parsed.edits.is_empty());
    }

    #[test]
    fn parse_cocreation_model_output_recovers_reply_with_unescaped_quotes() {
        let parsed = parse_cocreation_model_output(
            r#"{
                "reply": "这段可以重写成更生活化的版本，保留"肉片厚薄不均"这个细节。",
                "edits": []
            }"#,
        )
        .expect("malformed reply should be shown as chat text");

        assert!(parsed.reply.contains("这段可以重写成更生活化的版本"));
        assert!(parsed.reply.contains("肉片厚薄不均"));
        assert!(parsed.edits.is_empty());
    }

    #[test]
    fn parse_cocreation_model_output_recovers_malformed_json_edits() {
        let parsed = parse_cocreation_model_output(
            r#"```json
{
  "reply": "已根据要求，将剧本中所有牛魔王相关角色、称谓、关系及场景替换为沙和尚。",
  "edits": [
    {
      "target": "角色：牛魔王，铁扇公主，观音菩萨",
      "replacement": "角色：沙和尚，老伴一老沙，夫人一师妹/妻子，观音菩萨",
      "rationale": "调整角色称谓"
    },
    {
      "target": "灵山罗汉（双手合十，面无表情）：阿弥陀佛，牛魔，你具顽不灵，阻碍三界因果",
      "replacement": "灵山罗汉（双手合十，面无表情）：阿弥陀佛，沙和尚，你凡心未泯，妄动无明",
      "rationale": "调整对白"
    }
  ],
  "memories": []
}
```"#,
        )
        .expect("malformed code block should still recover edits");

        assert!(parsed.reply.contains("已根据要求"));
        assert_eq!(parsed.edits.len(), 2);
        assert_eq!(parsed.edits[0].target, "角色：牛魔王，铁扇公主，观音菩萨");
        assert!(parsed.edits[0].replacement.contains("沙和尚"));
        assert!(parsed.edits[1].replacement.contains("妄动无明"));
    }

    #[test]
    fn parse_cocreation_model_output_recovers_malformed_json_file_operations() {
        let parsed = parse_cocreation_model_output(
            r#"```json
{
  "reply": "已根据第1集剧情续写第2集，并新建到作品库。",
  "edits": [],
  "fileOperations": [
    {
      "action": "writeFile",
      "library": "works",
      "path": "works/第2集.md",
      "content": "第2集

开场：人物继续追查上一集留下的线索。
对白："这事不能拖。"
"
    }
  ],
  "memories": []
}
```"#,
        )
        .expect("malformed file operation should be recovered");

        assert!(parsed.reply.contains("第2集"));
        assert_eq!(parsed.file_operations.len(), 1);
        assert_eq!(parsed.file_operations[0].action, "writeFile");
        assert_eq!(parsed.file_operations[0].library, "works");
        assert_eq!(parsed.file_operations[0].path, "works/第2集.md");
        assert!(parsed.file_operations[0]
            .content
            .as_deref()
            .unwrap_or_default()
            .contains("开场"));
    }

    #[test]
    fn parse_cocreation_model_output_recovers_unclosed_structured_file_operation() {
        let parsed = parse_cocreation_model_output(
            r#"`json
{
  "reply": "第2集已根据第1集剧情续写完成。",
  "edits": [],
  "fileOperations": [
    {
      "action": "writeFile",
      "library": "works",
      "path": "第2集.docx",
      "content": "第2集\n\n开场：牛魔王踏上旅途。"
    }
  ],
  "memories": [
"#,
        )
        .expect("unclosed structured output should recover file operation");

        assert_eq!(parsed.reply, "第2集已根据第1集剧情续写完成。");
        assert_eq!(parsed.file_operations.len(), 1);
        assert_eq!(parsed.file_operations[0].action, "writeFile");
        assert_eq!(parsed.file_operations[0].library, "works");
        assert_eq!(parsed.file_operations[0].path, "第2集.docx");
        assert!(parsed.file_operations[0]
            .content
            .as_deref()
            .unwrap_or_default()
            .contains("开场"));
        assert!(!parsed.reply.contains("fileOperations"));
    }

    #[test]
    fn parse_cocreation_model_output_hides_unrecoverable_structured_payload() {
        let parsed = parse_cocreation_model_output(
            r#"``json
{
  "reply": "处理中",
  "edits": [],
  "fileOperations": [
    {
      "action": "writeFile",
      "library": "works"
"#,
        )
        .expect("unrecoverable structured output should be hidden");

        assert_eq!(parsed.reply, "处理中");
        assert!(parsed.file_operations.is_empty());
        assert!(!parsed.reply.contains("fileOperations"));
    }

    #[test]
    fn openai_compatible_body_can_omit_response_format_for_legacy_gateways() {
        let settings = openai_compatible_test_settings(std::collections::BTreeMap::new());
        let strict = openai_compatible_cocreation_body(&settings, "model-a", "prompt", true);
        assert_eq!(
            strict
                .get("response_format")
                .and_then(|value| value.get("type"))
                .and_then(serde_json::Value::as_str),
            Some("json_object")
        );

        let fallback = openai_compatible_cocreation_body(&settings, "model-a", "prompt", false);
        assert!(fallback.get("response_format").is_none());
    }

    #[test]
    fn openai_compatible_body_uses_explicit_env_options_only() {
        let mut extra_env = std::collections::BTreeMap::new();
        extra_env.insert(
            "WRIDIAN_OPENAI_COMPAT_MAX_TOKENS_FIELD".to_string(),
            "max_completion_tokens".to_string(),
        );
        extra_env.insert(
            "WRIDIAN_OPENAI_COMPAT_OMIT_TEMPERATURE".to_string(),
            "true".to_string(),
        );
        extra_env.insert(
            "WRIDIAN_OPENAI_COMPAT_THINKING".to_string(),
            "disabled".to_string(),
        );
        let settings = openai_compatible_test_settings(extra_env);
        let body = openai_compatible_cocreation_body(&settings, "model-a", "prompt", false);

        assert_eq!(
            body.get("max_completion_tokens")
                .and_then(serde_json::Value::as_u64),
            Some(2048)
        );
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert_eq!(
            body.get("thinking")
                .and_then(|value| value.get("type"))
                .and_then(serde_json::Value::as_str),
            Some("disabled")
        );
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
