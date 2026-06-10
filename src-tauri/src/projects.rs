use crate::file_lock::with_file_write_lock;
use crate::memory::read_project_compressed_memory;
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::workspace::{
    allowed_work_roots, is_supported_writing_file, read_active_work_root, works_root,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RELEVANT_SCAN_FILES: usize = 800;
const MAX_RELEVANT_SCAN_DEPTH: usize = 8;
const MAX_RELEVANT_FILE_BYTES: u64 = 512 * 1024;
const MAX_RELEVANT_CHUNKS_PER_FILE: usize = 48;
const MAX_RELEVANT_CHUNK_CHARS: usize = 900;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectConfig {
    id: String,
    name: String,
    description: String,
    model: Option<String>,
    system_prompt: String,
    inclusions: Vec<String>,
    exclusions: Vec<String>,
    web_urls: Vec<String>,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectState {
    active_project_id: Option<String>,
    projects: Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveProjectInput {
    id: Option<String>,
    name: String,
    description: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    inclusions: Option<Vec<String>>,
    exclusions: Option<Vec<String>>,
    web_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectProjectInput {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelevantNotesInput {
    source_path: String,
    content: String,
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelevantNote {
    path: String,
    title: String,
    snippet: String,
    score: f64,
    has_outgoing_links: bool,
    has_backlinks: bool,
}

#[tauri::command]
pub(crate) fn wridian_get_project_state() -> Result<ProjectState, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_project_state(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_save_project(input: SaveProjectInput) -> Result<ProjectState, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut state = read_project_state(&data_dir)?;
    let id = input
        .id
        .map(|id| sanitize_id(&id))
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| format!("project-{}", chrono_like_timestamp()));
    let project = ProjectConfig {
        id: id.clone(),
        name: input.name.trim().to_string(),
        description: input.description.unwrap_or_default().trim().to_string(),
        model: input.model.and_then(|model| {
            let trimmed = model.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        system_prompt: input.system_prompt.unwrap_or_default().trim().to_string(),
        inclusions: normalize_patterns(input.inclusions.unwrap_or_default()),
        exclusions: normalize_patterns(input.exclusions.unwrap_or_default()),
        web_urls: normalize_patterns(input.web_urls.unwrap_or_default()),
        updated_at: crate::runtime::iso_timestamp(),
    };
    if project.name.is_empty() {
        return Err("项目名称不能为空。".to_string());
    }
    if let Some(existing) = state.projects.iter_mut().find(|item| item.id == id) {
        *existing = project;
    } else {
        state.projects.push(project);
    }
    if state.active_project_id.is_none() {
        state.active_project_id = Some(id);
    }
    write_project_state(&data_dir, &state)?;
    Ok(state)
}

#[tauri::command]
pub(crate) fn wridian_select_project(input: SelectProjectInput) -> Result<ProjectState, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let mut state = read_project_state(&data_dir)?;
    if let Some(id) = input.id {
        if !state.projects.iter().any(|project| project.id == id) {
            return Err("项目不存在。".to_string());
        }
        state.active_project_id = Some(id);
    } else {
        state.active_project_id = None;
    }
    write_project_state(&data_dir, &state)?;
    Ok(state)
}

#[tauri::command]
pub(crate) fn wridian_find_relevant_notes(
    input: RelevantNotesInput,
) -> Result<Vec<RelevantNote>, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let state = read_project_state(&data_dir)?;
    let active_project = state
        .active_project_id
        .as_deref()
        .and_then(|id| state.projects.iter().find(|project| project.id == id));
    find_relevant_notes(&data_dir, &input, active_project)
}

pub(crate) fn read_active_project_context(data_dir: &Path) -> Result<String, String> {
    let state = read_project_state(data_dir)?;
    let Some(active_id) = state.active_project_id.as_deref() else {
        return Ok(String::new());
    };
    let Some(project) = state
        .projects
        .iter()
        .find(|project| project.id == active_id)
    else {
        return Ok(String::new());
    };
    let compressed_memory = read_project_compressed_memory(data_dir, &project.id)?;
    let compressed_block = if compressed_memory.trim().is_empty() {
        "暂无压缩记忆。".to_string()
    } else {
        compressed_memory
    };
    Ok(format!(
        "Project Mode：{}\n说明：{}\n项目系统提示：{}\n常驻来源：{}\n排除：{}\nURLs：{}\n作品压缩记忆：\n{}",
        project.name,
        project.description,
        project.system_prompt,
        project.inclusions.join(", "),
        project.exclusions.join(", "),
        project.web_urls.join(", "),
        compressed_block
    ))
}

pub(crate) fn active_project_model(data_dir: &Path) -> Result<Option<String>, String> {
    let state = read_project_state(data_dir)?;
    Ok(state
        .active_project_id
        .as_deref()
        .and_then(|id| state.projects.iter().find(|project| project.id == id))
        .and_then(|project| project.model.clone()))
}

fn read_project_state(data_dir: &Path) -> Result<ProjectState, String> {
    let path = project_state_path(data_dir);
    let persisted = if !path.exists() {
        ProjectState {
            active_project_id: None,
            projects: Vec::new(),
        }
    } else {
        let content =
            fs::read_to_string(path).map_err(|error| format!("项目配置读取失败：{error}"))?;
        serde_json::from_str(&content).map_err(|error| format!("项目配置格式损坏：{error}"))?
    };
    derive_project_state_from_work_folders(data_dir, persisted)
}

fn write_project_state(data_dir: &Path, state: &ProjectState) -> Result<(), String> {
    let path = project_state_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("项目目录创建失败：{error}"))?;
    }
    let content = serde_json::to_string_pretty(&json!(state)).map_err(|error| error.to_string())?;
    with_file_write_lock(data_dir, &path, || {
        fs::write(&path, content).map_err(|error| format!("项目配置写入失败：{error}"))
    })
}

fn project_state_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir)
        .join("projects")
        .join("projects.json")
}

fn derive_project_state_from_work_folders(
    data_dir: &Path,
    persisted: ProjectState,
) -> Result<ProjectState, String> {
    if read_active_work_root(data_dir)?.is_none() {
        return Ok(ProjectState {
            active_project_id: None,
            projects: Vec::new(),
        });
    }
    let root = works_root(data_dir)?;
    let mut projects = Vec::new();
    if root.is_dir() {
        for entry in
            fs::read_dir(&root).map_err(|error| format!("作品项目目录读取失败：{error}"))?
        {
            let entry = entry.map_err(|error| format!("作品项目目录读取失败：{error}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.')
                || matches!(
                    name.as_str(),
                    ".wridian" | ".wridian-trash" | "node_modules"
                )
            {
                continue;
            }
            let id = path
                .canonicalize()
                .unwrap_or(path.clone())
                .to_string_lossy()
                .into_owned();
            let existing = persisted.projects.iter().find(|project| project.id == id);
            projects.push(ProjectConfig {
                id: id.clone(),
                name,
                description: existing
                    .map(|project| project.description.clone())
                    .unwrap_or_default(),
                model: existing.and_then(|project| project.model.clone()),
                system_prompt: existing
                    .map(|project| project.system_prompt.clone())
                    .filter(|prompt| !prompt.trim().is_empty())
                    .unwrap_or_else(|| {
                        "当前作品项目的常驻上下文来自作品文件夹和该作品独立记忆。".to_string()
                    }),
                inclusions: vec![id],
                exclusions: existing
                    .map(|project| project.exclusions.clone())
                    .unwrap_or_default(),
                web_urls: existing
                    .map(|project| project.web_urls.clone())
                    .unwrap_or_default(),
                updated_at: existing
                    .map(|project| project.updated_at.clone())
                    .unwrap_or_else(crate::runtime::iso_timestamp),
            });
        }
    }
    projects.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    let active_project_id = persisted
        .active_project_id
        .filter(|id| projects.iter().any(|project| project.id == *id));
    Ok(ProjectState {
        active_project_id,
        projects,
    })
}

fn find_relevant_notes(
    data_dir: &Path,
    input: &RelevantNotesInput,
    active_project: Option<&ProjectConfig>,
) -> Result<Vec<RelevantNote>, String> {
    let roots = allowed_work_roots(data_dir)?;
    let source_path = PathBuf::from(input.source_path.trim());
    let source_text = input.query.as_deref().unwrap_or(&input.content);
    let source_terms = tokenize_mixed(source_text);
    if source_terms.is_empty() {
        return Ok(Vec::new());
    }
    let source_links = extract_wikilinks(&input.content);
    let candidates = collect_writing_files(&roots)?;
    let mut candidate_docs = Vec::new();
    let mut document_frequency: HashMap<String, usize> = HashMap::new();
    let mut chunk_count = 0usize;
    let mut total_chunk_terms = 0usize;

    for path in candidates {
        if path == source_path {
            continue;
        }
        if !project_allows_path(active_project, &path) {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("相关稿件读取失败（{}）：{error}", path.to_string_lossy()))?;
        let path_text = path.to_string_lossy().to_string();
        let candidate_links = extract_wikilinks(&content);
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_text.clone());
        let title_key = title
            .trim_end_matches(".md")
            .trim_end_matches(".markdown")
            .to_lowercase();
        let has_outgoing_links =
            !source_links.is_disjoint(&candidate_links) || source_links.contains(&title_key);
        let has_backlinks = candidate_links.contains(&source_title_key(&source_path));
        let link_score = if has_outgoing_links && has_backlinks {
            0.3
        } else if has_outgoing_links || has_backlinks {
            0.18
        } else {
            0.0
        };

        let chunks = build_relevant_chunks(&title, &path_text, &content, &source_terms);
        for chunk in &chunks {
            chunk_count += 1;
            total_chunk_terms += chunk.term_count;
            for term in chunk.frequencies.keys() {
                *document_frequency.entry(term.clone()).or_insert(0) += 1;
            }
        }
        candidate_docs.push(RelevantCandidateDoc {
            path_text,
            title,
            chunks,
            link_score,
            has_outgoing_links,
            has_backlinks,
        });
    }

    let avg_chunk_terms = if chunk_count == 0 {
        1.0
    } else {
        (total_chunk_terms as f64 / chunk_count as f64).max(1.0)
    };
    let mut scored = Vec::new();
    for doc in candidate_docs {
        let best_chunk = doc
            .chunks
            .iter()
            .map(|chunk| {
                (
                    bm25_chunk_score(
                        chunk,
                        &source_terms,
                        &document_frequency,
                        chunk_count,
                        avg_chunk_terms,
                    ),
                    chunk,
                )
            })
            .max_by(|left, right| {
                left.0
                    .partial_cmp(&right.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let (lexical_score, snippet) = best_chunk
            .map(|(score, chunk)| (score, best_snippet(&chunk.text, &source_terms)))
            .unwrap_or((0.0, String::new()));
        let score = lexical_score + doc.link_score;
        if score <= 0.0 {
            continue;
        }
        scored.push(RelevantNote {
            path: doc.path_text,
            title: doc.title,
            snippet,
            score,
            has_outgoing_links: doc.has_outgoing_links,
            has_backlinks: doc.has_backlinks,
        });
    }
    scored.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(input.limit.unwrap_or(8).min(20));
    Ok(scored)
}

#[derive(Debug)]
struct RelevantCandidateDoc {
    path_text: String,
    title: String,
    chunks: Vec<RelevantChunk>,
    link_score: f64,
    has_outgoing_links: bool,
    has_backlinks: bool,
}

#[derive(Debug)]
struct RelevantChunk {
    text: String,
    frequencies: HashMap<String, usize>,
    term_count: usize,
}

fn build_relevant_chunks(
    title: &str,
    path_text: &str,
    content: &str,
    source_terms: &HashSet<String>,
) -> Vec<RelevantChunk> {
    let mut chunks = split_relevant_chunks(content);
    if chunks.is_empty() {
        chunks.push(
            content
                .trim()
                .chars()
                .take(MAX_RELEVANT_CHUNK_CHARS)
                .collect(),
        );
    }
    chunks
        .into_iter()
        .take(MAX_RELEVANT_CHUNKS_PER_FILE)
        .map(|chunk| {
            let scoring_text = format!("{title}\n{path_text}\n{chunk}");
            let mut frequencies = HashMap::new();
            let tokens = tokenize_mixed_vec(&scoring_text);
            for token in tokens {
                if source_terms.contains(&token) {
                    *frequencies.entry(token).or_insert(0) += 1;
                }
            }
            let term_count = tokenize_mixed_vec(&chunk).len().max(1);
            RelevantChunk {
                text: chunk,
                frequencies,
                term_count,
            }
        })
        .collect()
}

fn split_relevant_chunks(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            push_relevant_chunk(&mut chunks, &mut current);
            continue;
        }
        let next_len = current.chars().count() + trimmed.chars().count() + 1;
        if next_len > MAX_RELEVANT_CHUNK_CHARS {
            push_relevant_chunk(&mut chunks, &mut current);
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(trimmed);
    }
    push_relevant_chunk(&mut chunks, &mut current);
    chunks
}

fn push_relevant_chunk(chunks: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        chunks.push(trimmed.chars().take(MAX_RELEVANT_CHUNK_CHARS).collect());
    }
    current.clear();
}

fn bm25_chunk_score(
    chunk: &RelevantChunk,
    source_terms: &HashSet<String>,
    document_frequency: &HashMap<String, usize>,
    chunk_count: usize,
    avg_chunk_terms: f64,
) -> f64 {
    if chunk.frequencies.is_empty() || chunk_count == 0 {
        return 0.0;
    }
    let k1 = 1.2;
    let b = 0.75;
    let chunk_len = chunk.term_count as f64;
    source_terms
        .iter()
        .filter_map(|term| {
            let tf = *chunk.frequencies.get(term)? as f64;
            let df = *document_frequency.get(term).unwrap_or(&0) as f64;
            let idf = (((chunk_count as f64 - df + 0.5) / (df + 0.5)) + 1.0).ln();
            let denominator = tf + k1 * (1.0 - b + b * chunk_len / avg_chunk_terms);
            Some(idf * (tf * (k1 + 1.0)) / denominator.max(0.0001))
        })
        .sum()
}

fn collect_writing_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in roots {
        collect_writing_files_recursive(root, 0, &mut files)?;
        if files.len() >= MAX_RELEVANT_SCAN_FILES {
            break;
        }
    }
    Ok(files)
}

fn collect_writing_files_recursive(
    root: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    if depth > MAX_RELEVANT_SCAN_DEPTH || files.len() >= MAX_RELEVANT_SCAN_FILES {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("相关稿件目录读取失败：{error}"))?
    {
        if files.len() >= MAX_RELEVANT_SCAN_FILES {
            break;
        }
        let entry = entry.map_err(|error| format!("相关稿件目录读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || matches!(
                name.as_str(),
                "node_modules" | ".wridian-trash" | ".wridian"
            )
        {
            continue;
        }
        if path.is_dir() {
            collect_writing_files_recursive(&path, depth + 1, files)?;
        } else if is_supported_writing_file(&path) {
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                format!(
                    "相关稿件文件信息读取失败（{}）：{error}",
                    path.to_string_lossy()
                )
            })?;
            if metadata.file_type().is_symlink() || metadata.len() > MAX_RELEVANT_FILE_BYTES {
                continue;
            }
            files.push(path);
        }
    }
    Ok(())
}

fn project_allows_path(project: Option<&ProjectConfig>, path: &Path) -> bool {
    let Some(project) = project else {
        return true;
    };
    let normalized = normalize_path_text(path);
    if project
        .exclusions
        .iter()
        .any(|pattern| !pattern.is_empty() && normalized.contains(&pattern.to_lowercase()))
    {
        return false;
    }
    project.inclusions.is_empty()
        || project
            .inclusions
            .iter()
            .any(|pattern| !pattern.is_empty() && normalized.contains(&pattern.to_lowercase()))
}

fn tokenize_mixed(text: &str) -> HashSet<String> {
    tokenize_mixed_vec(text).into_iter().collect()
}

fn tokenize_mixed_vec(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    for token in lower.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
        if token.chars().count() > 1 {
            tokens.push(token.to_string());
        }
    }
    let cjk: Vec<char> = text
        .chars()
        .filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect();
    for window in cjk.windows(2) {
        tokens.push(window.iter().collect());
    }
    tokens
}

fn extract_wikilinks(text: &str) -> HashSet<String> {
    let mut links = HashSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let link = rest[..end]
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if !link.is_empty() {
            links.insert(link);
        }
        rest = &rest[end + 2..];
    }
    links
}

fn source_title_key(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .trim_end_matches(".md")
        .trim_end_matches(".markdown")
        .to_lowercase()
}

fn best_snippet(content: &str, terms: &HashSet<String>) -> String {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .max_by_key(|line| {
            let lower = line.to_lowercase();
            terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count()
        })
        .unwrap_or_default()
        .chars()
        .take(180)
        .collect()
}

fn normalize_patterns(patterns: Vec<String>) -> Vec<String> {
    patterns
        .into_iter()
        .map(|pattern| pattern.trim().trim_matches('"').to_string())
        .filter(|pattern| !pattern.is_empty())
        .collect()
}

fn normalize_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn chrono_like_timestamp() -> String {
    crate::runtime::iso_timestamp()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-projects-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn relevant_notes_reports_candidate_read_errors() {
        let data_dir = temp_data_dir("read-error");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let vault = crate::runtime::vault_root(&data_dir);
        let source = vault.join("source.md");
        let candidate = vault.join("candidate.md");
        fs::write(&source, "共同线索").expect("write source");
        fs::write(&candidate, [0xff, 0xfe, 0xfd]).expect("write invalid utf8 candidate");

        let error = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "共同线索".to_string(),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect_err("candidate read error should be reported");

        assert!(error.contains("相关稿件读取失败"));
        assert!(error.contains("candidate.md"));
    }

    #[test]
    fn relevant_notes_skips_oversized_candidates() {
        let data_dir = temp_data_dir("skip-large");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let vault = crate::runtime::vault_root(&data_dir);
        let source = vault.join("source.md");
        let large = vault.join("large.md");
        fs::write(&source, "共同线索").expect("write source");
        fs::write(
            &large,
            format!(
                "共同线索 {}",
                "x".repeat((MAX_RELEVANT_FILE_BYTES as usize) + 1)
            ),
        )
        .expect("write large");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "共同线索".to_string(),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        assert!(!notes.iter().any(|note| note.path.ends_with("large.md")));
    }

    #[test]
    fn relevant_notes_returns_best_matching_chunk() {
        let data_dir = temp_data_dir("chunk-snippet");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let vault = crate::runtime::vault_root(&data_dir);
        let source = vault.join("source.md");
        let candidate = vault.join("candidate.md");
        fs::write(&source, "霜镜契约在第三幕回收").expect("write source");
        fs::write(
            &candidate,
            "普通段落只谈节奏。\n\n霜镜契约应该先作为误导物出现，再在第三幕回收。",
        )
        .expect("write candidate");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "霜镜契约在第三幕回收".to_string(),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        let note = notes
            .iter()
            .find(|note| note.path.ends_with("candidate.md"))
            .expect("candidate note");
        assert!(note.snippet.contains("霜镜契约"));
        assert!(note.snippet.contains("第三幕回收"));
    }

    #[test]
    fn relevant_notes_keeps_backlink_boost_without_text_overlap() {
        let data_dir = temp_data_dir("backlink");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let vault = crate::runtime::vault_root(&data_dir);
        let source = vault.join("source.md");
        let candidate = vault.join("candidate.md");
        fs::write(&source, "霜镜契约").expect("write source");
        fs::write(&candidate, "[[source]]\n\n另一张卡只通过反链关联。").expect("write candidate");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "霜镜契约".to_string(),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        let note = notes
            .iter()
            .find(|note| note.path.ends_with("candidate.md"))
            .expect("candidate note");
        assert!(note.has_backlinks);
        assert!(note.score > 0.0);
    }
}
