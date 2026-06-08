use crate::runtime::{
    candidates_path, ensure_workspace, iso_timestamp, memory_folder_path, memory_tree_path,
    next_runtime_id, wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateMemoryCandidateInput {
    source_path: String,
    title: String,
    content: String,
    user_intent: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryCandidateActionInput {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateMemoryCandidateInput {
    id: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MemoryItem {
    id: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MemoryCandidate {
    id: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStateResponse {
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
