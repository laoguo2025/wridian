use crate::model_accounts::{read_custom_api_settings, StoredCustomApiSettings};
use crate::runtime::{
    candidates_path, ensure_workspace, iso_timestamp, memory_folder_path, memory_tree_path,
    next_runtime_id, wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;

const MEMORY_CATEGORIES: [&str; 6] = ["人物", "世界观", "剧情线", "风格", "禁区", "其他"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMemoryCandidateInput {
    source_path: String,
    title: String,
    content: String,
    user_intent: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryCandidateActionInput {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateMemoryCandidateInput {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractMemoryCandidatesInput {
    source_path: String,
    title: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryItem {
    id: String,
    #[serde(default = "default_memory_category")]
    category: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryCandidate {
    id: String,
    #[serde(default = "default_memory_category")]
    category: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStateResponse {
    memories: Vec<MemoryItem>,
    candidates: Vec<MemoryCandidate>,
    memory_folder_path: String,
}

#[tauri::command]
pub(crate) fn wridian_get_memory_state() -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    Ok(MemoryStateResponse {
        memories: read_memory_items(&data_dir)?,
        candidates: read_memory_candidates(&data_dir)?,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_create_memory_candidate(
    input: CreateMemoryCandidateInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut candidates = read_memory_candidates(&data_dir)?;
    let created_at = iso_timestamp();
    for text in propose_memory_texts(&input) {
        if candidates.iter().any(|candidate| candidate.text == text) {
            continue;
        }
        candidates.push(MemoryCandidate {
            id: next_runtime_id("candidate"),
            category: "其他".to_string(),
            text,
            source_path: input.source_path.clone(),
            title: input.title.clone(),
            created_at: created_at.clone(),
        });
    }
    write_memory_candidates(&data_dir, &candidates)?;
    Ok(MemoryStateResponse {
        memories: read_memory_items(&data_dir)?,
        candidates,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) async fn wridian_extract_memory_candidates(
    input: ExtractMemoryCandidatesInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let content = input.content.trim();
    if content.is_empty() {
        return Err("当前正文为空，无法提取记忆。".to_string());
    }
    let settings = read_custom_api_settings(&data_dir)?
        .ok_or_else(|| "请先在模型设置里保存第三方 API。".to_string())?;
    let extracted = extract_candidates_with_model(&settings, &input).await?;
    if extracted.is_empty() {
        return Err("模型没有提取到可用的候选记忆。".to_string());
    }

    let mut candidates = read_memory_candidates(&data_dir)?;
    let created_at = iso_timestamp();
    for extracted_candidate in extracted {
        if candidates
            .iter()
            .any(|candidate| candidate.text == extracted_candidate.text)
        {
            continue;
        }
        candidates.push(MemoryCandidate {
            id: next_runtime_id("candidate"),
            category: extracted_candidate.category,
            text: extracted_candidate.text,
            source_path: input.source_path.clone(),
            title: input.title.clone(),
            created_at: created_at.clone(),
        });
    }

    write_memory_candidates(&data_dir, &candidates)?;
    Ok(MemoryStateResponse {
        memories: read_memory_items(&data_dir)?,
        candidates,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_update_memory_candidate(
    input: UpdateMemoryCandidateInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let text = input.text.trim().to_string();
    if text.is_empty() {
        return Err("候选记忆不能为空。".to_string());
    }
    let mut candidates = read_memory_candidates(&data_dir)?;
    let candidate = candidates
        .iter_mut()
        .find(|candidate| candidate.id == input.id)
        .ok_or_else(|| "待确认记忆不存在。".to_string())?;
    candidate.text = text;
    write_memory_candidates(&data_dir, &candidates)?;
    Ok(MemoryStateResponse {
        memories: read_memory_items(&data_dir)?,
        candidates,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_accept_memory_candidate(
    input: MemoryCandidateActionInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut candidates = read_memory_candidates(&data_dir)?;
    let index = candidates
        .iter()
        .position(|candidate| candidate.id == input.id)
        .ok_or_else(|| "待确认记忆不存在。".to_string())?;
    let candidate = candidates.remove(index);
    let mut memories = read_memory_items(&data_dir)?;
    if !memories.iter().any(|memory| memory.text == candidate.text) {
        memories.push(MemoryItem {
            id: next_runtime_id("memory"),
            category: candidate.category,
            text: candidate.text,
            source_path: candidate.source_path,
            title: candidate.title,
            created_at: candidate.created_at,
        });
    }
    write_memory_items(&data_dir, &memories)?;
    write_memory_candidates(&data_dir, &candidates)?;
    Ok(MemoryStateResponse {
        memories,
        candidates,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_ignore_memory_candidate(
    input: MemoryCandidateActionInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut candidates = read_memory_candidates(&data_dir)?;
    candidates.retain(|candidate| candidate.id != input.id);
    write_memory_candidates(&data_dir, &candidates)?;
    Ok(MemoryStateResponse {
        memories: read_memory_items(&data_dir)?,
        candidates,
        memory_folder_path: memory_folder_path(&data_dir).to_string_lossy().into_owned(),
    })
}

fn read_memory_items(data_dir: &Path) -> Result<Vec<MemoryItem>, String> {
    let path = memory_tree_path(data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|error| format!("记忆树读取失败：{error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("记忆树格式损坏：{error}"))?;
    if let Some(items) = value.get("memories") {
        serde_json::from_value(items.clone())
            .map_err(|error| format!("记忆树条目格式损坏：{error}"))
    } else {
        Ok(Vec::new())
    }
}

fn write_memory_items(data_dir: &Path, memories: &[MemoryItem]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "memories": memories
    }))
    .map_err(|error| error.to_string())?;
    fs::write(memory_tree_path(data_dir), content)
        .map_err(|error| format!("记忆树写入失败：{error}"))
}

fn read_memory_candidates(data_dir: &Path) -> Result<Vec<MemoryCandidate>, String> {
    let path = candidates_path(data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("待确认记忆读取失败：{error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("待确认记忆格式损坏：{error}"))?;
    if let Some(items) = value.get("items") {
        serde_json::from_value(items.clone())
            .map_err(|error| format!("待确认记忆条目格式损坏：{error}"))
    } else {
        Ok(Vec::new())
    }
}

fn write_memory_candidates(data_dir: &Path, candidates: &[MemoryCandidate]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "items": candidates
    }))
    .map_err(|error| error.to_string())?;
    fs::write(candidates_path(data_dir), content)
        .map_err(|error| format!("待确认记忆写入失败：{error}"))
}

fn propose_memory_texts(input: &CreateMemoryCandidateInput) -> Vec<String> {
    let mut texts = Vec::new();
    let user_intent = compact_text(&input.user_intent, 120);
    let content_signal = extract_content_signal(&input.content);
    if !user_intent.is_empty() {
        texts.push(format!("{}：用户希望处理“{}”。", input.title, user_intent));
    }
    if !content_signal.is_empty() {
        texts.push(format!(
            "{}：当前正文线索是“{}”。",
            input.title, content_signal
        ));
    }
    texts
}

fn extract_content_signal(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .last()
        .map(|line| compact_text(line, 120))
        .unwrap_or_default()
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_chars).collect()
}

#[derive(Debug, Deserialize)]
struct ModelMemoryCandidateResponse {
    items: Vec<ModelMemoryCandidate>,
}

#[derive(Debug, Deserialize)]
struct ModelMemoryCandidate {
    category: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtractedMemoryCandidate {
    category: String,
    text: String,
}

fn default_memory_category() -> String {
    "其他".to_string()
}

fn normalize_memory_category(category: &str) -> String {
    let trimmed = category.trim();
    if MEMORY_CATEGORIES.contains(&trimmed) {
        trimmed.to_string()
    } else {
        "其他".to_string()
    }
}

fn parse_model_memory_candidates(output: &str) -> Result<Vec<ExtractedMemoryCandidate>, String> {
    let parsed: ModelMemoryCandidateResponse = serde_json::from_str(output.trim())
        .map_err(|error| format!("模型记忆提取结果不是有效 JSON：{error}"))?;
    let mut candidates = Vec::new();
    for item in parsed.items {
        let text = item.text.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            continue;
        }
        let category = normalize_memory_category(&item.category.unwrap_or_default());
        if candidates
            .iter()
            .any(|candidate: &ExtractedMemoryCandidate| candidate.text == text)
        {
            continue;
        }
        candidates.push(ExtractedMemoryCandidate { category, text });
    }
    Ok(candidates)
}

async fn extract_candidates_with_model(
    settings: &StoredCustomApiSettings,
    input: &ExtractMemoryCandidatesInput,
) -> Result<Vec<ExtractedMemoryCandidate>, String> {
    let url = format!("{}/chat/completions", settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("记忆提取客户端创建失败：{error}"))?;
    let response = client
        .post(url)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": settings.model,
            "messages": [
                {
                    "role": "system",
                    "content": memory_extraction_system_prompt()
                },
                {
                    "role": "user",
                    "content": format!(
                        "文件标题：{}\n来源路径：{}\n\n正文：\n{}",
                        input.title,
                        input.source_path,
                        compact_text(&input.content, 6000)
                    )
                }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|error| format!("记忆提取请求失败：{error}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "记忆提取失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let content = read_chat_completion_content(&body)?;
    parse_model_memory_candidates(&content)
}

fn read_chat_completion_content(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("记忆提取响应格式损坏：{error}"))?;
    value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "记忆提取响应缺少 choices[0].message.content。".to_string())
}

fn memory_extraction_system_prompt() -> &'static str {
    r#"你是 Wridian 的写作记忆提取器。
请从正文中提取对后续写作有长期价值、需要用户确认后保存的候选记忆。
只提取稳定事实、人物设定、世界观设定、剧情线索、风格偏好、创作禁区。
不要总结全文，不要写建议，不要把临时措辞当成记忆。
输出必须是 JSON 对象，格式：
{"items":[{"category":"人物|世界观|剧情线|风格|禁区|其他","text":"一条可独立理解的中文候选记忆"}]}
最多输出 6 条；没有可用记忆时输出 {"items":[]}。"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_memory_candidates_keeps_valid_categories_and_texts() {
        let output = r#"{
            "items": [
                { "category": "人物", "text": "女主进入旧楼不是冲动，而是为了确认父亲线索。" },
                { "category": "未知", "text": "雾城旧楼区十年前发生过事故。" },
                { "category": "禁区", "text": "不要提前暴露凶手。" },
                { "category": "剧情线", "text": "" }
            ]
        }"#;

        let candidates = parse_model_memory_candidates(output).expect("model output parses");

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].category, "人物");
        assert_eq!(candidates[1].category, "其他");
        assert_eq!(candidates[2].text, "不要提前暴露凶手。");
    }

    #[test]
    fn read_chat_completion_content_reads_first_choice_message() {
        let body = r#"{
            "choices": [
                { "message": { "content": "{\"items\":[]}" } }
            ]
        }"#;

        let content = read_chat_completion_content(body).expect("content exists");

        assert_eq!(content, r#"{"items":[]}"#);
    }

    #[test]
    #[ignore = "requires WRIDIAN_TEST_API_BASE_URL, WRIDIAN_TEST_API_KEY, WRIDIAN_TEST_MODEL"]
    fn extract_candidates_with_configured_provider_returns_items() {
        let settings = StoredCustomApiSettings {
            base_url: std::env::var("WRIDIAN_TEST_API_BASE_URL")
                .expect("WRIDIAN_TEST_API_BASE_URL is required"),
            api_key: std::env::var("WRIDIAN_TEST_API_KEY")
                .expect("WRIDIAN_TEST_API_KEY is required"),
            model: std::env::var("WRIDIAN_TEST_MODEL").expect("WRIDIAN_TEST_MODEL is required"),
        };
        let input = ExtractMemoryCandidatesInput {
            source_path: "test://chapter.md".to_string(),
            title: "chapter.md".to_string(),
            content: "女主进入旧楼不是冲动，而是为了确认父亲留下的线索。雨夜场景不能提前暴露凶手。"
                .to_string(),
        };

        let candidates =
            tauri::async_runtime::block_on(extract_candidates_with_model(&settings, &input))
                .expect("provider returns parseable candidates");

        assert!(!candidates.is_empty());
    }
}
