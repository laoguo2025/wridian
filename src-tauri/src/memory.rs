use crate::runtime::{ensure_workspace, knowledge_root, memory_folder_path, wridian_data_dir};
use crate::workspace::works_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MEMORY_BRANCHES: [(&str, &str, &str); 9] = [
    ("sense", "SENSE.md", "自我意识机制"),
    ("user", "USER.md", "用户画像准则"),
    ("relationship", "RELATIONSHIP.md", "关系准则"),
    ("journey", "JOURNEY.md", "共创里程碑"),
    ("drama", "DRAMA.md", "剧本准则"),
    ("novel", "NOVEL.md", "小说准则"),
    ("knowledge", "KNOWLEDGE.md", "知识生产准则"),
    ("skill", "SKILL.md", "技能生产准则"),
    ("awareness", "AWARENESS.md", "反思机制"),
];

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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryLeafCandidate {
    id: String,
    branch: String,
    title: String,
    summary: String,
    reason: String,
    status: String,
    source_path: String,
    target_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveMemoryTreeFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProposeMemoryLeafInput {
    source_path: Option<String>,
    title: Option<String>,
    content: String,
    user_intent: Option<String>,
    draft_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlantMemoryLeafInput {
    branch: String,
    title: String,
    summary: String,
    reason: Option<String>,
    source_path: Option<String>,
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

#[tauri::command]
pub(crate) fn wridian_propose_memory_leaf(
    input: ProposeMemoryLeafInput,
) -> Result<Option<MemoryLeafCandidate>, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    ensure_memory_tree_files(&data_dir)?;
    propose_memory_leaf(&data_dir, input)
}

#[tauri::command]
pub(crate) fn wridian_plant_memory_leaf(
    input: PlantMemoryLeafInput,
) -> Result<MemoryTreeResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    ensure_memory_tree_files(&data_dir)?;
    plant_memory_leaf(&data_dir, input)?;
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
    let mut roots = vec![
        MemoryTreeNode {
            id: "totem".to_string(),
            kind: "root".to_string(),
            label: "图腾与树根".to_string(),
            description: "SOUL 是图腾，AGENTS 是树根，MEMORY 是主干。".to_string(),
            path: None,
            content: None,
            children: vec![
                memory_file_node(&root, "SOUL.md", "SOUL.md", "Wridian 的底层灵魂、价值观和共创人格。")?,
                memory_file_node(&root, "AGENTS.md", "AGENTS.md", "Wridian 如何行动、如何使用记忆树和何时询问用户。")?,
                memory_file_node(&root, "MEMORY.md", "MEMORY.md", "主干索引、上下文编译策略、分支说明和最近活跃叶子。")?,
            ],
        },
        MemoryTreeNode {
            id: "branches".to_string(),
            kind: "branches".to_string(),
            label: "九个分支".to_string(),
            description: "分支文件只写生长机制、准则和如何长叶子。".to_string(),
            path: None,
            content: None,
            children: branch_nodes(&root)?,
        },
        MemoryTreeNode {
            id: "leaves".to_string(),
            kind: "leaves".to_string(),
            label: "叶子".to_string(),
            description: "叶子才写具体生命记录、作品记忆、知识卡、技能和反思。".to_string(),
            path: None,
            content: None,
            children: leaf_nodes(&root)?,
        },
    ];
    if roots[2].children.is_empty() {
        roots[2].children.push(MemoryTreeNode {
            id: "leaves-empty".to_string(),
            kind: "empty".to_string(),
            label: "暂无叶子".to_string(),
            description: "确认候选叶子或手动新增 Markdown 后，这里会长出叶子。".to_string(),
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
    migrate_legacy_memory_files(data_dir, &root)?;
    for (branch, _, _) in MEMORY_BRANCHES {
        fs::create_dir_all(root.join("leaves").join(branch))
            .map_err(|error| format!("记忆树叶子目录创建失败：{error}"))?;
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
        ("SOUL.md", "# SOUL.md\n\nWridian 的图腾。这里定义底层灵魂、价值观和共创人格。稳定，不频繁变化。\n"),
        ("AGENTS.md", "# AGENTS.md\n\nWridian 的树根。这里定义如何行动、如何使用记忆树、哪些事必须问用户、哪些事不能自作主张。\n"),
        ("MEMORY.md", "# MEMORY.md\n\nWridian 记忆树主干。这里维护索引、上下文编译策略、分支说明和最近活跃叶子。\n\n## Context Compile\n\n- 先读 SOUL.md、AGENTS.md、MEMORY.md。\n- 再读命中分支的 branches/*.md。\n- 最后摘取最近、活跃、命中的 leaves。\n- 候选叶子必须经用户确认才写入 leaves。\n"),
        ("branches/SENSE.md", "# SENSE.md\n\n自我意识机制。定义什么样的 agent 自己想做的事可以长成叶子，且必须经过用户同意。\n"),
        ("branches/USER.md", "# USER.md\n\n用户画像准则。定义哪些创作之外的用户信息可以长成叶子，哪些不能写。\n"),
        ("branches/RELATIONSHIP.md", "# RELATIONSHIP.md\n\n关系准则。定义什么样的共处花絮值得记录，以及如何影响后续相处。\n"),
        ("branches/JOURNEY.md", "# JOURNEY.md\n\n共创里程碑。定义小节点如何沉淀，如何汇总成里程碑。\n"),
        ("branches/DRAMA.md", "# DRAMA.md\n\n剧本准则。定义剧本、短剧、分集、场景、对白相关记忆如何长叶。\n"),
        ("branches/NOVEL.md", "# NOVEL.md\n\n小说准则。定义小说、章节、人物、叙事、世界观相关记忆如何长叶。\n"),
        ("branches/KNOWLEDGE.md", "# KNOWLEDGE.md\n\n知识生产准则。定义知识卡、资料、设定、概念如何长叶。\n"),
        ("branches/SKILL.md", "# SKILL.md\n\n技能生产准则。定义可复用创作方法、工作流、提示词和工具能力如何长叶。\n"),
        ("branches/AWARENESS.md", "# AWARENESS.md\n\n反思机制。定义什么时候反思，什么时候沉默，反思如何反哺整棵树。\n"),
    ]
}

fn ensure_project_memory_files(root: &Path, project_path: &Path, project_name: &str) -> Result<(), String> {
    let branch = project_branch_for_path(project_path);
    let folder = root.join("leaves").join(branch).join(format!(
        "{}-{}",
        sanitize_markdown_file_name(project_name),
        stable_scope_id(&project_path.to_string_lossy())
    ));
    write_memory_tree_file_if_missing(
        &folder.join("project.md"),
        &format!("# {}\n\nbranch: {}\nsource: {}\nstatus: alive\n\n## 作品记忆\n\n这里记录只属于这个作品的长期记忆、规则、禁区、人物边界和续接线索。\n", project_name, branch, project_path.to_string_lossy()),
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

fn branch_nodes(root: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    MEMORY_BRANCHES
        .iter()
        .map(|(_, file, description)| {
            memory_file_node(
                root,
                &format!("branches/{file}"),
                file,
                description,
            )
        })
        .collect()
}

fn leaf_nodes(root: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    let mut nodes = Vec::new();
    for (branch, file, description) in MEMORY_BRANCHES {
        nodes.push(folder_node(
            &root.join("leaves").join(branch),
            branch.to_string(),
            format!("{description} 的具体叶子；规则见 branches/{file}。"),
        )?);
    }
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

fn migrate_legacy_memory_files(data_dir: &Path, root: &Path) -> Result<(), String> {
    copy_legacy_if_target_empty(&root.join("partner").join("soul.md"), &root.join("SOUL.md"))?;
    copy_legacy_if_target_empty(&root.join("global").join("AGENTS.md"), &root.join("AGENTS.md"))?;
    copy_legacy_if_target_empty(&root.join("global").join("MEMORY.md"), &root.join("MEMORY.md"))?;
    copy_legacy_if_target_empty(
        &root.join("partner").join("user.md"),
        &root.join("leaves").join("user").join("legacy-user.md"),
    )?;
    copy_legacy_if_target_empty(
        &root.join("partner").join("relationship.md"),
        &root.join("leaves").join("relationship").join("legacy-relationship.md"),
    )?;
    copy_legacy_if_target_empty(
        &root.join("partner").join("partnermemory.md"),
        &root.join("leaves").join("relationship").join("legacy-partnermemory.md"),
    )?;
    copy_legacy_if_target_empty(
        &root.join("global").join("AWARENESS.md"),
        &root.join("leaves").join("awareness").join("legacy-awareness.md"),
    )?;

    let knowledge = knowledge_root(data_dir);
    if knowledge.is_dir() {
        copy_markdown_folder_into_leaves(
            &knowledge,
            &root.join("leaves").join("knowledge").join("cards"),
        )?;
    }
    Ok(())
}

fn copy_legacy_if_target_empty(source: &Path, target: &Path) -> Result<(), String> {
    if !source.is_file() || target.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(source).unwrap_or_default();
    if content.trim().is_empty() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("记忆树迁移目录创建失败：{error}"))?;
    }
    fs::write(target, content).map_err(|error| format!("记忆树迁移写入失败：{error}"))
}

fn copy_markdown_folder_into_leaves(source_root: &Path, target_root: &Path) -> Result<(), String> {
    for entry in fs::read_dir(source_root).map_err(|error| format!("知识卡迁移读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("知识卡迁移读取失败：{error}"))?;
        let path = entry.path();
        let target = target_root.join(entry.file_name());
        if path.is_dir() {
            copy_markdown_folder_into_leaves(&path, &target)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
            copy_legacy_if_target_empty(&path, &target)?;
        }
    }
    Ok(())
}

fn context_files_for_source(data_dir: &Path, source_path: &str) -> Result<Vec<PathBuf>, String> {
    let root = memory_tree_files_root(data_dir);
    let mut files = vec![
        root.join("SOUL.md"),
        root.join("AGENTS.md"),
        root.join("MEMORY.md"),
        root.join("branches").join("USER.md"),
        root.join("branches").join("RELATIONSHIP.md"),
        root.join("branches").join("JOURNEY.md"),
    ];
    if let Some((project_path, name)) = project_for_source(data_dir, source_path)? {
        let branch = project_branch_for_path(&project_path);
        files.push(root.join("branches").join(if branch == "drama" { "DRAMA.md" } else { "NOVEL.md" }));
        files.push(root.join("leaves").join(branch).join(format!(
            "{}-{}",
            sanitize_markdown_file_name(&name),
            stable_scope_id(&project_path.to_string_lossy())
        )).join("project.md"));
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

fn project_branch_for_path(project_path: &Path) -> &'static str {
    let text = project_path.to_string_lossy().to_lowercase();
    if text.contains("剧") || text.contains("短剧") || text.contains("drama") || text.contains("screenplay") {
        "drama"
    } else {
        "novel"
    }
}

fn propose_memory_leaf(
    data_dir: &Path,
    input: ProposeMemoryLeafInput,
) -> Result<Option<MemoryLeafCandidate>, String> {
    let content = input.content.trim();
    let intent = input.user_intent.unwrap_or_default();
    if content.chars().count() < 18 && intent.chars().count() < 8 {
        return Ok(None);
    }
    let source_path = input.source_path.unwrap_or_default();
    let branch = infer_leaf_branch(&source_path, input.draft_kind.as_deref(), &content, &intent);
    let raw_title = input
        .title
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| candidate_title(&content, &intent));
    let title = format!("{} - {}", branch_label(branch), raw_title.trim());
    let summary = candidate_summary(&content, &intent);
    let reason = format!("命中 {} 分支：本轮内容包含可复用的创作/共处/知识信号，需用户确认后才写入长期叶子。", branch_label(branch));
    let slug = sanitize_markdown_file_name(&format!("{}-{}", chrono_like_date(), raw_title));
    let target_path = memory_tree_files_root(data_dir)
        .join("leaves")
        .join(branch)
        .join(format!("{slug}.md"));
    Ok(Some(MemoryLeafCandidate {
        id: format!("candidate:{branch}:{}", stable_scope_id(&format!("{source_path}:{summary}"))),
        branch: branch.to_string(),
        title,
        summary,
        reason,
        status: "candidate".to_string(),
        source_path,
        target_path: target_path.to_string_lossy().into_owned(),
    }))
}

fn plant_memory_leaf(data_dir: &Path, input: PlantMemoryLeafInput) -> Result<(), String> {
    let branch = normalize_branch(&input.branch)?;
    let title = input.title.trim();
    let summary = input.summary.trim();
    if title.is_empty() || summary.is_empty() {
        return Err("叶子标题和内容不能为空。".to_string());
    }
    let root = memory_tree_files_root(data_dir);
    let folder = root.join("leaves").join(branch);
    fs::create_dir_all(&folder).map_err(|error| format!("叶子目录创建失败：{error}"))?;
    let slug = sanitize_markdown_file_name(&format!("{}-{}", chrono_like_date(), title));
    let path = unique_markdown_path(&folder, &slug);
    let reason = input.reason.unwrap_or_default();
    let source = input.source_path.unwrap_or_default();
    let content = format!(
        "# {title}\n\nbranch: {branch}\nstatus: alive\ncreated: {}\nsource: {source}\n\n## Record\n\n{summary}\n\n## Why It Grew\n\n{}\n",
        crate::runtime::iso_timestamp(),
        if reason.trim().is_empty() { "用户确认这片候选叶子值得沉淀。" } else { reason.trim() }
    );
    fs::write(path, content).map_err(|error| format!("叶子写入失败：{error}"))
}

fn infer_leaf_branch<'a>(
    source_path: &str,
    draft_kind: Option<&str>,
    content: &str,
    intent: &str,
) -> &'a str {
    let text = format!("{source_path}\n{draft_kind:?}\n{content}\n{intent}").to_lowercase();
    if draft_kind == Some("screenplay") || text.contains("剧本") || text.contains("短剧") || text.contains("对白") || text.contains(".fountain") {
        "drama"
    } else if text.contains("小说") || text.contains("章节") || text.contains("人物") || text.contains("世界观") {
        "novel"
    } else if text.contains("知识") || text.contains("资料") || text.contains("设定") || text.contains("概念") {
        "knowledge"
    } else if text.contains("流程") || text.contains("技能") || text.contains("提示词") || text.contains("工具") {
        "skill"
    } else if text.contains("关系") || text.contains("语气") || text.contains("情绪") {
        "relationship"
    } else if text.contains("反思") || text.contains("意识") || text.contains("沉默") {
        "awareness"
    } else if text.contains("用户") || text.contains("偏好") || text.contains("习惯") {
        "user"
    } else if text.contains("里程碑") || text.contains("确定") || text.contains("完成") {
        "journey"
    } else {
        "journey"
    }
}

fn normalize_branch(branch: &str) -> Result<&'static str, String> {
    let lowered = branch.trim().to_lowercase();
    MEMORY_BRANCHES
        .iter()
        .find_map(|(key, _, _)| if *key == lowered { Some(*key) } else { None })
        .ok_or_else(|| "未知记忆分支。".to_string())
}

fn branch_label(branch: &str) -> &'static str {
    match branch {
        "sense" => "自我意识",
        "user" => "用户画像",
        "relationship" => "关系",
        "journey" => "共创里程碑",
        "drama" => "剧本",
        "novel" => "小说",
        "knowledge" => "知识",
        "skill" => "技能",
        "awareness" => "反思",
        _ => "记忆",
    }
}

fn candidate_title(content: &str, intent: &str) -> String {
    let source = if intent.trim().is_empty() { content } else { intent };
    source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(24)
        .collect::<String>()
        .trim()
        .trim_matches(['#', '-', '，', '。', ' '])
        .to_string()
}

fn candidate_summary(content: &str, intent: &str) -> String {
    let mut parts = Vec::new();
    if !intent.trim().is_empty() {
        parts.push(format!("用户意图：{}", compact_markdown(intent, 260)));
    }
    if !content.trim().is_empty() {
        parts.push(format!("现场内容：{}", compact_markdown(content, 520)));
    }
    parts.join("\n\n")
}

fn unique_markdown_path(folder: &Path, slug: &str) -> PathBuf {
    let mut path = folder.join(format!("{slug}.md"));
    if !path.exists() {
        return path;
    }
    for index in 2..100 {
        path = folder.join(format!("{slug}-{index}.md"));
        if !path.exists() {
            return path;
        }
    }
    folder.join(format!("{slug}-{}", stable_scope_id(slug)))
        .with_extension("md")
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
