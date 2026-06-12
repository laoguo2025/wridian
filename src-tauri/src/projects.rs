use crate::memory::read_project_continuity_memory;
use crate::path_safety::safe_child_path;
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::text_index::tokenize_mixed_set;
use crate::workspace::{
    is_supported_writing_file, read_active_work_root, read_workspace_text_content,
    resolved_knowledge_root, works_root,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RELEVANT_SCAN_FILES: usize = 800;
const MAX_RELEVANT_SCAN_DEPTH: usize = 8;
const MAX_RELEVANT_FILE_BYTES: u64 = 512 * 1024;

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
pub(crate) struct SelectProjectInput {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelevantNotesInput {
    source_path: String,
    content: String,
    library: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelevantNote {
    kind: String,
    path: String,
    relative_path: Option<String>,
    title: String,
    snippet: String,
    score: f64,
    has_outgoing_links: bool,
    has_backlinks: bool,
    reasons: Vec<String>,
}

struct RelevantCandidate {
    kind: &'static str,
    path: PathBuf,
    root: PathBuf,
}

struct RelevantCandidateContent {
    candidate: RelevantCandidate,
    content: String,
}

#[derive(Default)]
struct GraphSignals {
    concepts: HashSet<String>,
    sources: HashSet<String>,
}

#[tauri::command]
pub(crate) fn wridian_get_project_state() -> Result<ProjectState, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_project_state(&data_dir)
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
    let continuity_memory = read_project_continuity_memory(data_dir, &project.id, 6)?;
    let continuity_block = if continuity_memory.trim().is_empty() {
        "暂无续接记忆。".to_string()
    } else {
        continuity_memory
    };
    Ok(format!(
        "项目记忆：{}\n说明：{}\n项目系统提示：{}\n常驻来源：{}\n排除：{}\nURLs：{}\n作品续接记忆：\n{}",
        project.name,
        project.description,
        project.system_prompt,
        project.inclusions.join(", "),
        project.exclusions.join(", "),
        project.web_urls.join(", "),
        continuity_block
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
    fs::write(path, content).map_err(|error| format!("项目配置写入失败：{error}"))
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
    let source_path = PathBuf::from(input.source_path.trim());
    if input.library.as_deref() != Some("works") {
        return Ok(Vec::new());
    }
    let source_text = format!(
        "{}\n{}",
        input.content,
        input.query.as_deref().unwrap_or_default()
    );
    let source_terms = tokenize_mixed_set(&source_text);
    if source_terms.is_empty() {
        return Ok(Vec::new());
    }
    let source_signals = extract_graph_signals(&format!(
        "{}\n{}",
        input.content,
        input.query.as_deref().unwrap_or_default()
    ));
    let source_links = extract_wikilinks(&input.content);
    let candidates = collect_relevant_candidates(data_dir, false)?;
    let candidate_contents = read_relevant_candidate_contents(candidates)?;
    let mut scored = Vec::new();
    for candidate_content in candidate_contents {
        let candidate = candidate_content.candidate;
        let path = candidate.path;
        if path == source_path {
            continue;
        }
        if candidate.kind == "draft" && !project_allows_path(active_project, &path) {
            continue;
        }
        let content = candidate_content.content;
        let path_text = path.to_string_lossy().to_string();
        let candidate_terms = tokenize_mixed_set(&format!("{path_text}\n{content}"));
        let lexical_score = overlap_score(&source_terms, &candidate_terms);
        let common_terms = top_common_terms(&source_terms, &candidate_terms, 4);
        let candidate_links = extract_wikilinks(&content);
        let candidate_signals = extract_graph_signals(&content);
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
        let common_concepts =
            top_common_terms(&source_signals.concepts, &candidate_signals.concepts, 4);
        let common_sources =
            top_common_terms(&source_signals.sources, &candidate_signals.sources, 3);
        let link_score = if has_outgoing_links && has_backlinks {
            0.3
        } else if has_outgoing_links || has_backlinks {
            0.18
        } else {
            0.0
        };
        let graph_score =
            (common_concepts.len() as f64 * 0.14) + (common_sources.len() as f64 * 0.2);
        let score = lexical_score + link_score + graph_score;
        if score <= 0.0 {
            continue;
        }
        let reasons = relevant_note_reasons(
            &common_terms,
            has_outgoing_links,
            has_backlinks,
            &common_concepts,
            &common_sources,
        );
        if reasons.is_empty() {
            continue;
        }
        scored.push(RelevantNote {
            kind: candidate.kind.to_string(),
            path: path_text,
            relative_path: relative_candidate_path(&candidate.root, &path),
            title,
            snippet: best_snippet(&content, &source_terms),
            score,
            has_outgoing_links,
            has_backlinks,
            reasons,
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

fn read_relevant_candidate_contents(
    candidates: Vec<RelevantCandidate>,
) -> Result<Vec<RelevantCandidateContent>, String> {
    let mut values = Vec::new();
    let mut warnings = Vec::new();
    for candidate in candidates {
        match read_workspace_text_content(&candidate.path) {
            Ok(content) => values.push(RelevantCandidateContent { candidate, content }),
            Err(error) => {
                if warnings.len() < 8 {
                    warnings.push(format!("{}：{error}", candidate.path.to_string_lossy()));
                }
            }
        }
    }
    if !warnings.is_empty() && values.is_empty() {
        return Err(format!(
            "相关内容检索没有可读取的候选文件。已跳过：{}",
            warnings.join("；")
        ));
    }
    Ok(values)
}

fn collect_relevant_candidates(
    data_dir: &Path,
    include_knowledge: bool,
) -> Result<Vec<RelevantCandidate>, String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    if include_knowledge {
        let knowledge_root = resolved_knowledge_root(data_dir)?;
        if knowledge_root.is_dir() {
            let root = knowledge_root
                .canonicalize()
                .map_err(|error| format!("知识库目录解析失败：{error}"))?;
            for path in collect_writing_files(std::slice::from_ref(&root))? {
                let key = normalize_path_text(&path);
                if seen.insert(key) {
                    candidates.push(RelevantCandidate {
                        kind: "knowledge",
                        path,
                        root: root.clone(),
                    });
                }
            }
        }
    }
    let root = works_root(data_dir)?
        .canonicalize()
        .map_err(|error| format!("作品库目录解析失败：{error}"))?;
    if root.is_dir() {
        for path in collect_writing_files(std::slice::from_ref(&root))? {
            let key = normalize_path_text(&path);
            if seen.insert(key) {
                candidates.push(RelevantCandidate {
                    kind: "draft",
                    path,
                    root: root.clone(),
                });
            }
        }
    }
    Ok(candidates)
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

fn relative_candidate_path(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
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
        let Some(safe_path) = safe_child_path(root, &path, "相关稿件")? else {
            continue;
        };
        if safe_path.is_dir() {
            collect_writing_files_recursive(&safe_path, depth + 1, files)?;
        } else if is_supported_writing_file(&safe_path) {
            let metadata = fs::symlink_metadata(&safe_path).map_err(|error| {
                format!(
                    "相关稿件文件信息读取失败（{}）：{error}",
                    safe_path.to_string_lossy()
                )
            })?;
            if metadata.file_type().is_symlink() || metadata.len() > MAX_RELEVANT_FILE_BYTES {
                continue;
            }
            files.push(safe_path);
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

fn overlap_score(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let overlap = left.intersection(right).count() as f64;
    overlap / (left.len() as f64).sqrt().max(1.0)
}

fn top_common_terms(left: &HashSet<String>, right: &HashSet<String>, limit: usize) -> Vec<String> {
    let mut terms = left
        .intersection(right)
        .filter(|term| term.chars().count() > 1)
        .cloned()
        .collect::<Vec<_>>();
    terms.sort_by(|left, right| {
        right
            .chars()
            .count()
            .cmp(&left.chars().count())
            .then_with(|| left.cmp(right))
    });
    terms.truncate(limit);
    terms
}

fn relevant_note_reasons(
    common_terms: &[String],
    has_outgoing_links: bool,
    has_backlinks: bool,
    common_concepts: &[String],
    common_sources: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !common_terms.is_empty() {
        reasons.push(format!("同词：{}", common_terms.join("、")));
    }
    if has_backlinks {
        reasons.push("反链：链接当前稿件".to_string());
    }
    if has_outgoing_links {
        reasons.push("共同链接：当前稿件提及相关标题或链接".to_string());
    }
    if !common_concepts.is_empty() {
        reasons.push(format!("共同概念：{}", common_concepts.join("、")));
    }
    if !common_sources.is_empty() {
        reasons.push(format!("共同来源：{}", common_sources.join("、")));
    }
    reasons
}

fn extract_graph_signals(text: &str) -> GraphSignals {
    let mut signals = GraphSignals::default();
    signals.concepts.extend(extract_wikilinks(text));
    if let Some(frontmatter) = extract_frontmatter_block(text) {
        collect_frontmatter_terms(
            frontmatter,
            &["concept", "concepts", "entity", "entities", "tag", "tags"],
            &mut signals.concepts,
        );
        collect_frontmatter_terms(
            frontmatter,
            &["source", "sources", "origin", "origins"],
            &mut signals.sources,
        );
    }
    signals
}

fn extract_frontmatter_block(text: &str) -> Option<&str> {
    let trimmed = text.strip_prefix("---")?;
    let end = trimmed.find("\n---")?;
    Some(&trimmed[..end])
}

fn collect_frontmatter_terms(frontmatter: &str, keys: &[&str], output: &mut HashSet<String>) {
    let mut active = false;
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_lowercase();
            active = keys.iter().any(|candidate| *candidate == key);
            if active {
                insert_graph_terms(value, output);
            }
            continue;
        }
        if active && trimmed.starts_with('-') {
            insert_graph_terms(trimmed.trim_start_matches('-'), output);
        } else if !trimmed.starts_with('-') {
            active = false;
        }
    }
}

fn insert_graph_terms(value: &str, output: &mut HashSet<String>) {
    for link in extract_wikilinks(value) {
        output.insert(link);
    }
    let cleaned = value
        .trim()
        .trim_matches(|ch| matches!(ch, '[' | ']' | '"' | '\'' | ' '));
    for term in cleaned.split(',') {
        let term = term
            .trim()
            .trim_matches(|ch| matches!(ch, '[' | ']' | '"' | '\'' | ' '))
            .to_lowercase();
        if term.chars().count() > 1 && !term.contains("[[") {
            output.insert(term);
        }
    }
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

fn normalize_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
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
    fn relevant_notes_skips_unreadable_candidates() {
        let data_dir = temp_data_dir("read-error");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let works = crate::runtime::vault_root(&data_dir).join("works");
        fs::create_dir_all(&works).expect("create works");
        let source = works.join("source.md");
        let candidate = works.join("candidate.md");
        let readable = works.join("readable.md");
        fs::write(&source, "共同线索").expect("write source");
        fs::write(&candidate, [0xff, 0xfe, 0xfd]).expect("write invalid utf8 candidate");
        fs::write(&readable, "共同线索 可读内容").expect("write readable candidate");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "共同线索".to_string(),
                library: Some("works".to_string()),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        assert!(notes.iter().any(|note| note.path.ends_with("readable.md")));
        assert!(!notes.iter().any(|note| note.path.ends_with("candidate.md")));
    }

    #[test]
    fn relevant_notes_skips_oversized_candidates() {
        let data_dir = temp_data_dir("skip-large");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let works = crate::runtime::vault_root(&data_dir).join("works");
        fs::create_dir_all(&works).expect("create works");
        let source = works.join("source.md");
        let large = works.join("large.md");
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
                library: Some("works".to_string()),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        assert!(!notes.iter().any(|note| note.path.ends_with("large.md")));
    }

    #[test]
    fn relevant_notes_does_not_auto_mix_knowledge_cards() {
        let data_dir = temp_data_dir("knowledge-reasons");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let works = crate::runtime::vault_root(&data_dir).join("works");
        fs::create_dir_all(&works).expect("create works");
        let source = works.join("source.md");
        let knowledge_root = crate::runtime::default_knowledge_root(&data_dir);
        let card_dir = knowledge_root.join("03故事模型");
        fs::create_dir_all(&card_dir).expect("create card dir");
        let card = card_dir.join("流亡结构.md");
        fs::write(
            &source,
            "---\nconcepts:\n  - [[流亡]]\nsources:\n  - [[史料A]]\n---\n荒原线索",
        )
        .expect("write source");
        fs::write(
            &card,
            "---\nconcepts:\n  - [[流亡]]\nsources:\n  - [[史料A]]\n---\n荒原里的角色会隐瞒身份。",
        )
        .expect("write card");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: fs::read_to_string(&source).expect("read source"),
                library: Some("works".to_string()),
                query: Some("角色为什么隐瞒身份".to_string()),
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        assert!(notes
            .iter()
            .all(|note| !note.path.ends_with("流亡结构.md") && note.kind != "knowledge"));
    }

    #[test]
    fn relevant_notes_are_disabled_for_knowledge_sources() {
        let data_dir = temp_data_dir("knowledge-source-disabled");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let works = crate::runtime::vault_root(&data_dir).join("works");
        fs::create_dir_all(&works).expect("create works");
        let source = crate::runtime::default_knowledge_root(&data_dir).join("知识.md");
        fs::write(&source, "共同线索").expect("write source");
        fs::write(works.join("稿件.md"), "共同线索").expect("write draft");

        let notes = find_relevant_notes(
            &data_dir,
            &RelevantNotesInput {
                source_path: source.to_string_lossy().into_owned(),
                content: "共同线索".to_string(),
                library: Some("knowledge".to_string()),
                query: None,
                limit: Some(8),
            },
            None,
        )
        .expect("find notes");

        assert!(notes.is_empty());
    }
}
