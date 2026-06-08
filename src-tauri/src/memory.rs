use crate::model_accounts::{read_custom_api_settings, StoredCustomApiSettings};
use crate::runtime::{
    candidates_path, ensure_workspace, iso_timestamp, memory_folder_path, memory_tree_path,
    memory_wiki_root, next_runtime_id, wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchMemoryWikiInput {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryWikiSearchResult {
    kind: String,
    path: String,
    score: f64,
    snippet: String,
    title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryGraphResponse {
    nodes: Vec<MemoryGraphNode>,
    edges: Vec<MemoryGraphEdge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryGraphNode {
    id: String,
    kind: String,
    path: String,
    title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryGraphEdge {
    from: String,
    to: String,
    label: String,
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
pub(crate) fn wridian_ingest_memory_wiki() -> Result<MemoryGraphResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let memories = read_memory_items(&data_dir)?;
    write_memory_wiki(&data_dir, &memories)?;
    read_memory_graph(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_rebuild_memory_wiki_index() -> Result<MemoryGraphResponse, String> {
    wridian_ingest_memory_wiki()
}

#[tauri::command]
pub(crate) fn wridian_search_memory_wiki(input: SearchMemoryWikiInput) -> Result<Vec<MemoryWikiSearchResult>, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    search_memory_wiki(&data_dir, &input.query, input.limit.unwrap_or(8).min(30))
}

#[tauri::command]
pub(crate) fn wridian_get_memory_graph() -> Result<MemoryGraphResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_memory_graph(&data_dir)
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

pub(crate) fn read_relevant_memory_snippets(
    data_dir: &Path,
    source_path: &str,
    title: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    let memories = read_memory_items(data_dir)?;
    let mut matched = Vec::new();
    let mut fallback = Vec::new();

    for memory in memories {
        let snippet = format!("【{}】{}", memory.category, memory.text);
        if memory.source_path == source_path || memory.title == title {
            matched.push(snippet);
        } else {
            fallback.push(snippet);
        }
    }

    matched.extend(fallback);
    matched.truncate(limit);
    Ok(matched)
}

fn write_memory_items(data_dir: &Path, memories: &[MemoryItem]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "memories": memories
    }))
    .map_err(|error| error.to_string())?;
    fs::write(memory_tree_path(data_dir), content)
        .map_err(|error| format!("记忆树写入失败：{error}"))?;
    write_memory_wiki(data_dir, memories)
}

fn write_memory_wiki(data_dir: &Path, memories: &[MemoryItem]) -> Result<(), String> {
    let wiki = memory_wiki_root(data_dir);
    let sources = wiki.join("sources");
    let entities = wiki.join("entities");
    let concepts = wiki.join("concepts");
    let cache = wiki.join(".cache");
    for dir in [&wiki, &sources, &entities, &concepts, &cache] {
        fs::create_dir_all(dir).map_err(|error| format!("记忆 wiki 目录创建失败：{error}"))?;
    }

    let pages = build_wiki_pages(memories);
    let graph = build_wiki_graph(&pages);
    let mut index = String::from("# Wridian 记忆索引\n\n");
    let mut hot = String::from("# Hot Context\n\n");
    for memory in memories {
        let page = pages
            .iter()
            .find(|page| page.memory_id == memory.id)
            .ok_or_else(|| "记忆 wiki 页面生成失败。".to_string())?;
        let folder = match page.kind.as_str() {
            "entity" => &entities,
            "concept" => &concepts,
            _ => &sources,
        };
        let path = folder.join(&page.file_name);
        fs::write(&path, render_wiki_page(page, &graph))
            .map_err(|error| format!("记忆 wiki 写入失败：{error}"))?;
        index.push_str(&format!(
            "- [[{}]] `{}：{}`\n",
            page.title,
            page.kind,
            compact_text(&page.summary, 90)
        ));
    }

    for page in pages.iter().rev().take(12) {
        hot.push_str(&format!("- [[{}]]：{}\n", page.title, page.summary.trim()));
    }

    fs::write(wiki.join("index.md"), index).map_err(|error| format!("记忆索引写入失败：{error}"))?;
    fs::write(wiki.join("hot.md"), hot).map_err(|error| format!("Hot Context 写入失败：{error}"))?;
    fs::write(
        wiki.join("log.md"),
        format!(
            "# 记忆同步日志\n\n## {} ingest | confirmed memories\n- Pages: {}\n- Edges: {}\n- Strategy: sources/entities/concepts + wikilink graph + local BM25 cache.\n",
            iso_timestamp(),
            pages.len(),
            graph.edges.len()
        ),
    )
    .map_err(|error| format!("记忆日志写入失败：{error}"))?;
    fs::write(cache.join("index.json"), render_wiki_cache(&pages, &graph)?)
        .map_err(|error| format!("记忆检索缓存写入失败：{error}"))?;
    Ok(())
}

#[derive(Debug, Clone)]
struct WikiPage {
    category: String,
    file_name: String,
    kind: String,
    memory_id: String,
    path: String,
    source_path: String,
    summary: String,
    title: String,
    wikilinks: Vec<String>,
}

#[derive(Debug, Clone)]
struct WikiGraph {
    edges: Vec<(String, String, String)>,
    links_in: HashMap<String, Vec<String>>,
    links_out: HashMap<String, Vec<String>>,
}

fn build_wiki_pages(memories: &[MemoryItem]) -> Vec<WikiPage> {
    let title_by_id: HashMap<String, String> = memories
        .iter()
        .map(|memory| (memory.id.clone(), wiki_title(memory)))
        .collect();
    memories
        .iter()
        .map(|memory| {
            let kind = match memory.category.as_str() {
                "人物" => "entity",
                "其他" => "source",
                _ => "concept",
            }
            .to_string();
            let title = wiki_title(memory);
            let folder = match kind.as_str() {
                "entity" => "entities",
                "concept" => "concepts",
                _ => "sources",
            };
            let wikilinks = memories
                .iter()
                .filter(|other| other.id != memory.id)
                .filter(|other| memory.text.contains(&other.title) || other.text.contains(&memory.title))
                .filter_map(|other| title_by_id.get(&other.id).cloned())
                .take(8)
                .collect::<Vec<_>>();
            let file_name = format!("{}.md", sanitize_markdown_file_name(&title));
            WikiPage {
                category: memory.category.clone(),
                file_name: file_name.clone(),
                kind,
                memory_id: memory.id.clone(),
                path: format!("{folder}/{file_name}"),
                source_path: memory.source_path.clone(),
                summary: memory.text.trim().to_string(),
                title,
                wikilinks,
            }
        })
        .collect()
}

fn build_wiki_graph(pages: &[WikiPage]) -> WikiGraph {
    let titles: HashSet<String> = pages.iter().map(|page| page.title.clone()).collect();
    let mut edges = Vec::new();
    let mut links_in: HashMap<String, Vec<String>> = HashMap::new();
    let mut links_out: HashMap<String, Vec<String>> = HashMap::new();
    for page in pages {
        for target in &page.wikilinks {
            if !titles.contains(target) {
                continue;
            }
            edges.push((page.title.clone(), target.clone(), "related".to_string()));
            links_out.entry(page.title.clone()).or_default().push(target.clone());
            links_in.entry(target.clone()).or_default().push(page.title.clone());
        }
    }
    WikiGraph {
        edges,
        links_in,
        links_out,
    }
}

fn render_wiki_page(page: &WikiPage, graph: &WikiGraph) -> String {
    let related = page
        .wikilinks
        .iter()
        .map(|title| format!("  - \"[[{}]]\"", title))
        .collect::<Vec<_>>()
        .join("\n");
    let backlinks = graph
        .links_in
        .get(&page.title)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|title| format!("- [[{}]]", title))
        .collect::<Vec<_>>()
        .join("\n");
    let outgoing = graph
        .links_out
        .get(&page.title)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|title| format!("- [[{}]]", title))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\ntype: {}\nid: {}\ntitle: {}\ncategory: {}\nsource_path: {}\nstatus: seed\ncreated: {}\nupdated: {}\nrelated:\n{}\nsources:\n  - {}\n---\n\n# {}\n\n## Summary\n\n{}\n\n## Connections\n\n{}\n\n## Backlinks\n\n{}\n\n## Source\n\n- `{}`\n",
        page.kind,
        escape_yaml(&page.memory_id),
        escape_yaml(&page.title),
        escape_yaml(&page.category),
        escape_yaml(&page.source_path),
        iso_timestamp(),
        iso_timestamp(),
        if related.is_empty() { "  []".to_string() } else { related },
        escape_yaml(&page.source_path),
        page.title,
        page.summary,
        if outgoing.is_empty() { "- 暂无。".to_string() } else { outgoing },
        if backlinks.is_empty() { "- 暂无。".to_string() } else { backlinks },
        page.source_path
    )
}

fn render_wiki_cache(pages: &[WikiPage], graph: &WikiGraph) -> Result<String, String> {
    serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "updatedAt": iso_timestamp(),
        "pages": pages.iter().map(|page| json!({
            "id": page.memory_id,
            "kind": page.kind,
            "title": page.title,
            "path": page.path,
            "summary": page.summary,
            "sourcePath": page.source_path,
            "tokens": tokenize_for_search(&format!("{} {} {}", page.title, page.category, page.summary)),
        })).collect::<Vec<_>>(),
        "edges": graph.edges.iter().map(|(from, to, label)| json!({
            "from": from,
            "to": to,
            "label": label,
        })).collect::<Vec<_>>()
    }))
    .map_err(|error| error.to_string())
}

fn sanitize_markdown_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches(['.', ' ']).trim();
    if trimmed.is_empty() {
        "memory".to_string()
    } else {
        trimmed.to_string()
    }
}

fn wiki_title(memory: &MemoryItem) -> String {
    let signal = compact_text(&memory.text, 34);
    sanitize_markdown_file_name(&format!("{}-{}-{}", memory.category, memory.title, signal))
}

fn search_memory_wiki(
    data_dir: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<MemoryWikiSearchResult>, String> {
    let cache_path = memory_wiki_root(data_dir).join(".cache").join("index.json");
    if !cache_path.exists() {
        write_memory_wiki(data_dir, &read_memory_items(data_dir)?)?;
    }
    let content = fs::read_to_string(&cache_path).map_err(|error| format!("记忆检索缓存读取失败：{error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("记忆检索缓存格式损坏：{error}"))?;
    let query_tokens = tokenize_for_search(query);
    if query_tokens.is_empty() {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    if let Some(pages) = value.get("pages").and_then(serde_json::Value::as_array) {
        for page in pages {
            let tokens = page
                .get("tokens")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default();
            let overlap = query_tokens.iter().filter(|token| tokens.contains(*token)).count();
            if overlap == 0 {
                continue;
            }
            let score = (overlap as f64) / (query_tokens.len() as f64).sqrt().max(1.0);
            results.push(MemoryWikiSearchResult {
                kind: page.get("kind").and_then(serde_json::Value::as_str).unwrap_or("source").to_string(),
                path: page.get("path").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
                score,
                snippet: page.get("summary").and_then(serde_json::Value::as_str).unwrap_or_default().chars().take(180).collect(),
                title: page.get("title").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
            });
        }
    }
    results.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    Ok(results)
}

fn read_memory_graph(data_dir: &Path) -> Result<MemoryGraphResponse, String> {
    let cache_path = memory_wiki_root(data_dir).join(".cache").join("index.json");
    if !cache_path.exists() {
        write_memory_wiki(data_dir, &read_memory_items(data_dir)?)?;
    }
    let content = fs::read_to_string(&cache_path).map_err(|error| format!("记忆图谱缓存读取失败：{error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("记忆图谱缓存格式损坏：{error}"))?;
    let mut nodes = Vec::new();
    if let Some(pages) = value.get("pages").and_then(serde_json::Value::as_array) {
        for page in pages {
            nodes.push(MemoryGraphNode {
                id: page.get("title").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
                kind: page.get("kind").and_then(serde_json::Value::as_str).unwrap_or("source").to_string(),
                path: page.get("path").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
                title: page.get("title").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
            });
        }
    }
    let mut edges = Vec::new();
    if let Some(cached_edges) = value.get("edges").and_then(serde_json::Value::as_array) {
        for edge in cached_edges {
            edges.push(MemoryGraphEdge {
                from: edge.get("from").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
                to: edge.get("to").and_then(serde_json::Value::as_str).unwrap_or_default().to_string(),
                label: edge.get("label").and_then(serde_json::Value::as_str).unwrap_or("related").to_string(),
            });
        }
    }
    Ok(MemoryGraphResponse { nodes, edges })
}

fn tokenize_for_search(text: &str) -> HashSet<String> {
    let lower = text.to_lowercase();
    let mut tokens = HashSet::new();
    for token in lower.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
        if token.chars().count() > 1 {
            tokens.insert(token.to_string());
        }
    }
    let cjk: Vec<char> = text.chars().filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch)).collect();
    for window in cjk.windows(2) {
        tokens.insert(window.iter().collect());
    }
    tokens
}

fn escape_yaml(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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
