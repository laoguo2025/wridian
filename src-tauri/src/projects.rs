use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::workspace::{allowed_work_roots, is_supported_writing_file, read_active_work_root, works_root};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
            if trimmed.is_empty() { None } else { Some(trimmed) }
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
pub(crate) fn wridian_find_relevant_notes(input: RelevantNotesInput) -> Result<Vec<RelevantNote>, String> {
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
    let Some(project) = state.projects.iter().find(|project| project.id == active_id) else {
        return Ok(String::new());
    };
    Ok(format!(
        "Project Mode：{}\n说明：{}\n项目系统提示：{}\n常驻来源：{}\n排除：{}\nURLs：{}",
        project.name,
        project.description,
        project.system_prompt,
        project.inclusions.join(", "),
        project.exclusions.join(", "),
        project.web_urls.join(", ")
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
        let content = fs::read_to_string(path).map_err(|error| format!("项目配置读取失败：{error}"))?;
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
    runtime_root(data_dir).join("projects").join("projects.json")
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
        for entry in fs::read_dir(&root).map_err(|error| format!("作品项目目录读取失败：{error}"))? {
            let entry = entry.map_err(|error| format!("作品项目目录读取失败：{error}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || matches!(name.as_str(), ".wridian" | ".wridian-trash" | "node_modules") {
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
                description: existing.map(|project| project.description.clone()).unwrap_or_default(),
                model: existing.and_then(|project| project.model.clone()),
                system_prompt: existing
                    .map(|project| project.system_prompt.clone())
                    .filter(|prompt| !prompt.trim().is_empty())
                    .unwrap_or_else(|| "当前作品项目的常驻上下文来自作品文件夹和该作品独立记忆。".to_string()),
                inclusions: vec![id],
                exclusions: existing.map(|project| project.exclusions.clone()).unwrap_or_default(),
                web_urls: existing.map(|project| project.web_urls.clone()).unwrap_or_default(),
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
    let mut scored = Vec::new();
    for path in candidates {
        if path == source_path {
            continue;
        }
        if !project_allows_path(active_project, &path) {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        let path_text = path.to_string_lossy().to_string();
        let candidate_terms = tokenize_mixed(&format!("{path_text}\n{content}"));
        let lexical_score = overlap_score(&source_terms, &candidate_terms);
        let candidate_links = extract_wikilinks(&content);
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_text.clone());
        let title_key = title.trim_end_matches(".md").trim_end_matches(".markdown").to_lowercase();
        let has_outgoing_links = !source_links.is_disjoint(&candidate_links) || source_links.contains(&title_key);
        let has_backlinks = candidate_links.contains(&source_title_key(&source_path));
        let link_score = if has_outgoing_links && has_backlinks {
            0.3
        } else if has_outgoing_links || has_backlinks {
            0.18
        } else {
            0.0
        };
        let score = lexical_score + link_score;
        if score <= 0.0 {
            continue;
        }
        scored.push(RelevantNote {
            path: path_text,
            title,
            snippet: best_snippet(&content, &source_terms),
            score,
            has_outgoing_links,
            has_backlinks,
        });
    }
    scored.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(input.limit.unwrap_or(8).min(20));
    Ok(scored)
}

fn collect_writing_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in roots {
        collect_writing_files_recursive(root, &mut files)?;
    }
    Ok(files)
}

fn collect_writing_files_recursive(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("相关稿件目录读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("相关稿件目录读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || matches!(name.as_str(), "node_modules" | ".wridian-trash" | ".wridian") {
            continue;
        }
        if path.is_dir() {
            collect_writing_files_recursive(&path, files)?;
        } else if is_supported_writing_file(&path) {
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

fn overlap_score(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let overlap = left.intersection(right).count() as f64;
    overlap / (left.len() as f64).sqrt().max(1.0)
}

fn extract_wikilinks(text: &str) -> HashSet<String> {
    let mut links = HashSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let link = rest[..end].split('|').next().unwrap_or("").trim().to_lowercase();
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
            terms.iter().filter(|term| lower.contains(term.as_str())).count()
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
        .map(|ch| if ch.is_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' })
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
