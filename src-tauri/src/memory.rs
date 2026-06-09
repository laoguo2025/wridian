use crate::runtime::{ensure_workspace, knowledge_root, memory_folder_path, wridian_data_dir};
use crate::workspace::works_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryScopeInput {
    source_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStateResponse {
    memories: Vec<MemoryItem>,
    candidates: Vec<MemoryCandidate>,
    memory_folder_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryItem {
    id: String,
    category: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryCandidate {
    id: String,
    category: String,
    text: String,
    source_path: String,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryTreeResponse {
    roots: Vec<MemoryTreeNode>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryTreeNode {
    id: String,
    kind: String,
    label: String,
    description: String,
    path: Option<String>,
    content: Option<String>,
    children: Vec<MemoryTreeNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveMemoryTreeFileInput {
    path: String,
    content: String,
}

#[tauri::command]
pub(crate) fn wridian_get_memory_state() -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_memory_state_for_source(&data_dir, "")
}

#[tauri::command]
pub(crate) fn wridian_get_memory_state_for_source(
    input: MemoryScopeInput,
) -> Result<MemoryStateResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_memory_state_for_source(&data_dir, input.source_path.as_deref().unwrap_or_default())
}

#[tauri::command]
pub(crate) fn wridian_get_memory_tree() -> Result<MemoryTreeResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    read_memory_tree_files(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_save_memory_tree_file(
    input: SaveMemoryTreeFileInput,
) -> Result<MemoryTreeResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    save_memory_tree_file(&data_dir, &input.path, &input.content)?;
    read_memory_tree_files(&data_dir)
}

pub(crate) fn read_relevant_memory_snippets(
    data_dir: &Path,
    source_path: &str,
    _title: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    ensure_memory_tree_files(data_dir)?;
    let mut snippets = Vec::new();
    for file in context_files_for_source(data_dir, source_path)? {
        let content = fs::read_to_string(&file).unwrap_or_default();
        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        let label = file.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
        snippets.push(format!("【{}】{}", label, compact_markdown(trimmed, 900)));
    }
    snippets.truncate(limit);
    Ok(snippets)
}

fn read_memory_state_for_source(data_dir: &Path, source_path: &str) -> Result<MemoryStateResponse, String> {
    ensure_memory_tree_files(data_dir)?;
    let root = memory_tree_files_root(data_dir);
    let memories = context_files_for_source(data_dir, source_path)?
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let title = path.file_name()?.to_string_lossy().into_owned();
            Some(MemoryItem {
                id: path.to_string_lossy().into_owned(),
                category: "记忆树".to_string(),
                text,
                source_path: path.to_string_lossy().into_owned(),
                title,
                created_at: String::new(),
            })
        })
        .collect();
    Ok(MemoryStateResponse {
        memories,
        candidates: Vec::new(),
        memory_folder_path: root.to_string_lossy().into_owned(),
    })
}

fn read_memory_tree_files(data_dir: &Path) -> Result<MemoryTreeResponse, String> {
    ensure_memory_tree_files(data_dir)?;
    let root = memory_tree_files_root(data_dir);
    let works = works_root(data_dir)?;
    let knowledge = knowledge_root(data_dir);
    let mut roots = vec![
        MemoryTreeNode {
            id: "global".to_string(),
            kind: "layer".to_string(),
            label: "全局层".to_string(),
            description: "工作区规则、全局记忆和长期 awareness。".to_string(),
            path: None,
            content: None,
            children: vec![
                memory_file_node(&root, "global/AGENTS.md", "AGENTS.md", "长期工作区规则")?,
                memory_file_node(&root, "global/MEMORY.md", "MEMORY.md", "普通聊天全局记忆")?,
                memory_file_node(&root, "global/AWARENESS.md", "AWARENESS.md", "长期反思和意识记录")?,
            ],
        },
        MemoryTreeNode {
            id: "partner".to_string(),
            kind: "layer".to_string(),
            label: "伙伴层".to_string(),
            description: "共创伙伴的灵魂、用户画像、关系和伙伴记忆。".to_string(),
            path: None,
            content: None,
            children: vec![
                memory_file_node(&root, "partner/soul.md", "soul.md", "共创伙伴底层人格")?,
                memory_file_node(&root, "partner/user.md", "user.md", "用户画像和创作偏好")?,
                memory_file_node(&root, "partner/relationship.md", "relationship.md", "关系语气覆盖层")?,
                memory_file_node(&root, "partner/partnermemory.md", "partnermemory.md", "伙伴长期相处记忆")?,
            ],
        },
        MemoryTreeNode {
            id: "works".to_string(),
            kind: "layer".to_string(),
            label: "作品层".to_string(),
            description: "每个作品项目的规则、作品记忆、续接便签、episode 和 imprint。".to_string(),
            path: None,
            content: None,
            children: work_memory_tree_nodes(&root, &works)?,
        },
        MemoryTreeNode {
            id: "knowledge".to_string(),
            kind: "layer".to_string(),
            label: "知识层".to_string(),
            description: "知识库 cards/*.md，可被多个作品按需引用。".to_string(),
            path: Some(knowledge.to_string_lossy().into_owned()),
            content: None,
            children: knowledge_card_nodes(&knowledge)?,
        },
    ];
    if roots[2].children.is_empty() {
        roots[2].children.push(MemoryTreeNode {
            id: "works-empty".to_string(),
            kind: "empty".to_string(),
            label: "暂无作品项目".to_string(),
            description: "在作品库创建作品文件夹后，这里会出现对应的作品记忆文件。".to_string(),
            path: None,
            content: None,
            children: Vec::new(),
        });
    }
    Ok(MemoryTreeResponse { roots })
}

fn save_memory_tree_file(data_dir: &Path, path: &str, content: &str) -> Result<(), String> {
    ensure_memory_tree_files(data_dir)?;
    let target = PathBuf::from(path);
    let canonical_parent = target
        .parent()
        .ok_or_else(|| "记忆树文件路径无效。".to_string())?
        .canonicalize()
        .map_err(|error| format!("记忆树文件目录不存在：{error}"))?;
    let memory_root = memory_tree_files_root(data_dir)
        .canonicalize()
        .map_err(|error| format!("记忆树目录不存在：{error}"))?;
    let knowledge = knowledge_root(data_dir)
        .canonicalize()
        .map_err(|error| format!("知识库目录不存在：{error}"))?;
    if !canonical_parent.starts_with(&memory_root) && !canonical_parent.starts_with(&knowledge) {
        return Err("只能编辑记忆树或知识库里的 Markdown 文件。".to_string());
    }
    if target.extension().and_then(|extension| extension.to_str()) != Some("md") {
        return Err("记忆树只允许编辑 Markdown 文件。".to_string());
    }
    fs::write(target, content).map_err(|error| format!("记忆树文件写入失败：{error}"))
}

fn ensure_memory_tree_files(data_dir: &Path) -> Result<(), String> {
    let root = memory_tree_files_root(data_dir);
    for (relative, content) in default_memory_tree_files() {
        write_memory_tree_file_if_missing(&root.join(relative), content)?;
    }
    let works = works_root(data_dir)?;
    if works.is_dir() {
        for entry in fs::read_dir(&works).map_err(|error| format!("作品记忆树读取失败：{error}"))? {
            let entry = entry.map_err(|error| format!("作品记忆树读取失败：{error}"))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            ensure_project_memory_files(&root, &path, &name)?;
        }
    }
    Ok(())
}

fn default_memory_tree_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("global/AGENTS.md", "# AGENTS.md\n\n这里记录 Wridian 全局工作区规则、上下文边界和不可违反的长期协作原则。\n"),
        ("global/MEMORY.md", "# MEMORY.md\n\n这里记录普通聊天的全局长期记忆，不归属于任何单个作品。\n"),
        ("global/AWARENESS.md", "# AWARENESS.md\n\n这里记录长期反思、稳定变化和跨作品意识线索。\n"),
        ("partner/soul.md", "# soul.md\n\n这里定义 Wridian 作为共创伙伴的底层人格、判断原则和表达气质。\n"),
        ("partner/user.md", "# user.md\n\n这里记录用户画像、创作身份、工作节奏、语言偏好和审美偏好。\n"),
        ("partner/relationship.md", "# relationship.md\n\n这里记录你和 Wridian 的关系校准。用户的关系校准优先于默认人格。\n\n## Names\n\n## Register\n\n## Drift Warnings\n\n## Canonical Anchor\n"),
        ("partner/partnermemory.md", "# partnermemory.md\n\n这里记录 Wridian 与用户长期共创过程中形成的伙伴记忆。\n"),
    ]
}

fn ensure_project_memory_files(root: &Path, project_path: &Path, project_name: &str) -> Result<(), String> {
    let folder = project_memory_folder(root, project_path, project_name);
    let today = chrono_like_date();
    for (name, content) in [
        ("projectrules.md", format!("# projectrules.md\n\n作品：{}\n\n这里记录题材、风格、禁区、人物边界、世界观硬设定和分集/章节规则。\n", project_name)),
        ("workmemory.md", format!("# workmemory.md\n\n作品：{}\n\n这里记录只属于这个作品的长期记忆。\n", project_name)),
        ("caring-note.md", format!("# caring-note.md\n\n作品：{}\n\n这里记录下一轮创作要接住的短期续接便签。\n", project_name)),
    ] {
        write_memory_tree_file_if_missing(&folder.join(name), &content)?;
    }
    write_memory_tree_file_if_missing(
        &folder.join("episodes").join(format!("{today}.md")),
        &format!("# Episode - {today}\n\n## Key Moments\n\n## Behavior Signals\n\n## Candidate Memory Updates\n\n## Open Threads\n"),
    )?;
    write_memory_tree_file_if_missing(
        &folder.join("imprints").join(format!("{today}.md")),
        &format!("# Imprint - {today}\n\n这里记录真正值得留下的共创心迹；沉默是正常态。\n"),
    )
}

fn write_memory_tree_file_if_missing(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("记忆树目录创建失败：{error}"))?;
    }
    fs::write(path, content).map_err(|error| format!("记忆树文件创建失败：{error}"))
}

fn memory_tree_files_root(data_dir: &Path) -> PathBuf {
    memory_folder_path(data_dir).join("memory-tree")
}

fn memory_file_node(root: &Path, relative: &str, label: &str, description: &str) -> Result<MemoryTreeNode, String> {
    let path = root.join(relative);
    arbitrary_file_node(&path, label.to_string(), description.to_string())
}

fn arbitrary_file_node(path: &Path, label: String, description: String) -> Result<MemoryTreeNode, String> {
    Ok(MemoryTreeNode {
        id: path.to_string_lossy().into_owned(),
        kind: "file".to_string(),
        label,
        description,
        path: Some(path.to_string_lossy().into_owned()),
        content: Some(fs::read_to_string(path).map_err(|error| format!("记忆树文件读取失败：{error}"))?),
        children: Vec::new(),
    })
}

fn work_memory_tree_nodes(root: &Path, works: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    let mut nodes = Vec::new();
    if !works.is_dir() {
        return Ok(nodes);
    }
    for entry in fs::read_dir(works).map_err(|error| format!("作品记忆树读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("作品记忆树读取失败：{error}"))?;
        let project_path = entry.path();
        if !project_path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let folder = project_memory_folder(root, &project_path, &name);
        nodes.push(MemoryTreeNode {
            id: project_path.to_string_lossy().into_owned(),
            kind: "project".to_string(),
            label: name,
            description: "作品项目".to_string(),
            path: Some(project_path.to_string_lossy().into_owned()),
            content: None,
            children: vec![
                arbitrary_file_node(&folder.join("projectrules.md"), "projectrules.md".to_string(), "作品规则".to_string())?,
                arbitrary_file_node(&folder.join("workmemory.md"), "workmemory.md".to_string(), "作品长期记忆".to_string())?,
                arbitrary_file_node(&folder.join("caring-note.md"), "caring-note.md".to_string(), "短期续接便签".to_string())?,
                folder_node(&folder.join("episodes"), "episodes".to_string(), "日级内部 digest".to_string())?,
                folder_node(&folder.join("imprints"), "imprints".to_string(), "用户可见共创心迹".to_string())?,
            ],
        });
    }
    nodes.sort_by(|left, right| left.label.to_lowercase().cmp(&right.label.to_lowercase()));
    Ok(nodes)
}

fn folder_node(path: &Path, label: String, description: String) -> Result<MemoryTreeNode, String> {
    let mut children = Vec::new();
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|error| format!("记忆树目录读取失败：{error}"))? {
            let entry = entry.map_err(|error| format!("记忆树目录读取失败：{error}"))?;
            let child = entry.path();
            if child.extension().and_then(|extension| extension.to_str()) == Some("md") {
                children.push(arbitrary_file_node(
                    &child,
                    entry.file_name().to_string_lossy().into_owned(),
                    "Markdown 记忆文件".to_string(),
                )?);
            }
        }
    }
    children.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(MemoryTreeNode {
        id: path.to_string_lossy().into_owned(),
        kind: "folder".to_string(),
        label,
        description,
        path: Some(path.to_string_lossy().into_owned()),
        content: None,
        children,
    })
}

fn knowledge_card_nodes(knowledge: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    let mut nodes = Vec::new();
    collect_markdown_nodes(knowledge, &mut nodes)?;
    nodes.sort_by(|left, right| left.label.to_lowercase().cmp(&right.label.to_lowercase()));
    Ok(nodes)
}

fn collect_markdown_nodes(root: &Path, nodes: &mut Vec<MemoryTreeNode>) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("知识卡目录读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("知识卡目录读取失败：{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_nodes(&path, nodes)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
            nodes.push(arbitrary_file_node(
                &path,
                entry.file_name().to_string_lossy().into_owned(),
                "知识卡".to_string(),
            )?);
        }
    }
    Ok(())
}

fn context_files_for_source(data_dir: &Path, source_path: &str) -> Result<Vec<PathBuf>, String> {
    let root = memory_tree_files_root(data_dir);
    let mut files = vec![
        root.join("global").join("AGENTS.md"),
        root.join("global").join("MEMORY.md"),
        root.join("global").join("AWARENESS.md"),
        root.join("partner").join("soul.md"),
        root.join("partner").join("user.md"),
        root.join("partner").join("relationship.md"),
        root.join("partner").join("partnermemory.md"),
    ];
    if let Some((project_path, name)) = project_for_source(data_dir, source_path)? {
        let folder = project_memory_folder(&root, &project_path, &name);
        files.extend([
            folder.join("projectrules.md"),
            folder.join("workmemory.md"),
            folder.join("caring-note.md"),
        ]);
    }
    Ok(files)
}

fn project_for_source(data_dir: &Path, source_path: &str) -> Result<Option<(PathBuf, String)>, String> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(trimmed);
    let canonical = path.canonicalize().unwrap_or(path);
    let works = works_root(data_dir)?.canonicalize().unwrap_or(works_root(data_dir)?);
    if !canonical.starts_with(&works) {
        return Ok(None);
    }
    let Ok(relative) = canonical.strip_prefix(&works) else {
        return Ok(None);
    };
    let Some(first) = relative.components().next() else {
        return Ok(None);
    };
    let name = first.as_os_str().to_string_lossy().into_owned();
    let project_path = works.join(&name);
    if project_path.is_dir() {
        Ok(Some((project_path, name)))
    } else {
        Ok(None)
    }
}

fn project_memory_folder(root: &Path, project_path: &Path, project_name: &str) -> PathBuf {
    root.join("works").join(format!(
        "{}-{}",
        sanitize_markdown_file_name(project_name),
        stable_scope_id(&project_path.to_string_lossy())
    ))
}

fn compact_markdown(text: &str, max_chars: usize) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max_chars).collect()
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

fn stable_scope_id(value: &str) -> String {
    let mut hash: u64 = 1469598103934665603;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{hash:x}")
}

fn chrono_like_date() -> String {
    let seconds = crate::runtime::iso_timestamp().parse::<i64>().unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    civil_date_from_days(days)
}

fn civil_date_from_days(days_since_epoch: i64) -> String {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02}")
}
