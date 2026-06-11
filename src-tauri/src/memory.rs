use crate::path_safety::safe_child_path;
use crate::runtime::{ensure_workspace, memory_folder_path, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MEMORY_BRANCHES: [(&str, &str, &str); 9] = [
    ("sense", "SENSE.md", "自我意识机制"),
    ("user", "USER.md", "用户画像准则"),
    ("relationship", "RELATIONSHIP.md", "关系准则"),
    ("journey", "JOURNEY.md", "创作里程碑"),
    ("drama", "DRAMA.md", "剧本准则"),
    ("novel", "NOVEL.md", "小说准则"),
    ("knowledge", "KNOWLEDGE.md", "知识库调用机制"),
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryLeafDraft {
    pub(crate) branch: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) reason: Option<String>,
    pub(crate) source_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveMemoryTreeFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteMemoryTreeFileInput {
    path: String,
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
pub(crate) fn wridian_delete_memory_tree_file(
    input: DeleteMemoryTreeFileInput,
) -> Result<MemoryTreeResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    delete_memory_tree_file(&data_dir, &input.path)?;
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
        let label = file
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        snippets.push(format!("【{}】{}", label, compact_markdown(trimmed, 900)));
    }
    snippets.truncate(limit);
    Ok(snippets)
}

pub(crate) fn read_project_continuity_memory(
    data_dir: &Path,
    project_id: &str,
    extra_leaf_limit: usize,
) -> Result<String, String> {
    ensure_memory_tree_files(data_dir)?;
    let project_path = PathBuf::from(project_id.trim());
    if !project_path.is_dir() {
        return Ok(String::new());
    }
    let folder = project_memory_folder(&memory_tree_files_root(data_dir), &project_path);
    let mut files = vec![folder.join("project.md"), folder.join("compressed.md")];
    let mut extra = Vec::new();
    collect_project_memory_leaf_files(&folder, &mut extra)?;
    extra.sort();
    extra.truncate(extra_leaf_limit);
    files.extend(extra);

    let mut blocks = Vec::new();
    for file in files {
        let content = match fs::read_to_string(&file) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("作品续接记忆读取失败：{error}")),
        };
        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        let label = project_memory_context_label(&file);
        blocks.push(format!("【{}】{}", label, compact_markdown(trimmed, 900)));
    }
    Ok(compact_markdown(&blocks.join("\n"), 2600))
}

fn read_memory_state_for_source(
    data_dir: &Path,
    source_path: &str,
) -> Result<MemoryStateResponse, String> {
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
                memory_file_node(
                    &root,
                    "SOUL.md",
                    "SOUL.md",
                    "Wridian 的底层灵魂、价值观和对话人格。",
                )?,
                memory_file_node(
                    &root,
                    "AGENTS.md",
                    "AGENTS.md",
                    "Wridian 如何行动、如何使用记忆树和何时询问用户。",
                )?,
                memory_file_node(
                    &root,
                    "MEMORY.md",
                    "MEMORY.md",
                    "主干索引、上下文编译策略、分支说明和最近活跃叶子。",
                )?,
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
            children: leaf_nodes(data_dir, &root)?,
        },
    ];
    if roots[2].children.is_empty() {
        roots[2].children.push(MemoryTreeNode {
            id: "leaves-empty".to_string(),
            kind: "empty".to_string(),
            label: "暂无叶子".to_string(),
            description: "对话自动沉淀或手动新增 Markdown 后，这里会长出叶子。".to_string(),
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
    if !target.is_absolute() {
        return Err("记忆树文件路径必须是绝对路径。".to_string());
    }
    let canonical_parent = target
        .parent()
        .ok_or_else(|| "记忆树文件路径无效。".to_string())?
        .canonicalize()
        .map_err(|error| format!("记忆树文件目录不存在：{error}"))?;
    let memory_root = memory_tree_files_root(data_dir)
        .canonicalize()
        .map_err(|error| format!("记忆树目录不存在：{error}"))?;
    let knowledge = resolved_knowledge_root(data_dir)?
        .canonicalize()
        .map_err(|error| format!("知识库目录不存在：{error}"))?;
    if !canonical_parent.starts_with(&memory_root) && !canonical_parent.starts_with(&knowledge) {
        return Err("只能编辑记忆树或知识库里的 Markdown 文件。".to_string());
    }
    if target
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .as_deref()
        != Some("md")
    {
        return Err("记忆树只允许编辑 Markdown 文件。".to_string());
    }
    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if metadata.file_type().is_symlink() {
            return Err("记忆树不允许编辑符号链接文件。".to_string());
        }
        let canonical_target = target
            .canonicalize()
            .map_err(|error| format!("记忆树文件路径无效：{error}"))?;
        if !canonical_target.starts_with(&memory_root) && !canonical_target.starts_with(&knowledge)
        {
            return Err("只能编辑记忆树或知识库里的 Markdown 文件。".to_string());
        }
    }
    fs::write(target, content).map_err(|error| format!("记忆树文件写入失败：{error}"))
}

fn delete_memory_tree_file(data_dir: &Path, path: &str) -> Result<(), String> {
    ensure_memory_tree_files(data_dir)?;
    let target = resolve_deletable_memory_tree_file(data_dir, path)?;
    fs::remove_file(target).map_err(|error| format!("记忆树文件删除失败：{error}"))
}

fn resolve_deletable_memory_tree_file(data_dir: &Path, path: &str) -> Result<PathBuf, String> {
    let target = PathBuf::from(path.trim());
    if !target.is_absolute() {
        return Err("记忆树文件路径必须是绝对路径。".to_string());
    }
    let metadata =
        fs::symlink_metadata(&target).map_err(|error| format!("记忆树文件不存在：{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("只能删除记忆树里的普通 Markdown 叶子文件。".to_string());
    }
    if target
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .as_deref()
        != Some("md")
    {
        return Err("记忆树只允许删除 Markdown 文件。".to_string());
    }
    let canonical_target = target
        .canonicalize()
        .map_err(|error| format!("记忆树文件路径无效：{error}"))?;
    let leaves_root = memory_tree_files_root(data_dir)
        .join("leaves")
        .canonicalize()
        .map_err(|error| format!("记忆树叶子目录不存在：{error}"))?;
    if !canonical_target.starts_with(&leaves_root) {
        return Err("只能删除记忆树 leaves 下的叶子文件。".to_string());
    }
    let file_name = canonical_target
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if matches!(file_name.as_str(), "project.md" | "compressed.md") {
        return Err("作品项目主记忆和压缩记忆不能删除，只能编辑。".to_string());
    }
    Ok(canonical_target)
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
    if let Some(active_work_root) = read_active_work_root(data_dir)? {
        let works = PathBuf::from(active_work_root);
        if !works.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(&works).map_err(|error| format!("作品记忆树读取失败：{error}"))?
        {
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
        ("SOUL.md", "# SOUL.md\n\nWridian 的图腾。这里定义底层灵魂、价值观和对话人格。稳定，不频繁变化。\n"),
        ("AGENTS.md", "# AGENTS.md\n\nWridian 的树根。这里定义如何行动、如何使用记忆树、哪些事必须问用户、哪些事不能自作主张。\n"),
        ("MEMORY.md", "# MEMORY.md\n\nWridian 记忆树主干。这里维护索引、上下文编译策略、分支说明和最近活跃叶子。\n\n## Context Compile\n\n- 先读 SOUL.md、AGENTS.md、MEMORY.md。\n- 再读命中分支的 branches/*.md。\n- 最后摘取最近、活跃、命中的 leaves。\n- 对话完成后可由模型返回结构化 memories，Wridian 自动写入 leaves；用户在创作记忆树里编辑或删除。\n"),
        ("branches/SENSE.md", "# SENSE.md\n\n自我意识机制。定义什么样的 agent 自己想做的事可以长成叶子，以及写入后如何被用户审阅、编辑和删除。\n"),
        ("branches/USER.md", "# USER.md\n\n用户画像准则。定义哪些创作之外的用户信息可以长成叶子，哪些不能写。\n"),
        ("branches/RELATIONSHIP.md", "# RELATIONSHIP.md\n\n关系准则。定义什么样的共处花絮值得记录，以及如何影响后续相处。\n"),
        ("branches/JOURNEY.md", "# JOURNEY.md\n\n创作里程碑。定义小节点如何沉淀，如何汇总成里程碑。\n"),
        ("branches/DRAMA.md", "# DRAMA.md\n\n剧本准则。定义剧本、短剧、分集、场景、对白相关记忆如何长叶。\n"),
        ("branches/NOVEL.md", "# NOVEL.md\n\n小说准则。定义小说、章节、人物、叙事、世界观相关记忆如何长叶。\n"),
        ("branches/KNOWLEDGE.md", "# KNOWLEDGE.md\n\n知识库调用机制。这里只定义创作记忆树如何引用外部通用知识库、知识卡和知识图谱；不把通用知识沉淀写成作品项目记忆。\n\n- 知识卡通过显式 @ 引用进入本轮对话上下文。\n- 作品项目可以采纳知识卡内容，但采纳后应改写成作品设定或规则。\n- 通用知识的来源、实体、概念和图谱留在知识库，不在创作记忆树中复制成死副本。\n"),
        ("branches/SKILL.md", "# SKILL.md\n\n技能生产准则。定义可复用创作方法、工作流、提示词和工具能力如何长叶。\n"),
        ("branches/AWARENESS.md", "# AWARENESS.md\n\n反思机制。定义什么时候反思，什么时候沉默，反思如何反哺整棵树。\n"),
    ]
}

fn ensure_project_memory_files(
    root: &Path,
    project_path: &Path,
    project_name: &str,
) -> Result<(), String> {
    let branch = project_branch_for_path(project_path);
    let folder = project_memory_folder(root, project_path);
    write_memory_tree_file_if_missing(
        &folder.join("project.md"),
        &format!("# {}\n\nbranch: {}\nsource: {}\nstatus: alive\n\n## 作品记忆\n\n这里记录只属于这个作品的长期记忆、规则、禁区、人物边界和续接线索。\n", project_name, branch, project_path.to_string_lossy()),
    )?;
    write_memory_tree_file_if_missing(
        &folder.join("compressed.md"),
        &format!("# {} 压缩记忆\n\nbranch: {}\nsource: {}\nstatus: active\n\n## 压缩记忆\n\n这里写当前作品项目最应该被项目记忆常驻读取的压缩记忆：核心设定、人物边界、禁区、当前进度和下一步。\n", project_name, branch, project_path.to_string_lossy()),
    )
}

fn project_memory_folder(root: &Path, project_path: &Path) -> PathBuf {
    let project_name = project_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "作品项目".to_string());
    let branch = project_branch_for_path(project_path);
    root.join("leaves").join(branch).join(format!(
        "{}-{}",
        sanitize_markdown_file_name(&project_name),
        stable_scope_id(&project_path.to_string_lossy())
    ))
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

fn memory_file_node(
    root: &Path,
    relative: &str,
    label: &str,
    description: &str,
) -> Result<MemoryTreeNode, String> {
    let path = root.join(relative);
    arbitrary_file_node(&path, label.to_string(), description.to_string())
}

fn arbitrary_file_node(
    path: &Path,
    label: String,
    description: String,
) -> Result<MemoryTreeNode, String> {
    Ok(MemoryTreeNode {
        id: path.to_string_lossy().into_owned(),
        kind: "file".to_string(),
        label,
        description,
        path: Some(path.to_string_lossy().into_owned()),
        content: Some(
            fs::read_to_string(path).map_err(|error| format!("记忆树文件读取失败：{error}"))?,
        ),
        children: Vec::new(),
    })
}

fn branch_nodes(root: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    MEMORY_BRANCHES
        .iter()
        .map(|(_, file, description)| {
            memory_file_node(root, &format!("branches/{file}"), file, description)
        })
        .collect()
}

fn leaf_nodes(data_dir: &Path, root: &Path) -> Result<Vec<MemoryTreeNode>, String> {
    let mut nodes = Vec::new();
    for (branch, file, description) in MEMORY_BRANCHES {
        let mut node = folder_node(
            &root.join("leaves").join(branch),
            branch.to_string(),
            format!("{description} 的具体叶子；规则见 branches/{file}。"),
        )?;
        if branch == "knowledge" {
            node.children
                .push(knowledge_cards_folder_node(data_dir, root)?);
            node.children
                .sort_by(|left, right| left.label.cmp(&right.label));
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn folder_node(path: &Path, label: String, description: String) -> Result<MemoryTreeNode, String> {
    let mut children = Vec::new();
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|error| format!("记忆树目录读取失败：{error}"))?
        {
            let entry = entry.map_err(|error| format!("记忆树目录读取失败：{error}"))?;
            let child = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Some(safe_child) = safe_child_path(path, &child, "记忆树")? else {
                continue;
            };
            if safe_child.is_dir() {
                children.push(folder_node(
                    &safe_child,
                    name,
                    "作品项目记忆分组。".to_string(),
                )?);
            } else if safe_child
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("md")
            {
                children.push(arbitrary_file_node(
                    &safe_child,
                    name,
                    "Markdown 记忆文件".to_string(),
                )?);
            }
        }
    }
    children.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.label.cmp(&right.label))
    });
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

fn knowledge_cards_folder_node(data_dir: &Path, root: &Path) -> Result<MemoryTreeNode, String> {
    let knowledge = Some(resolved_knowledge_root(data_dir)?).filter(|path| path.is_dir());
    let mut children = Vec::new();
    if let Some(knowledge_root) = &knowledge {
        collect_knowledge_card_nodes(knowledge_root, &mut children)?;
    }
    children.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(MemoryTreeNode {
        id: root
            .join("leaves")
            .join("knowledge")
            .join("cards")
            .to_string_lossy()
            .into_owned(),
        kind: "folder".to_string(),
        label: "cards".to_string(),
        description: "从当前知识库同步读取的知识卡。".to_string(),
        path: knowledge.map(|path| path.to_string_lossy().into_owned()),
        content: None,
        children,
    })
}

fn collect_knowledge_card_nodes(
    root: &Path,
    nodes: &mut Vec<MemoryTreeNode>,
) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| format!("知识卡目录读取失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("知识卡目录读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Some(safe_path) = safe_child_path(root, &path, "知识卡")? else {
            continue;
        };
        if safe_path.is_dir() {
            collect_knowledge_card_nodes(&safe_path, nodes)?;
        } else if safe_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("md")
        {
            let mut node = arbitrary_file_node(
                &safe_path,
                name,
                "当前知识库中的 Markdown 知识卡。".to_string(),
            )?;
            node.kind = "knowledge-card".to_string();
            nodes.push(node);
        }
    }
    Ok(())
}

fn migrate_legacy_memory_files(_data_dir: &Path, root: &Path) -> Result<(), String> {
    copy_legacy_if_target_empty(&root.join("partner").join("soul.md"), &root.join("SOUL.md"))?;
    copy_legacy_if_target_empty(
        &root.join("global").join("AGENTS.md"),
        &root.join("AGENTS.md"),
    )?;
    copy_legacy_if_target_empty(
        &root.join("global").join("MEMORY.md"),
        &root.join("MEMORY.md"),
    )?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-memory-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn memory_tree_reads_knowledge_cards_from_selected_source_without_mirror_copy() {
        let data_dir = temp_data_dir("knowledge-sync");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        fs::write(knowledge_root.join("人物.md"), "第一版").expect("write knowledge");
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

        let first = read_memory_tree_files(&data_dir).expect("read first tree");
        let knowledge_leaf =
            find_node_by_label(&first.roots, "人物.md").expect("knowledge leaf exists");
        assert_eq!(knowledge_leaf.kind, "knowledge-card");
        assert_eq!(knowledge_leaf.content.as_deref(), Some("第一版"));

        fs::write(knowledge_root.join("人物.md"), "第二版").expect("update knowledge");
        let second = read_memory_tree_files(&data_dir).expect("read second tree");
        let updated_leaf =
            find_node_by_label(&second.roots, "人物.md").expect("updated leaf exists");

        assert_eq!(updated_leaf.content.as_deref(), Some("第二版"));
        assert!(!memory_tree_files_root(&data_dir)
            .join("leaves/knowledge/cards/人物.md")
            .exists());
    }

    #[test]
    fn memory_tree_does_not_sync_default_libraries_before_user_selection() {
        let data_dir = temp_data_dir("unselected-libraries");
        let default_work = crate::runtime::vault_root(&data_dir)
            .join("works")
            .join("默认作品");
        let default_knowledge = crate::runtime::knowledge_root(&data_dir);
        fs::create_dir_all(&default_work).expect("create default work");
        fs::create_dir_all(&default_knowledge).expect("create default knowledge");
        fs::write(default_knowledge.join("默认知识.md"), "默认知识")
            .expect("write default knowledge");

        let tree = read_memory_tree_files(&data_dir).expect("read tree");
        let novel_leaves = memory_tree_files_root(&data_dir)
            .join("leaves")
            .join("novel");
        let novel_leaf_count = fs::read_dir(&novel_leaves)
            .expect("read novel leaves")
            .filter_map(Result::ok)
            .count();

        assert!(find_node_by_label(&tree.roots, "默认知识.md").is_none());
        assert_eq!(novel_leaf_count, 0);
    }

    #[test]
    fn legacy_branch_files_are_not_copied_into_leaf_nodes() {
        let data_dir = temp_data_dir("legacy-not-leaves");
        let root = memory_tree_files_root(&data_dir);
        fs::create_dir_all(root.join("partner")).expect("create partner");
        fs::create_dir_all(root.join("global")).expect("create global");
        fs::write(root.join("partner").join("user.md"), "旧用户主文件").expect("write legacy user");
        fs::write(root.join("partner").join("relationship.md"), "旧关系主文件")
            .expect("write legacy relationship");
        fs::write(
            root.join("partner").join("partnermemory.md"),
            "旧伙伴记忆主文件",
        )
        .expect("write legacy partner memory");
        fs::write(root.join("global").join("AWARENESS.md"), "旧反思主文件")
            .expect("write legacy awareness");

        let tree = read_memory_tree_files(&data_dir).expect("read tree");

        assert!(find_node_by_label(&tree.roots, "legacy-user.md").is_none());
        assert!(find_node_by_label(&tree.roots, "legacy-relationship.md").is_none());
        assert!(find_node_by_label(&tree.roots, "legacy-partnermemory.md").is_none());
        assert!(find_node_by_label(&tree.roots, "legacy-awareness.md").is_none());
    }

    #[test]
    fn memory_tree_save_rejects_absolute_path_outside_allowed_roots() {
        let data_dir = temp_data_dir("reject-outside-root");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let outside_dir = data_dir.join("outside");
        fs::create_dir_all(&outside_dir).expect("create outside dir");
        let outside_file = outside_dir.join("escape.md");

        let error = save_memory_tree_file(&data_dir, &outside_file.to_string_lossy(), "逃逸")
            .expect_err("outside path should be rejected");

        assert!(error.contains("只能编辑记忆树或知识库里的 Markdown 文件"));
        assert!(!outside_file.exists());
    }

    #[test]
    fn write_memory_leaves_creates_editable_leaf_file() {
        let data_dir = temp_data_dir("auto-leaf");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");

        let paths = write_memory_leaves(
            &data_dir,
            &[MemoryLeafDraft {
                branch: "novel".to_string(),
                title: "人物禁区".to_string(),
                summary: "女主不能主动说出真相。".to_string(),
                reason: Some("模型从本轮对话提取。".to_string()),
                source_path: Some("chapter.md".to_string()),
            }],
        )
        .expect("write leaf");

        assert_eq!(paths.len(), 1);
        let content = fs::read_to_string(&paths[0]).expect("read leaf");
        assert!(content.contains("# 人物禁区"));
        assert!(content.contains("女主不能主动说出真相。"));
    }

    #[test]
    fn delete_memory_tree_file_rejects_project_core_files() {
        let data_dir = temp_data_dir("delete-core");
        let work_root = data_dir.join("works");
        let project = work_root.join("作品A");
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            crate::runtime::workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": data_dir.join("knowledge").to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");
        ensure_memory_tree_files(&data_dir).expect("ensure tree");
        let core_file = memory_tree_files_root(&data_dir)
            .join("leaves")
            .join("novel")
            .join(format!(
                "{}-{}",
                sanitize_markdown_file_name("作品A"),
                stable_scope_id(&project.to_string_lossy())
            ))
            .join("project.md");

        let error = delete_memory_tree_file(&data_dir, &core_file.to_string_lossy())
            .expect_err("core file delete should be rejected");

        assert!(error.contains("不能删除"));
        assert!(core_file.exists());
    }

    #[test]
    fn memory_tree_includes_project_core_files() {
        let data_dir = temp_data_dir("project-core-visible");
        let work_root = data_dir.join("works");
        let project = work_root.join("作品A");
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            crate::runtime::workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": data_dir.join("knowledge").to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let tree = read_memory_tree_files(&data_dir).expect("read tree");

        assert!(find_node_by_label(&tree.roots, "project.md").is_some());
        assert!(find_node_by_label(&tree.roots, "compressed.md").is_some());
    }

    #[test]
    fn project_continuity_memory_reads_only_project_memory_tree_files() {
        let data_dir = temp_data_dir("project-continuity");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        let project = work_root.join("作品A");
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        fs::write(knowledge_root.join("通用知识.md"), "不应进入作品续接").expect("write knowledge");
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
        ensure_memory_tree_files(&data_dir).expect("ensure tree");
        let folder = project_memory_folder(&memory_tree_files_root(&data_dir), &project);
        fs::write(folder.join("project.md"), "项目长期规则").expect("write project");
        fs::write(folder.join("compressed.md"), "项目压缩进度").expect("write compressed");
        fs::write(folder.join("必要叶子.md"), "续接线索").expect("write leaf");

        let context = read_project_continuity_memory(&data_dir, &project.to_string_lossy(), 6)
            .expect("read continuity");

        assert!(context.contains("项目长期规则"));
        assert!(context.contains("项目压缩进度"));
        assert!(context.contains("续接线索"));
        assert!(!context.contains("不应进入作品续接"));
    }

    fn find_node_by_label<'a>(
        nodes: &'a [MemoryTreeNode],
        label: &str,
    ) -> Option<&'a MemoryTreeNode> {
        for node in nodes {
            if node.label == label {
                return Some(node);
            }
            if let Some(child) = find_node_by_label(&node.children, label) {
                return Some(child);
            }
        }
        None
    }
}

fn collect_project_memory_leaf_files(
    folder: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !folder.is_dir() {
        return Ok(());
    }
    for entry in
        fs::read_dir(folder).map_err(|error| format!("作品记忆叶子目录读取失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("作品记忆叶子目录读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.starts_with('.') {
            continue;
        }
        let Some(safe_path) = safe_child_path(folder, &path, "作品记忆叶子")? else {
            continue;
        };
        if safe_path.is_dir() {
            collect_project_memory_leaf_files(&safe_path, files)?;
        } else if safe_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("md")
            && !matches!(name.as_str(), "project.md" | "compressed.md")
        {
            files.push(safe_path);
        }
    }
    Ok(())
}

fn project_memory_context_label(path: &Path) -> String {
    match path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
    {
        "project.md" => "项目长期记忆".to_string(),
        "compressed.md" => "项目压缩记忆".to_string(),
        other => format!("项目叶子：{other}"),
    }
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
        files.push(root.join("branches").join(if branch == "drama" {
            "DRAMA.md"
        } else {
            "NOVEL.md"
        }));
        files.push(
            root.join("leaves")
                .join(branch)
                .join(format!(
                    "{}-{}",
                    sanitize_markdown_file_name(&name),
                    stable_scope_id(&project_path.to_string_lossy())
                ))
                .join("project.md"),
        );
        files.push(
            root.join("leaves")
                .join(branch)
                .join(format!(
                    "{}-{}",
                    sanitize_markdown_file_name(&name),
                    stable_scope_id(&project_path.to_string_lossy())
                ))
                .join("compressed.md"),
        );
    }
    Ok(files)
}

fn project_for_source(
    data_dir: &Path,
    source_path: &str,
) -> Result<Option<(PathBuf, String)>, String> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(trimmed);
    let canonical = path.canonicalize().unwrap_or(path);
    let Some(active_work_root) = read_active_work_root(data_dir)? else {
        return Ok(None);
    };
    let works_path = PathBuf::from(active_work_root);
    if !works_path.is_dir() {
        return Ok(None);
    }
    let works = works_path.canonicalize().unwrap_or(works_path);
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
    if text.contains("剧")
        || text.contains("短剧")
        || text.contains("drama")
        || text.contains("screenplay")
    {
        "drama"
    } else {
        "novel"
    }
}

pub(crate) fn write_memory_leaves(
    data_dir: &Path,
    leaves: &[MemoryLeafDraft],
) -> Result<Vec<PathBuf>, String> {
    ensure_memory_tree_files(data_dir)?;
    leaves
        .iter()
        .map(|leaf| write_memory_leaf(data_dir, leaf))
        .collect()
}

fn write_memory_leaf(data_dir: &Path, leaf: &MemoryLeafDraft) -> Result<PathBuf, String> {
    let branch = normalize_branch(&leaf.branch)?;
    let title = leaf.title.trim();
    let summary = leaf.summary.trim();
    if title.is_empty() || summary.is_empty() {
        return Err("叶子标题和内容不能为空。".to_string());
    }
    let root = memory_tree_files_root(data_dir);
    let folder = root.join("leaves").join(branch);
    fs::create_dir_all(&folder).map_err(|error| format!("叶子目录创建失败：{error}"))?;
    let slug = sanitize_markdown_file_name(&format!("{}-{}", chrono_like_date(), title));
    let path = unique_markdown_path(&folder, &slug);
    let reason = leaf.reason.as_deref().unwrap_or_default();
    let source = leaf.source_path.as_deref().unwrap_or_default();
    let content = format!(
        "# {title}\n\nbranch: {branch}\nstatus: alive\ncreated: {}\nsource: {source}\n\n## Record\n\n{summary}\n\n## Why It Grew\n\n{}\n",
        crate::runtime::iso_timestamp(),
        if reason.trim().is_empty() { "模型从本轮对话中提取出可复用长期记忆。" } else { reason.trim() }
    );
    fs::write(&path, content).map_err(|error| format!("叶子写入失败：{error}"))?;
    Ok(path)
}

fn normalize_branch(branch: &str) -> Result<&'static str, String> {
    let lowered = branch.trim().to_lowercase();
    MEMORY_BRANCHES
        .iter()
        .find_map(|(key, _, _)| if *key == lowered { Some(*key) } else { None })
        .ok_or_else(|| "未知记忆分支。".to_string())
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
    folder
        .join(format!("{slug}-{}", stable_scope_id(slug)))
        .with_extension("md")
}

fn compact_markdown(text: &str, max_chars: usize) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
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
