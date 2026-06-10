use crate::file_lock::with_file_write_lock;
use crate::memory::{read_relevant_memory_snippets, write_memory_leaves, MemoryLeafDraft};
use crate::model_accounts::{
    anthropic_messages_url, is_openai_oauth_settings, openai_chat_completions_url,
    openai_oauth_account_id, read_active_model_settings, read_anthropic_response_text,
    read_gemini_response_text, response_body_summary, ActiveModelSettings,
};
use crate::projects::{active_project_model, read_active_project_context};
use crate::runtime::{ensure_workspace, iso_timestamp, runtime_root, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const BUDGET_ACTIVE_CONTEXT_CHARS: usize = 1200;
const BUDGET_CURRENT_DRAFT_CHARS: usize = 7000;
const BUDGET_EXPLICIT_ITEM_CHARS: usize = 1400;
const BUDGET_EXPLICIT_TOTAL_CHARS: usize = 4200;
const BUDGET_MEMORY_TOTAL_CHARS: usize = 2200;
const BUDGET_PROJECT_CONTEXT_CHARS: usize = 2600;
const BUDGET_SELECTION_CHARS: usize = 3000;
const BUDGET_HOT_KNOWLEDGE_ITEM_CHARS: usize = 760;
const BUDGET_HOT_KNOWLEDGE_TOTAL_CHARS: usize = 1800;
const BUDGET_TOOL_ITEM_CHARS: usize = 1800;
const BUDGET_TOOL_TOTAL_CHARS: usize = 2600;
const MAX_HOT_CACHE_ENTRIES: usize = 40;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateInput {
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
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories_used: Vec<String>,
    memories_written: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateEdit {
    target: String,
    replacement: String,
    rationale: Option<String>,
}

#[tauri::command]
pub(crate) async fn wridian_cocreate(mut input: CoCreateInput) -> Result<CoCreateResponse, String> {
    let user_input = input.user_input.trim();
    if user_input.is_empty() {
        return Err("对话输入不能为空。".to_string());
    }

    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    input.context_items = expand_context_items(&data_dir, &input.context_items)?;
    let hot_knowledge = read_hot_knowledge_snippets(&data_dir, &input);
    let _ = record_hot_cache_from_context(&data_dir, &input.context_items);
    let settings = read_active_model_settings(&data_dir, input.selected_model_id.as_deref())?
        .ok_or_else(|| "请先在模型设置里保存模型账户。".to_string())?;
    let memories_used =
        read_relevant_memory_snippets(&data_dir, &input.source_path, &input.title, 8)?;
    let active_context = read_active_context(&data_dir);
    let active_project_context = read_active_project_context(&data_dir)?;
    let project_model = active_project_model(&data_dir)?;
    let model_output = cocreate_with_model(
        &settings,
        project_model.as_deref(),
        &input,
        &memories_used,
        &active_context,
        &active_project_context,
        &hot_knowledge,
    )
    .await?;

    let memories_written = write_memory_leaves(&data_dir, &model_output.memories)?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    Ok(CoCreateResponse {
        reply: model_output.reply,
        edits: model_output.edits,
        memories_used,
        memories_written,
    })
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
    hot_knowledge: &str,
) -> Result<ParsedCoCreateResponse, String> {
    if is_openai_oauth_settings(settings) {
        return cocreate_with_openai_oauth(
            settings,
            project_model,
            input,
            memories,
            active_context,
            active_project_context,
            hot_knowledge,
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
                hot_knowledge,
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
                hot_knowledge,
            )
            .await
        }
        _ => {
            cocreate_with_openai_compatible(
                settings,
                project_model,
                input,
                memories,
                active_context,
                active_project_context,
                hot_knowledge,
            )
            .await
        }
    }
}

async fn cocreate_with_openai_oauth(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    hot_knowledge: &str,
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
            "input": build_cocreation_prompt(input, memories, active_context, active_project_context, hot_knowledge),
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
    hot_knowledge: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let url = openai_chat_completions_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let response = client
        .post(url)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": project_model.unwrap_or(&settings.model),
            "messages": [
                {
                    "role": "system",
                    "content": cocreation_system_prompt()
                },
                {
                    "role": "user",
                    "content": build_cocreation_prompt(input, memories, active_context, active_project_context, hot_knowledge)
                }
            ],
            "response_format": { "type": "json_object" },
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

async fn cocreate_with_anthropic(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    hot_knowledge: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let url = anthropic_messages_url(&settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let mut request = client.post(url).header("anthropic-version", "2023-06-01");
    request = if settings.auth_style == "oauth_external" {
        request
            .bearer_auth(&settings.api_key)
            .header("anthropic-beta", "claude-code-20250219,oauth-2025-04-20")
            .header("user-agent", "claude-cli/2.1.74 (external, cli)")
            .header("x-app", "cli")
    } else if settings.auth_style == "auth_token" {
        request.bearer_auth(&settings.api_key)
    } else {
        request.header("x-api-key", &settings.api_key)
    };
    let response = request
        .json(&json!({
            "model": project_model.unwrap_or(&settings.model),
            "system": cocreation_system_prompt(),
            "messages": [
                {
                    "role": "user",
                    "content": build_cocreation_prompt(input, memories, active_context, active_project_context, hot_knowledge)
                }
            ],
            "max_tokens": 2048,
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
    let content = read_anthropic_response_text(&body)?;
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

async fn cocreate_with_gemini(
    settings: &ActiveModelSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
    hot_knowledge: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let model = project_model.unwrap_or(&settings.model);
    let url = format!("{}/models/{}:generateContent", settings.base_url, model);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
    let mut request = client.post(if settings.auth_style == "oauth_external" {
        url
    } else {
        format!("{}?key={}", url, settings.api_key)
    });
    if settings.auth_style == "oauth_external" {
        request = request.bearer_auth(&settings.api_key);
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
                        { "text": build_cocreation_prompt(input, memories, active_context, active_project_context, hot_knowledge) }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
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

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct KnowledgeHotCache {
    schema_version: u32,
    entries: Vec<KnowledgeHotCacheEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct KnowledgeHotCacheEntry {
    path: String,
    relative_path: String,
    title: String,
    last_used: String,
    hits: usize,
    related_terms: Vec<String>,
}

fn knowledge_hot_cache_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("knowledge-hot-cache.json")
}

fn read_hot_knowledge_snippets(data_dir: &Path, input: &CoCreateInput) -> String {
    let knowledge_root = match resolved_knowledge_root(data_dir).and_then(|path| {
        path.canonicalize()
            .map_err(|error| format!("知识库目录解析失败：{error}"))
    }) {
        Ok(root) => root,
        Err(_) => return String::new(),
    };
    let cache = read_hot_cache(data_dir);
    if cache.entries.is_empty() {
        return String::new();
    }
    let explicit_paths = input
        .context_items
        .iter()
        .filter_map(|item| item.source_path.as_deref())
        .filter_map(|path| PathBuf::from(path).canonicalize().ok())
        .collect::<HashSet<_>>();
    let query = hot_cache_query_text(input);
    let mut scored = cache
        .entries
        .iter()
        .filter_map(|entry| {
            let path = PathBuf::from(&entry.path).canonicalize().ok()?;
            if !path.is_file()
                || !path.starts_with(&knowledge_root)
                || explicit_paths.contains(&path)
            {
                return None;
            }
            let score = hot_cache_entry_score(entry, &query);
            (score > 0).then_some((score, entry, path))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.hits.cmp(&left.1.hits))
    });

    let mut rendered = String::new();
    for (_, entry, path) in scored.into_iter().take(3) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let block = format!(
            "【hot｜{}｜{}】\n{}\n",
            entry.title,
            entry.relative_path,
            compact_text(&content, BUDGET_HOT_KNOWLEDGE_ITEM_CHARS)
        );
        if rendered.chars().count() + block.chars().count() > BUDGET_HOT_KNOWLEDGE_TOTAL_CHARS {
            break;
        }
        rendered.push_str(&block);
    }
    rendered.trim().to_string()
}

fn record_hot_cache_from_context(
    data_dir: &Path,
    items: &[DialogueContextItem],
) -> Result<(), String> {
    let knowledge_root = resolved_knowledge_root(data_dir)?
        .canonicalize()
        .map_err(|error| format!("知识库目录解析失败：{error}"))?;
    let mut cache = read_hot_cache(data_dir);
    cache.schema_version = 1;
    let now = iso_timestamp();

    for item in items.iter().filter(|item| item.kind.trim() != "tool") {
        let Some(source_path) = item.source_path.as_deref() else {
            continue;
        };
        let Ok(path) = PathBuf::from(source_path).canonicalize() else {
            continue;
        };
        if !path.is_file() || !path.starts_with(&knowledge_root) {
            continue;
        }
        let relative_path = path
            .strip_prefix(&knowledge_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let title = if item.label.trim().is_empty() {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_else(|| relative_path.clone())
        } else {
            item.label.trim().to_string()
        };
        let related_terms = hot_cache_terms(&format!("{title}\n{relative_path}\n{}", item.value));
        let canonical_path = path.to_string_lossy().into_owned();
        if let Some(entry) = cache
            .entries
            .iter_mut()
            .find(|entry| entry.path == canonical_path)
        {
            entry.title = title;
            entry.relative_path = relative_path;
            entry.last_used = now.clone();
            entry.hits = entry.hits.saturating_add(1);
            entry.related_terms = merge_hot_terms(&entry.related_terms, &related_terms);
        } else {
            cache.entries.push(KnowledgeHotCacheEntry {
                path: canonical_path,
                relative_path,
                title,
                last_used: now.clone(),
                hits: 1,
                related_terms,
            });
        }
    }

    cache.entries.sort_by(|left, right| {
        right
            .last_used
            .cmp(&left.last_used)
            .then_with(|| right.hits.cmp(&left.hits))
    });
    cache.entries.truncate(MAX_HOT_CACHE_ENTRIES);
    write_hot_cache(data_dir, &cache)
}

fn read_hot_cache(data_dir: &Path) -> KnowledgeHotCache {
    fs::read_to_string(knowledge_hot_cache_path(data_dir))
        .ok()
        .and_then(|content| serde_json::from_str::<KnowledgeHotCache>(&content).ok())
        .unwrap_or(KnowledgeHotCache {
            schema_version: 1,
            entries: Vec::new(),
        })
}

fn write_hot_cache(data_dir: &Path, cache: &KnowledgeHotCache) -> Result<(), String> {
    let path = knowledge_hot_cache_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("hot cache 目录创建失败：{error}"))?;
    }
    let content = serde_json::to_string_pretty(cache).map_err(|error| error.to_string())?;
    with_file_write_lock(data_dir, &path, || {
        fs::write(&path, content).map_err(|error| format!("hot cache 写入失败：{error}"))
    })
}

fn hot_cache_query_text(input: &CoCreateInput) -> String {
    let explicit = input
        .context_items
        .iter()
        .map(|item| {
            format!(
                "{} {}",
                item.label,
                item.relative_path.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n{}\n{}\n{}\n{}",
        input.title,
        input.user_input,
        input.selected_text.as_deref().unwrap_or(""),
        input.content.chars().take(1600).collect::<String>(),
        explicit
    )
}

fn hot_cache_entry_score(entry: &KnowledgeHotCacheEntry, query: &str) -> usize {
    let query = query.to_lowercase();
    let mut score = entry.hits.min(6);
    for term in &entry.related_terms {
        if term.chars().count() >= 2 && query.contains(term) {
            score += 10;
        }
    }
    for token in hot_cache_terms(&format!("{}\n{}", entry.title, entry.relative_path)) {
        if query.contains(&token) {
            score += 14;
        }
    }
    score
}

fn hot_cache_terms(text: &str) -> Vec<String> {
    let mut terms = HashSet::new();
    for token in text
        .split(|ch: char| !(ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&ch)))
        .map(|token| token.trim().to_lowercase())
    {
        if token.chars().count() >= 2 && token.chars().count() <= 32 {
            terms.insert(token);
        }
    }
    extract_wikilink_like_terms(text, &mut terms);
    let mut terms = terms.into_iter().collect::<Vec<_>>();
    terms.sort();
    terms.truncate(24);
    terms
}

fn extract_wikilink_like_terms(text: &str, terms: &mut HashSet<String>) {
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let term = rest[..end]
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches(".md")
            .trim_end_matches(".markdown")
            .to_lowercase();
        if term.chars().count() >= 2 {
            terms.insert(term);
        }
        rest = &rest[end + 2..];
    }
}

fn merge_hot_terms(current: &[String], next: &[String]) -> Vec<String> {
    let mut merged = current
        .iter()
        .chain(next.iter())
        .cloned()
        .collect::<HashSet<_>>();
    let mut terms = merged.drain().collect::<Vec<_>>();
    terms.sort();
    terms.truncate(24);
    terms
}

fn expand_context_items(
    data_dir: &Path,
    items: &[DialogueContextItem],
) -> Result<Vec<DialogueContextItem>, String> {
    items
        .iter()
        .map(|item| expand_context_item(data_dir, item))
        .collect()
}

fn expand_context_item(
    data_dir: &Path,
    item: &DialogueContextItem,
) -> Result<DialogueContextItem, String> {
    let Some(raw_path) = referenced_context_path(item) else {
        return Ok(item.clone());
    };
    let (path, relative_path) = resolve_allowed_context_file(data_dir, &raw_path)?;
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
    hot_knowledge: &str,
) -> String {
    let memories_block = render_memory_context(memories);
    let active_context_block = if active_context.is_empty() {
        "暂无当前现场。".to_string()
    } else {
        compact_text(active_context, BUDGET_ACTIVE_CONTEXT_CHARS)
    };
    let active_project_block = if active_project_context.trim().is_empty() {
        "未启用 Project Mode。".to_string()
    } else {
        compact_text(active_project_context, BUDGET_PROJECT_CONTEXT_CHARS)
    };
    let draft_kind = match input.draft_kind.as_deref() {
        Some("screenplay") => "短剧/剧本稿件",
        _ => "小说/散文稿件",
    };
    let explicit_context_block = render_context_items_by_slot(&input.context_items, false);
    let tool_context_block = render_context_items_by_slot(&input.context_items, true);
    let hot_knowledge_block = if hot_knowledge.trim().is_empty() {
        "无。".to_string()
    } else {
        hot_knowledge.trim().to_string()
    };

    let source_label = prompt_source_label(&input.source_path, &input.title);
    format!(
        "稿件类型：{}\n当前文件：{}\n来源路径：{}\n\n上下文编译顺序：当前稿件/选区 → Project Mode → 当前现场 → 记忆树 → hot cache 知识召回 → 显式知识卡/相关稿件 → 技能协议 → 用户请求。\n\n[1 当前稿件与选区]\n用户选中的片段：\n{}\n\n稿件内容：\n{}\n\n[2 Project Mode]\n{}\n\n[3 当前现场]\n{}\n\n[4 记忆树上下文]\n{}\n\n[5 Hot Cache 知识召回]\n{}\n\n[6 显式上下文]\n{}\n\n[7 技能协议]\n{}\n\n用户这次想要：\n{}",
        draft_kind,
        input.title,
        source_label,
        input
            .selected_text
            .as_deref()
            .map(|text| compact_text(text, BUDGET_SELECTION_CHARS))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "未选择片段。".to_string()),
        compact_text(&input.content, BUDGET_CURRENT_DRAFT_CHARS),
        active_project_block,
        active_context_block,
        memories_block,
        hot_knowledge_block,
        explicit_context_block,
        tool_context_block,
        input.user_input.trim()
    )
}

fn prompt_source_label(source_path: &str, title: &str) -> String {
    Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| (!title.trim().is_empty()).then(|| title.trim().to_string()))
        .unwrap_or_else(|| "当前稿件".to_string())
}

fn render_memory_context(memories: &[String]) -> String {
    if memories.is_empty() {
        return "暂无记忆树上下文。".to_string();
    }
    let mut rendered = String::new();
    for memory in memories {
        let item = format!("- {}\n", compact_text(memory, 420));
        if rendered.chars().count() + item.chars().count() > BUDGET_MEMORY_TOTAL_CHARS {
            rendered.push_str("- 其余记忆因预算限制未展开。\n");
            break;
        }
        rendered.push_str(&item);
    }
    rendered.trim().to_string()
}

fn render_context_items_by_slot(items: &[DialogueContextItem], tool_slot: bool) -> String {
    let filtered = items
        .iter()
        .filter(|item| (item.kind.trim() == "tool") == tool_slot)
        .collect::<Vec<_>>();
    render_context_items(
        &filtered,
        if tool_slot {
            BUDGET_TOOL_ITEM_CHARS
        } else {
            BUDGET_EXPLICIT_ITEM_CHARS
        },
        if tool_slot {
            BUDGET_TOOL_TOTAL_CHARS
        } else {
            BUDGET_EXPLICIT_TOTAL_CHARS
        },
    )
}

fn render_context_items(
    items: &[&DialogueContextItem],
    per_item_budget: usize,
    total_budget: usize,
) -> String {
    if items.is_empty() {
        return "无。".to_string();
    }
    let mut rendered = String::new();
    for item in items {
        let value = compact_text(&item.value, per_item_budget);
        if value.trim().is_empty() {
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
        let block = format!("{header}\n{value}\n");
        if rendered.chars().count() + block.chars().count() > total_budget {
            rendered.push_str("【预算】其余上下文因预算限制未展开。\n");
            break;
        }
        rendered.push_str(&block);
    }
    if rendered.trim().is_empty() {
        "无。".to_string()
    } else {
        rendered.trim().to_string()
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
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_chars).collect()
}

fn parse_cocreation_model_output(output: &str) -> Result<ParsedCoCreateResponse, String> {
    let parsed: ModelCoCreateResponse = serde_json::from_str(output.trim())
        .map_err(|error| format!("对话结果不是有效 JSON：{error}"))?;
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
必须输出 JSON 对象：
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
            "Project Mode：短剧项目",
            "",
        );

        assert!(prompt.contains("稿件内容"));
        assert!(prompt.contains("记忆树上下文"));
        assert!(prompt.contains("强化她进门前的动机"));
    }

    #[test]
    fn build_prompt_separates_selection_from_explicit_context_items() {
        let input = CoCreateInput {
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

        let prompt = build_cocreation_prompt(&input, &[], "", "", "");

        assert!(prompt.contains("用户选中的片段：\n她推开门"));
        assert!(prompt.contains("[6 显式上下文]"));
        assert!(prompt.contains("【memory｜人物卡｜人物卡.md】\n她怕黑，但不承认。"));
    }

    #[test]
    fn build_prompt_separates_tool_protocol_from_explicit_context_items() {
        let input = CoCreateInput {
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

        let prompt = build_cocreation_prompt(&input, &[], "", "", "");

        assert!(prompt
            .contains("[6 显式上下文]\n【memory｜知识卡｜03故事模型/知识卡.md】\n一条显式知识卡"));
        assert!(prompt.contains("[7 技能协议]\n【tool｜知识库运维】\nWridian 技能协议：知识库运维"));
    }

    #[test]
    fn build_prompt_does_not_send_absolute_source_path() {
        let input = CoCreateInput {
            source_path: "D:/private/vault/works/第一章.md".to_string(),
            title: "第一章.md".to_string(),
            content: "她推开门。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "润色".to_string(),
            selected_text: None,
            selected_model_id: None,
            context_items: Vec::new(),
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "", "");

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
    fn hot_cache_recalls_recent_related_knowledge_without_explicit_duplicate() {
        let data_dir = temp_data_dir("hot-cache");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let knowledge_root = crate::runtime::default_knowledge_root(&data_dir);
        let card_path = knowledge_root.join("03故事模型").join("反转钩子.md");
        fs::write(&card_path, "反转钩子需要先埋误导，再在场尾回收。").expect("write card");
        record_hot_cache_from_context(
            &data_dir,
            &[DialogueContextItem {
                kind: "memory".to_string(),
                label: "反转钩子".to_string(),
                value: "path".to_string(),
                source_path: Some(card_path.to_string_lossy().into_owned()),
                relative_path: Some("03故事模型/反转钩子.md".to_string()),
            }],
        )
        .expect("record hot cache");

        let recalled = read_hot_knowledge_snippets(
            &data_dir,
            &CoCreateInput {
                source_path: "demo://01.md".to_string(),
                title: "01.md".to_string(),
                content: "这一场需要一个反转钩子。".to_string(),
                draft_kind: Some("screenplay".to_string()),
                user_input: "加强场尾反转".to_string(),
                selected_text: None,
                selected_model_id: None,
                context_items: Vec::new(),
            },
        );

        assert!(recalled.contains("【hot｜反转钩子｜03故事模型/反转钩子.md】"));
        assert!(recalled.contains("先埋误导"));

        let duplicate_suppressed = read_hot_knowledge_snippets(
            &data_dir,
            &CoCreateInput {
                source_path: "demo://01.md".to_string(),
                title: "01.md".to_string(),
                content: "这一场需要一个反转钩子。".to_string(),
                draft_kind: Some("screenplay".to_string()),
                user_input: "加强场尾反转".to_string(),
                selected_text: None,
                selected_model_id: None,
                context_items: vec![DialogueContextItem {
                    kind: "memory".to_string(),
                    label: "反转钩子".to_string(),
                    value: "反转钩子需要先埋误导，再在场尾回收。".to_string(),
                    source_path: Some(card_path.to_string_lossy().into_owned()),
                    relative_path: Some("03故事模型/反转钩子.md".to_string()),
                }],
            },
        );

        assert!(duplicate_suppressed.is_empty());
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
}
