use crate::path_safety::{is_symlink_or_reparse, safe_child_path};
use crate::runtime::{
    default_knowledge_root, ensure_workspace, iso_timestamp, vault_root, workspace_config_path,
    wridian_data_dir,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

const MAX_WORKSPACE_TEXT_FILE_BYTES: u64 = 512 * 1024;
const MAX_PREVIEW_ASSET_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInfo {
    vault_path: String,
    runtime_path: String,
    files_root_path: String,
    active_work_root: Option<String>,
    work_root_configured: bool,
    files: Vec<WorkFileNode>,
    knowledge_root_path: String,
    active_knowledge_root: Option<String>,
    knowledge_root_configured: bool,
    knowledge_files: Vec<WorkFileNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetWorkRootInput {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetKnowledgeRootInput {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilePathInput {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateNodeInput {
    parent_path: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameNodeInput {
    path: String,
    new_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenFileResponse {
    path: String,
    name: String,
    content: String,
    editable: bool,
    preview_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveFileResponse {
    ok: bool,
    saved_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewFileResponse {
    path: String,
    name: String,
    content: Option<String>,
    editable: bool,
    preview_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewAssetResponse {
    url: String,
    mime_type: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkFileNode {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) relative_path: String,
    pub(crate) library: String,
    pub(crate) folder: bool,
    pub(crate) children: Vec<WorkFileNode>,
}

#[tauri::command]
pub(crate) fn wridian_init_workspace() -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_set_work_root(input: SetWorkRootInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = PathBuf::from(input.path.trim());
    if !root.is_dir() {
        return Err("请选择一个存在的本地文件夹。".to_string());
    }
    write_workspace_roots_config(
        &data_dir,
        Some(&root),
        read_active_knowledge_root(&data_dir)?
            .as_deref()
            .map(Path::new),
    )?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_set_knowledge_root(
    input: SetKnowledgeRootInput,
) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = PathBuf::from(input.path.trim());
    if !root.is_dir() {
        return Err("请选择一个存在的本地文件夹。".to_string());
    }
    write_workspace_roots_config(
        &data_dir,
        read_active_work_root(&data_dir)?.as_deref().map(Path::new),
        Some(&root),
    )?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_open_file(input: FilePathInput) -> Result<OpenFileResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_editable_file(&data_dir, &input.path)?;
    let content = read_editable_file_content(&path)?;
    let preview_type = workspace_preview_type(&path);
    Ok(OpenFileResponse {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "未命名".to_string()),
        path: path.to_string_lossy().into_owned(),
        content,
        editable: true,
        preview_type,
    })
}

#[tauri::command]
pub(crate) fn wridian_preview_file(input: FilePathInput) -> Result<PreviewFileResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_workspace_file(&data_dir, &input.path)?;
    let editable = is_supported_editable_file(&path);
    let preview_type = workspace_preview_type(&path);
    let content = if editable || is_supported_text_preview_file(&path) {
        Some(read_workspace_text_content(&path)?)
    } else {
        None
    };
    Ok(PreviewFileResponse {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "未命名".to_string()),
        path: path.to_string_lossy().into_owned(),
        content,
        editable,
        preview_type,
    })
}

#[tauri::command]
pub(crate) fn wridian_preview_asset(input: FilePathInput) -> Result<PreviewAssetResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_workspace_file(&data_dir, &input.path)?;
    preview_asset_response(&path)
}

#[tauri::command]
pub(crate) fn wridian_save_file(input: SaveFileInput) -> Result<SaveFileResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_editable_file(&data_dir, &input.path)?;
    write_editable_file_content(&path, &input.content)?;
    Ok(SaveFileResponse {
        ok: true,
        saved_at: iso_timestamp(),
    })
}

#[tauri::command]
pub(crate) fn wridian_create_work_file(input: CreateNodeInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let parent = resolve_allowed_folder(&data_dir, &input.parent_path)?;
    let file_name = normalize_file_name(&input.name)?;
    let path = unique_child_path(&parent, &file_name);
    fs::write(&path, "").map_err(|error| format!("文件创建失败：{error}"))?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_create_work_folder(input: CreateNodeInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let parent = resolve_allowed_folder(&data_dir, &input.parent_path)?;
    let folder_name = normalize_node_name(&input.name)?;
    let path = unique_child_path(&parent, &folder_name);
    fs::create_dir_all(&path).map_err(|error| format!("文件夹创建失败：{error}"))?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_duplicate_work_node(input: FilePathInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let source = resolve_allowed_existing_node(&data_dir, &input.path)?;
    let parent = source
        .parent()
        .ok_or_else(|| "无法复制工作区根目录。".to_string())?;
    let source_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| "无法复制工作区根目录。".to_string())?;
    let target = unique_child_path(parent, &format!("{source_name} 副本"));
    if source.is_dir() {
        copy_dir_recursive(&source, &source, &target)?;
    } else {
        fs::copy(&source, &target).map_err(|error| format!("文件副本创建失败：{error}"))?;
    }
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_rename_work_node(input: RenameNodeInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let source = resolve_allowed_existing_node(&data_dir, &input.path)?;
    let parent = source
        .parent()
        .ok_or_else(|| "无法重命名工作区根目录。".to_string())?;
    let name = if source.is_file() {
        normalize_workspace_file_name(&input.new_name)?
    } else {
        normalize_node_name(&input.new_name)?
    };
    let target = parent.join(name);
    if target.exists() {
        return Err("同名文件或文件夹已存在。".to_string());
    }
    fs::rename(&source, &target).map_err(|error| format!("重命名失败：{error}"))?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_trash_work_node(input: FilePathInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    trash_workspace_node(&data_dir, &input.path)?;
    workspace_info(&data_dir)
}

fn trash_workspace_node(data_dir: &Path, path: &str) -> Result<(), String> {
    let source = resolve_allowed_existing_node(data_dir, path)?;
    let root = containing_work_root(&data_dir, &source)?
        .ok_or_else(|| "文件不在当前 Wridian 工作目录内。".to_string())?;
    if source == root {
        return Err("不能移动工作区根目录。".to_string());
    }
    move_workspace_node_to_system_trash(&source)?;
    Ok(())
}

pub(crate) fn read_workspace_file_trees(data_dir: &Path) -> Result<Vec<WorkFileNode>, String> {
    let mut nodes = Vec::new();
    let files_root = files_root(data_dir)?;
    if read_active_work_root(data_dir)?.is_some() {
        nodes.extend(read_work_tree(&files_root, &files_root, "works")?);
    }
    let knowledge_root = resolved_knowledge_root(data_dir)?;
    nodes.extend(read_work_tree(
        &knowledge_root,
        &knowledge_root,
        "knowledge",
    )?);
    Ok(nodes)
}

pub(crate) fn apply_workspace_write_file(
    data_dir: &Path,
    library: &str,
    relative_path: &str,
    content: &str,
) -> Result<PathBuf, String> {
    let root = workspace_library_root(data_dir, library)?;
    let path = resolve_relative_workspace_target(&root, relative_path)?;
    if !is_supported_editable_file(&path) {
        return Err("只能写入 md、txt、docx 文件。".to_string());
    }
    ensure_safe_workspace_parent(&root, &path, "写入文件")?;
    if path.exists() {
        return Err(
            "writeFile 只用于新建文件；修改已有文件内容请返回 edits 并由前端内联确认。".to_string(),
        );
    }
    ensure_safe_workspace_write_target(&root, &path, "写入文件")?;
    write_editable_file_content(&path, content)?;
    Ok(path)
}

pub(crate) fn apply_workspace_create_folder(
    data_dir: &Path,
    library: &str,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let root = workspace_library_root(data_dir, library)?;
    let path = resolve_relative_workspace_target(&root, relative_path)?;
    ensure_safe_workspace_parent(&root, &path, "创建文件夹")?;
    fs::create_dir_all(&path).map_err(|error| format!("文件夹创建失败：{error}"))?;
    ensure_safe_existing_workspace_path(&root, &path, "创建文件夹")?;
    Ok(path)
}

pub(crate) fn apply_workspace_rename_node(
    data_dir: &Path,
    library: &str,
    relative_path: &str,
    new_name: &str,
) -> Result<PathBuf, String> {
    let root = workspace_library_root(data_dir, library)?;
    let source = resolve_existing_relative_workspace_node(&root, relative_path)?;
    let parent = source
        .parent()
        .ok_or_else(|| "不能重命名库根目录。".to_string())?;
    let name = if source.is_file() {
        normalize_workspace_file_name(new_name)?
    } else {
        normalize_node_name(new_name)?
    };
    let target = parent.join(name);
    if !target.starts_with(&root) {
        return Err("目标路径不在当前库内。".to_string());
    }
    if target.exists() {
        return Err("同名文件或文件夹已存在。".to_string());
    }
    fs::rename(&source, &target).map_err(|error| format!("重命名失败：{error}"))?;
    Ok(target)
}

pub(crate) fn apply_workspace_trash_node(
    data_dir: &Path,
    library: &str,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let root = workspace_library_root(data_dir, library)?;
    let source = resolve_existing_relative_workspace_node(&root, relative_path)?;
    if source == root {
        return Err("不能移动库根目录。".to_string());
    }
    move_workspace_node_to_system_trash(&source)?;
    Ok(source)
}

fn workspace_info(data_dir: &Path) -> Result<WorkspaceInfo, String> {
    let files_root = files_root(data_dir)?;
    let active_work_root = read_active_work_root(data_dir)?;
    let resolved_knowledge = resolved_knowledge_root(data_dir)?;
    let active_knowledge_root = read_active_knowledge_root(data_dir)?;
    Ok(WorkspaceInfo {
        vault_path: vault_root(data_dir).to_string_lossy().into_owned(),
        runtime_path: crate::runtime::runtime_root(data_dir)
            .to_string_lossy()
            .into_owned(),
        files_root_path: files_root.to_string_lossy().into_owned(),
        active_work_root: active_work_root.clone(),
        work_root_configured: active_work_root.is_some(),
        files: if active_work_root.is_some() {
            read_work_tree(&files_root, &files_root, "works")?
        } else {
            Vec::new()
        },
        knowledge_root_path: resolved_knowledge.to_string_lossy().into_owned(),
        active_knowledge_root: active_knowledge_root.clone(),
        knowledge_root_configured: true,
        knowledge_files: read_work_tree(&resolved_knowledge, &resolved_knowledge, "knowledge")?,
    })
}

fn workspace_library_root(data_dir: &Path, library: &str) -> Result<PathBuf, String> {
    let root = match library.trim() {
        "works" => files_root(data_dir)?,
        "knowledge" => resolved_knowledge_root(data_dir)?,
        _ => return Err("文件操作 library 必须是 works 或 knowledge。".to_string()),
    };
    root.canonicalize()
        .map_err(|error| format!("库目录解析失败：{error}"))
}

pub(crate) fn workspace_library_root_for_audit(
    data_dir: &Path,
    library: &str,
) -> Result<PathBuf, String> {
    workspace_library_root(data_dir, library)
}

fn resolve_relative_workspace_target(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = normalize_relative_workspace_path(relative_path)?;
    let target = root.join(relative);
    if target.starts_with(root) {
        Ok(target)
    } else {
        Err("目标路径不在当前库内。".to_string())
    }
}

pub(crate) fn resolve_relative_workspace_target_for_audit(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    resolve_relative_workspace_target(root, relative_path)
}

fn ensure_safe_workspace_parent(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label}目标缺少父目录。"))?;
    if parent == root {
        return Ok(());
    }
    let relative_parent = parent
        .strip_prefix(root)
        .map_err(|_| format!("{label}目标不在当前库内。"))?;
    let mut current = root.to_path_buf();
    for segment in relative_parent.components() {
        current.push(segment.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if is_symlink_or_reparse(&metadata) {
                    return Err(format!("{label}目标路径包含链接或重解析点。"));
                }
                if !metadata.is_dir() {
                    return Err(format!("{label}目标父路径不是文件夹。"));
                }
                let canonical = current
                    .canonicalize()
                    .map_err(|error| format!("{label}路径解析失败：{error}"))?;
                if !canonical.starts_with(root) {
                    return Err(format!("{label}目标路径不在当前库内。"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&current).map_err(|error| format!("文件夹创建失败：{error}"))?;
                ensure_safe_existing_workspace_path(root, &current, label)?;
            }
            Err(error) => return Err(format!("{label}路径信息读取失败：{error}")),
        }
    }
    Ok(())
}

fn ensure_safe_workspace_write_target(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if is_symlink_or_reparse(&metadata) {
                return Err(format!("{label}目标不能是链接或重解析点。"));
            }
            if !metadata.is_file() {
                return Err(format!("{label}目标不是普通文件。"));
            }
            ensure_safe_existing_workspace_path(root, path, label)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{label}目标路径信息读取失败：{error}")),
    }
}

fn ensure_safe_existing_workspace_path(
    root: &Path,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{label}路径信息读取失败：{error}"))?;
    if is_symlink_or_reparse(&metadata) {
        return Err(format!("{label}目标不能是链接或重解析点。"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("{label}路径解析失败：{error}"))?;
    if canonical.starts_with(root) {
        Ok(())
    } else {
        Err(format!("{label}目标路径不在当前库内。"))
    }
}

fn resolve_existing_relative_workspace_node(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let target = resolve_relative_workspace_target(root, relative_path)?;
    reject_link_or_reparse_node(&target, "文件树节点")?;
    let canonical = target
        .canonicalize()
        .map_err(|error| format!("文件树节点不存在：{error}"))?;
    if !canonical.starts_with(root) {
        return Err("目标路径不在当前库内。".to_string());
    }
    if canonical.is_file() && !is_supported_workspace_file(&canonical) {
        return Err("只能操作 Wridian 文件树支持显示的常见文件。".to_string());
    }
    Ok(canonical)
}

fn normalize_relative_workspace_path(relative_path: &str) -> Result<PathBuf, String> {
    let trimmed = relative_path.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return Err("文件操作需要库内相对路径。".to_string());
    }
    let path = Path::new(&trimmed);
    if path.is_absolute()
        || trimmed
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("文件操作路径不能包含绝对路径、空段或 ..。".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

fn write_workspace_roots_config(
    data_dir: &Path,
    work_root: Option<&Path>,
    knowledge_root: Option<&Path>,
) -> Result<(), String> {
    let config = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "activeWorkRoot": work_root.map(|root| root.to_string_lossy().into_owned()),
        "knowledgeRoot": knowledge_root.map(|root| root.to_string_lossy().into_owned())
    }))
    .map_err(|error| error.to_string())?;
    fs::write(workspace_config_path(data_dir), config)
        .map_err(|error| format!("Wridian 工作区配置写入失败：{error}"))
}

pub(crate) fn read_active_work_root(data_dir: &Path) -> Result<Option<String>, String> {
    let path = workspace_config_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Wridian 工作区配置读取失败：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| format!("Wridian 工作区配置格式损坏：{error}"))?;
    Ok(value
        .get("activeWorkRoot")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned))
}

pub(crate) fn read_active_knowledge_root(data_dir: &Path) -> Result<Option<String>, String> {
    let path = workspace_config_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Wridian 工作区配置读取失败：{error}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| format!("Wridian 工作区配置格式损坏：{error}"))?;
    Ok(value
        .get("knowledgeRoot")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned))
}

fn files_root(data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return Ok(path);
        }
    }
    Ok(vault_root(data_dir).join("works"))
}

pub(crate) fn resolved_knowledge_root(data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(root) = read_active_knowledge_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return Ok(path);
        }
    }
    Ok(default_knowledge_root(data_dir))
}

fn read_work_tree(root: &Path, base: &Path, library: &str) -> Result<Vec<WorkFileNode>, String> {
    let mut visited = 0;
    read_work_tree_inner(root, base, library, 0, &mut visited)
}

fn read_work_tree_inner(
    root: &Path,
    base: &Path,
    library: &str,
    depth: usize,
    visited: &mut usize,
) -> Result<Vec<WorkFileNode>, String> {
    const MAX_WORK_TREE_DEPTH: usize = 24;
    const MAX_WORK_TREE_NODES: usize = 5000;
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    if depth > MAX_WORK_TREE_DEPTH || *visited >= MAX_WORK_TREE_NODES {
        return Ok(Vec::new());
    }
    let mut nodes = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| format!("作品目录读取失败：{error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("作品目录读取失败：{error}"))?;
        let path = entry.path();
        if *visited >= MAX_WORK_TREE_NODES {
            break;
        }
        if safe_child_path(base, &path, "文件树")?.is_none() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if should_skip_entry(&name) {
            continue;
        }
        let relative = relative_path(base, &path);
        if should_skip_workspace_tree_node(library, &relative) {
            continue;
        }
        *visited += 1;
        if path.is_dir() {
            let children = read_work_tree_inner(&path, base, library, depth + 1, visited)?;
            nodes.push(WorkFileNode {
                name,
                path: path.to_string_lossy().into_owned(),
                relative_path: relative,
                library: library.to_string(),
                folder: true,
                children,
            });
        } else if is_supported_workspace_file(&path) {
            nodes.push(WorkFileNode {
                name,
                path: path.to_string_lossy().into_owned(),
                relative_path: relative,
                library: library.to_string(),
                folder: false,
                children: Vec::new(),
            });
        }
    }
    nodes.sort_by(|a, b| {
        b.folder
            .cmp(&a.folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(nodes)
}

fn should_skip_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | ".wridian" | ".wridian-trash"
    ) || name.starts_with('.')
}

#[cfg(not(test))]
fn move_workspace_node_to_system_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| format!("移到系统回收站失败：{error}"))
}

#[cfg(test)]
fn move_workspace_node_to_system_trash(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("测试回收站删除失败：{error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("测试回收站删除失败：{error}"))
    }
}

fn should_skip_workspace_tree_node(library: &str, relative_path: &str) -> bool {
    library == "knowledge"
        && matches!(
            relative_path.replace('\\', "/").as_str(),
            "hot.md" | "00知识库治理/folds"
        )
}

pub(crate) fn is_supported_writing_file(path: &Path) -> bool {
    is_supported_editable_file(path)
}

pub(crate) fn is_supported_editable_file(path: &Path) -> bool {
    file_extension(path)
        .map(|extension| matches!(extension.as_str(), "md" | "markdown" | "txt" | "docx"))
        .unwrap_or(false)
}

pub(crate) fn is_supported_text_preview_file(path: &Path) -> bool {
    file_extension(path)
        .map(|extension| {
            matches!(
                extension.as_str(),
                "md" | "markdown" | "txt" | "docx" | "csv" | "json" | "yaml" | "yml"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn is_supported_workspace_file(path: &Path) -> bool {
    file_extension(path)
        .map(|extension| {
            matches!(
                extension.as_str(),
                "md" | "markdown"
                    | "txt"
                    | "doc"
                    | "docx"
                    | "wps"
                    | "pdf"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "webp"
                    | "gif"
                    | "svg"
                    | "bmp"
                    | "csv"
                    | "xlsx"
                    | "xls"
                    | "et"
                    | "ppt"
                    | "pptx"
                    | "dps"
                    | "json"
                    | "yaml"
                    | "yml"
            )
        })
        .unwrap_or(false)
}

fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn resolve_allowed_editable_file(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_file() || !is_supported_editable_file(&path) {
        return Err("文件编辑区只能直接编辑 md、txt、docx 文件。".to_string());
    }
    resolve_allowed_workspace_file_path(data_dir, &path)
}

fn resolve_allowed_workspace_file(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_file() || !is_supported_workspace_file(&path) {
        return Err("文件不属于 Wridian 支持的常见格式。".to_string());
    }
    resolve_allowed_workspace_file_path(data_dir, &path)
}

fn resolve_allowed_workspace_file_path(data_dir: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("文件路径解析失败：{error}"))?;
    let roots = allowed_work_roots(data_dir)?;
    if roots.iter().any(|root| canonical_path.starts_with(root)) {
        Ok(canonical_path)
    } else {
        Err("文件不在当前 Wridian 工作目录内。".to_string())
    }
}

fn resolve_allowed_folder(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_dir() {
        return Err("请选择一个存在的文件夹。".to_string());
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("文件夹路径解析失败：{error}"))?;
    if containing_work_root(data_dir, &canonical_path)?.is_some() {
        Ok(canonical_path)
    } else {
        Err("文件夹不在当前 Wridian 工作目录内。".to_string())
    }
}

fn resolve_allowed_existing_node(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !(path.is_file() || path.is_dir()) {
        return Err("文件或文件夹不存在。".to_string());
    }
    reject_link_or_reparse_node(&path, "文件树节点")?;
    if path.is_file() && !is_supported_workspace_file(&path) {
        return Err("只能操作 Wridian 文件树支持显示的常见文件。".to_string());
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("路径解析失败：{error}"))?;
    if containing_work_root(data_dir, &canonical_path)?.is_some() {
        Ok(canonical_path)
    } else {
        Err("文件不在当前 Wridian 工作目录内。".to_string())
    }
}

fn reject_link_or_reparse_node(path: &Path, label: &str) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{label}路径信息读取失败：{error}"))?;
    if is_symlink_or_reparse(&metadata) {
        return Err(format!("{label}不能是链接或重解析点。"));
    }
    Ok(())
}

pub(crate) fn allowed_work_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            push_unique_root(
                &mut roots,
                path.canonicalize()
                    .map_err(|error| format!("作品目录解析失败：{error}"))?,
            );
        }
    }
    let knowledge = resolved_knowledge_root(data_dir)?;
    if knowledge.is_dir() {
        push_unique_root(
            &mut roots,
            knowledge
                .canonicalize()
                .map_err(|error| format!("知识库目录解析失败：{error}"))?,
        );
    }
    if roots.is_empty() {
        push_unique_root(
            &mut roots,
            vault_root(data_dir)
                .canonicalize()
                .map_err(|error| format!("默认写作目录解析失败：{error}"))?,
        );
    }
    Ok(roots)
}

fn push_unique_root(roots: &mut Vec<PathBuf>, root: PathBuf) {
    if !roots.iter().any(|existing| existing == &root) {
        roots.push(root);
    }
}

pub(crate) fn works_root(data_dir: &Path) -> Result<PathBuf, String> {
    files_root(data_dir)
}

fn containing_work_root(data_dir: &Path, path: &Path) -> Result<Option<PathBuf>, String> {
    Ok(allowed_work_roots(data_dir)?
        .into_iter()
        .find(|root| path.starts_with(root)))
}

fn normalize_file_name(name: &str) -> Result<String, String> {
    let mut normalized = normalize_node_name(name)?;
    let path = Path::new(&normalized);
    if path.extension().is_none() {
        normalized.push_str(".md");
    }
    if !is_supported_editable_file(Path::new(&normalized)) {
        return Err("文件名只支持 md、markdown、txt 或 docx 后缀。".to_string());
    }
    Ok(normalized)
}

fn normalize_workspace_file_name(name: &str) -> Result<String, String> {
    let normalized = normalize_node_name(name)?;
    if !is_supported_workspace_file(Path::new(&normalized)) {
        return Err("文件名只支持 Wridian 文件树可显示的常见文件后缀。".to_string());
    }
    Ok(normalized)
}

fn normalize_node_name(name: &str) -> Result<String, String> {
    let normalized = name.trim();
    if normalized.is_empty() || normalized == "." || normalized == ".." {
        return Err("名称不能为空。".to_string());
    }
    if normalized.contains('/') || normalized.contains('\\') {
        return Err("名称不能包含路径分隔符。".to_string());
    }
    Ok(normalized.to_string())
}

fn unique_child_path(parent: &Path, desired_name: &str) -> PathBuf {
    let mut candidate = parent.join(desired_name);
    if !candidate.exists() {
        return candidate;
    }

    let desired_path = Path::new(desired_name);
    let stem = desired_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| desired_name.to_string());
    let extension = desired_path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();

    for index in 2..1000 {
        candidate = parent.join(format!("{stem} {index}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!(
        "{stem} {}{extension}",
        crate::runtime::filename_timestamp()
    ))
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn copy_dir_recursive(root: &Path, source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| format!("文件夹副本创建失败：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("文件夹读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("文件夹读取失败：{error}"))?;
        let source_path = entry.path();
        let Some(safe_source_path) = safe_child_path(root, &source_path, "复制文件")? else {
            continue;
        };
        let target_path = target.join(entry.file_name());
        if safe_source_path.is_dir() {
            copy_dir_recursive(root, &safe_source_path, &target_path)?;
        } else {
            fs::copy(&safe_source_path, &target_path)
                .map_err(|error| format!("文件副本创建失败：{error}"))?;
        }
    }
    Ok(())
}

pub(crate) fn read_workspace_text_content(path: &Path) -> Result<String, String> {
    ensure_file_size_at_most(path, MAX_WORKSPACE_TEXT_FILE_BYTES, "文件读取")?;
    if file_extension(path).as_deref() == Some("docx") {
        return read_docx_plain_text(path);
    }
    fs::read_to_string(path).map_err(|error| format!("文件读取失败：{error}"))
}

fn read_editable_file_content(path: &Path) -> Result<String, String> {
    read_workspace_text_content(path)
}

fn write_editable_file_content(path: &Path, content: &str) -> Result<(), String> {
    if file_extension(path).as_deref() == Some("docx") {
        ensure_docx_plain_text_editable(path)?;
        return write_docx_plain_text(path, content);
    }
    fs::write(path, content).map_err(|error| format!("文件保存失败：{error}"))
}

fn workspace_preview_type(path: &Path) -> String {
    match file_extension(path).as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp") => "image",
        Some("pdf") => "pdf",
        Some("md" | "markdown" | "txt" | "docx" | "csv" | "json" | "yaml" | "yml") => "text",
        _ => "external",
    }
    .to_string()
}

fn preview_asset_response(path: &Path) -> Result<PreviewAssetResponse, String> {
    let mime_type =
        preview_asset_mime_type(path).ok_or_else(|| "当前格式不能直接预览。".to_string())?;
    ensure_file_size_at_most(path, MAX_PREVIEW_ASSET_BYTES, "预览文件")?;
    let bytes = fs::read(path).map_err(|error| format!("预览文件读取失败：{error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(PreviewAssetResponse {
        url: format!("data:{mime_type};base64,{encoded}"),
        mime_type: mime_type.to_string(),
    })
}

fn preview_asset_mime_type(path: &Path) -> Option<&'static str> {
    match file_extension(path).as_deref() {
        Some("pdf") => Some("application/pdf"),
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        Some("svg") => Some("image/svg+xml"),
        Some("bmp") => Some("image/bmp"),
        _ => None,
    }
}

fn ensure_file_size_at_most(path: &Path, max_bytes: u64, label: &str) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{label}大小检查失败：{error}"))?;
    if metadata.len() > max_bytes {
        return Err(format!(
            "{label}过大，已跳过。当前上限为 {} MB。",
            bytes_to_display_mb(max_bytes)
        ));
    }
    Ok(())
}

fn bytes_to_display_mb(bytes: u64) -> u64 {
    (bytes / (1024 * 1024)).max(1)
}

fn read_docx_plain_text(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("DOCX 读取失败：{error}"))?;
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|error| format!("DOCX 格式解析失败：{error}"))?;
    let mut document = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| format!("DOCX 正文读取失败：{error}"))?
        .read_to_string(&mut document)
        .map_err(|error| format!("DOCX 正文解码失败：{error}"))?;
    Ok(docx_document_xml_to_text(&document))
}

fn ensure_docx_plain_text_editable(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("DOCX 读取失败：{error}"))?;
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|error| format!("DOCX 格式解析失败：{error}"))?;
    let mut document = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| format!("DOCX 正文读取失败：{error}"))?
        .read_to_string(&mut document)
        .map_err(|error| format!("DOCX 正文解码失败：{error}"))?;
    if docx_has_complex_semantics(&document) {
        return Err(
            "该 DOCX 包含表格、脚注、批注、修订或复杂结构；Wridian 当前只允许保存纯文本 DOCX，避免破坏原文档。"
                .to_string(),
        );
    }
    Ok(())
}

fn write_docx_plain_text(path: &Path, content: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("DOCX 读取失败：{error}"))?;
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|error| format!("DOCX 格式解析失败：{error}"))?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| format!("DOCX 文件读取失败：{error}"))?;
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|error| format!("DOCX 文件读取失败：{error}"))?;
        files.push((name, data));
    }
    drop(archive);

    let document = crate::docx_xml::minimal_docx_document_xml(content);
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut replaced_document = false;
        for (name, data) in files {
            writer
                .start_file(&name, options)
                .map_err(|error| format!("DOCX 写入失败：{error}"))?;
            if name == "word/document.xml" {
                writer
                    .write_all(document.as_bytes())
                    .map_err(|error| format!("DOCX 正文写入失败：{error}"))?;
                replaced_document = true;
            } else {
                writer
                    .write_all(&data)
                    .map_err(|error| format!("DOCX 文件写入失败：{error}"))?;
            }
        }
        if !replaced_document {
            return Err("DOCX 缺少 word/document.xml。".to_string());
        }
        writer
            .finish()
            .map_err(|error| format!("DOCX 保存失败：{error}"))?;
    }
    fs::write(path, output.into_inner()).map_err(|error| format!("DOCX 保存失败：{error}"))
}

fn docx_has_complex_semantics(document_xml: &str) -> bool {
    [
        "<w:tbl",
        "<w:footnoteReference",
        "<w:endnoteReference",
        "<w:commentRangeStart",
        "<w:commentReference",
        "<w:ins",
        "<w:del",
        "<w:drawing",
        "<w:pict",
        "<w:hyperlink",
        "<w:sdt",
    ]
    .iter()
    .any(|marker| document_xml.contains(marker))
}

fn docx_document_xml_to_text(xml: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut search = 0;
    while let Some(start) = xml[search..].find("<w:p") {
        let paragraph_start = search + start;
        let Some(start_tag_end) = xml[paragraph_start..].find('>') else {
            break;
        };
        let content_start = paragraph_start + start_tag_end + 1;
        let Some(end_offset) = xml[content_start..].find("</w:p>") else {
            break;
        };
        let content_end = content_start + end_offset;
        let text = docx_text_runs_to_text(&xml[content_start..content_end]);
        paragraphs.push(text);
        search = content_end + "</w:p>".len();
    }
    if !paragraphs.is_empty() {
        return paragraphs.join("\n").trim().to_string();
    }
    let fallback = docx_text_runs_to_text(xml);
    if fallback.trim().is_empty() {
        plain_text_from_xml(xml)
    } else {
        fallback.trim().to_string()
    }
}

fn docx_text_runs_to_text(xml: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find("<w:t") {
        let tag_start = cursor + start;
        let Some(tag_end_offset) = xml[tag_start..].find('>') else {
            break;
        };
        let text_start = tag_start + tag_end_offset + 1;
        let Some(text_end_offset) = xml[text_start..].find("</w:t>") else {
            break;
        };
        let text_end = text_start + text_end_offset;
        output.push_str(&decode_xml_text(&xml[text_start..text_end]));
        cursor = text_end + "</w:t>".len();
    }
    output
}

fn plain_text_from_xml(xml: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    decode_xml_text(&output)
}

fn decode_xml_text(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-workspace-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn workspace_info_exposes_relative_paths_for_selected_libraries() {
        let data_dir = temp_data_dir("relative-paths");
        let work_root = data_dir.join("user-works");
        let knowledge_root = data_dir.join("user-knowledge");
        fs::create_dir_all(work_root.join("作品A")).expect("create work project");
        fs::create_dir_all(&knowledge_root).expect("create knowledge root");
        fs::write(work_root.join("作品A").join("第一章.md"), "正文").expect("write work file");
        fs::write(knowledge_root.join("人物卡.md"), "人物").expect("write knowledge file");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let info = workspace_info(&data_dir).expect("workspace info");
        let work_file = &info.files[0].children[0];
        let knowledge_file = &info.knowledge_files[0];

        assert_eq!(work_file.relative_path, "作品A/第一章.md");
        assert_eq!(knowledge_file.relative_path, "人物卡.md");
        assert_eq!(work_file.library, "works");
        assert_eq!(knowledge_file.library, "knowledge");
    }

    #[test]
    fn workspace_tree_displays_common_files_and_edits_word_notes() {
        let data_dir = temp_data_dir("common-files");
        let work_root = data_dir.join("user-works");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::write(work_root.join("方案.md"), "正文").expect("write md");
        fs::write(work_root.join("资料.txt"), "文本").expect("write txt");
        fs::write(work_root.join("报告.pdf"), b"pdf").expect("write pdf");
        write_minimal_test_docx(&work_root.join("合同.docx"), "合同正文").expect("write docx");
        fs::write(work_root.join("国产文档.wps"), b"wps").expect("write wps");
        fs::write(work_root.join("图片.png"), b"png").expect("write png");
        fs::write(work_root.join("程序.exe"), b"exe").expect("write exe");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let info = workspace_info(&data_dir).expect("workspace info");
        let names = info
            .files
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"方案.md"));
        assert!(names.contains(&"资料.txt"));
        assert!(names.contains(&"报告.pdf"));
        assert!(names.contains(&"合同.docx"));
        assert!(names.contains(&"国产文档.wps"));
        assert!(names.contains(&"图片.png"));
        assert!(!names.contains(&"程序.exe"));
        assert!(resolve_allowed_editable_file(
            &data_dir,
            &work_root.join("方案.md").to_string_lossy()
        )
        .is_ok());
        assert!(resolve_allowed_editable_file(
            &data_dir,
            &work_root.join("资料.txt").to_string_lossy()
        )
        .is_ok());
        assert!(resolve_allowed_editable_file(
            &data_dir,
            &work_root.join("报告.pdf").to_string_lossy()
        )
        .is_err());
        assert!(resolve_allowed_editable_file(
            &data_dir,
            &work_root.join("合同.docx").to_string_lossy()
        )
        .is_ok());

        let pdf_preview = preview_asset_response(&work_root.join("报告.pdf")).expect("preview pdf");
        assert_eq!(pdf_preview.mime_type, "application/pdf");
        assert!(pdf_preview.url.starts_with("data:application/pdf;base64,"));

        let image_preview =
            preview_asset_response(&work_root.join("图片.png")).expect("preview image");
        assert_eq!(image_preview.mime_type, "image/png");
        assert!(image_preview.url.starts_with("data:image/png;base64,"));

        assert!(preview_asset_response(&work_root.join("资料.txt")).is_err());
    }

    #[test]
    fn workspace_trash_moves_to_system_trash_without_local_trash_folder() {
        let data_dir = temp_data_dir("system-trash");
        let work_root = data_dir.join("user-works");
        let knowledge_root = data_dir.join("user-knowledge");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::create_dir_all(&knowledge_root).expect("create knowledge root");
        let work_file = work_root.join("旧稿.md");
        let knowledge_file = knowledge_root.join("旧卡.md");
        fs::write(&work_file, "旧稿").expect("write work file");
        fs::write(&knowledge_file, "旧卡").expect("write knowledge file");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        trash_workspace_node(&data_dir, &work_file.to_string_lossy()).expect("trash work file");
        let info = workspace_info(&data_dir).expect("workspace info");
        assert!(!work_file.exists());
        assert!(!work_root.join(".wridian-trash").exists());
        assert!(!info.files.iter().any(|node| node.name == "旧稿.md"));

        let removed = apply_workspace_trash_node(&data_dir, "knowledge", "旧卡.md")
            .expect("trash knowledge file");
        assert_eq!(removed.file_name(), knowledge_file.file_name());
        assert_eq!(
            removed.parent().and_then(Path::file_name),
            knowledge_file.parent().and_then(Path::file_name)
        );
        assert!(!knowledge_file.exists());
        assert!(!knowledge_root.join(".wridian-trash").exists());
    }

    #[test]
    fn workspace_trash_rejects_roots_and_outside_paths() {
        let data_dir = temp_data_dir("system-trash-boundary");
        let work_root = data_dir.join("user-works");
        let outside = data_dir.join("outside.md");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::write(&outside, "outside").expect("write outside file");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        assert!(trash_workspace_node(&data_dir, &work_root.to_string_lossy()).is_err());
        assert!(trash_workspace_node(&data_dir, &outside.to_string_lossy()).is_err());
        assert!(outside.exists());
    }

    #[test]
    fn docx_plain_text_can_roundtrip() {
        let data_dir = temp_data_dir("docx-roundtrip");
        let path = data_dir.join("剧本.docx");
        write_minimal_test_docx(&path, "第一场\n对白").expect("write docx");

        assert_eq!(
            read_docx_plain_text(&path).expect("read docx"),
            "第一场\n对白"
        );

        write_docx_plain_text(&path, "第二场\n新对白").expect("save docx");
        assert_eq!(
            read_docx_plain_text(&path).expect("read saved docx"),
            "第二场\n新对白"
        );
    }

    #[test]
    fn workspace_text_and_asset_preview_reject_oversized_files() {
        let data_dir = temp_data_dir("oversized-preview");
        let text_path = data_dir.join("large.txt");
        let allowed_image_path = data_dir.join("allowed.png");
        let image_path = data_dir.join("large.png");
        fs::write(
            &text_path,
            "x".repeat((MAX_WORKSPACE_TEXT_FILE_BYTES as usize) + 1),
        )
        .expect("write large text");
        fs::write(
            &allowed_image_path,
            vec![0_u8; MAX_PREVIEW_ASSET_BYTES as usize],
        )
        .expect("write allowed image");
        fs::write(
            &image_path,
            vec![0_u8; (MAX_PREVIEW_ASSET_BYTES as usize) + 1],
        )
        .expect("write large image");

        assert!(read_workspace_text_content(&text_path).is_err());
        assert!(preview_asset_response(&allowed_image_path).is_ok());
        assert!(preview_asset_response(&image_path).is_err());
    }

    #[test]
    fn complex_docx_save_is_rejected_to_preserve_semantics() {
        let data_dir = temp_data_dir("complex-docx");
        let path = data_dir.join("复杂.docx");
        write_test_docx_document_xml(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:tbl><w:tr><w:tc><w:p><w:r><w:t>表格</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:sectPr/></w:body></w:document>"#,
        )
        .expect("write complex docx");

        assert!(write_editable_file_content(&path, "替换").is_err());
        assert!(read_docx_plain_text(&path)
            .expect("read original docx")
            .contains("表格"));
    }

    #[test]
    fn workspace_tree_skips_directory_links_when_available() {
        let data_dir = temp_data_dir("tree-link-skip");
        let work_root = data_dir.join("user-works");
        let outside = data_dir.join("outside");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::create_dir_all(&outside).expect("create outside");
        fs::write(work_root.join("正文.md"), "正文").expect("write work file");
        fs::write(outside.join("secret.md"), "secret").expect("write outside file");
        if create_dir_link(&outside, &work_root.join("linked")).is_err() {
            return;
        }
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let info = workspace_info(&data_dir).expect("workspace info");
        let names = info
            .files
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"正文.md"));
        assert!(!names.contains(&"linked"));
    }

    #[test]
    fn workspace_file_operations_reject_linked_parent_when_available() {
        let data_dir = temp_data_dir("write-link-parent");
        let work_root = data_dir.join("user-works");
        let outside = data_dir.join("outside");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::create_dir_all(&outside).expect("create outside");
        if create_dir_link(&outside, &work_root.join("linked")).is_err() {
            return;
        }
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        assert!(
            apply_workspace_write_file(&data_dir, "works", "linked/secret.md", "leak").is_err()
        );
        assert!(!outside.join("secret.md").exists());
        assert!(apply_workspace_create_folder(&data_dir, "works", "linked/generated").is_err());
        assert!(!outside.join("generated").exists());
    }

    #[test]
    fn workspace_existing_node_operations_reject_link_targets_when_available() {
        let data_dir = temp_data_dir("existing-link-target");
        let work_root = data_dir.join("user-works");
        fs::create_dir_all(&work_root).expect("create work root");
        let real_file = work_root.join("real.md");
        let link_file = work_root.join("linked.md");
        fs::write(&real_file, "正文").expect("write real file");
        if create_file_link(&real_file, &link_file).is_err() {
            return;
        }
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");
        let root = workspace_library_root(&data_dir, "works").expect("work root");

        assert!(resolve_existing_relative_workspace_node(&root, "linked.md").is_err());
        assert!(resolve_allowed_existing_node(&data_dir, &link_file.to_string_lossy()).is_err());
        assert!(real_file.exists());
    }

    #[test]
    fn workspace_info_uses_default_knowledge_root_without_user_selection() {
        let data_dir = temp_data_dir("default-knowledge-root");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");

        let info = workspace_info(&data_dir).expect("workspace info");

        assert!(info.knowledge_root_configured);
        assert!(info.knowledge_root_path.ends_with("Wridian知识库"));
        assert!(info
            .knowledge_files
            .iter()
            .any(|node| node.folder && node.name == "00知识库治理"));
    }

    #[test]
    fn workspace_tree_shows_generated_knowledge_health_reports() {
        let data_dir = temp_data_dir("generated-health-report");
        let knowledge_root = data_dir.join("user-knowledge");
        let governance_dir = knowledge_root.join("00知识库治理");
        let folds_dir = governance_dir.join("folds");
        fs::create_dir_all(&folds_dir).expect("create folds");
        fs::write(
            knowledge_root.join("hot.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_hot_cache\n---\n# hot",
        )
        .expect("write hot");
        fs::write(
            folds_dir.join("knowledge-fold-20260612.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_fold\n---\n# fold",
        )
        .expect("write fold");
        fs::write(
            governance_dir.join("知识库体检-2026-06-12.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_health_report\n---\n# 报告",
        )
        .expect("write health report");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let info = workspace_info(&data_dir).expect("workspace info");
        let governance = info
            .knowledge_files
            .iter()
            .find(|node| node.folder && node.name == "00知识库治理")
            .expect("governance folder");

        assert!(governance
            .children
            .iter()
            .any(|node| node.name == "知识库体检-2026-06-12.md"));
        assert!(!info
            .knowledge_files
            .iter()
            .any(|node| node.name == "hot.md"));
        assert!(!governance
            .children
            .iter()
            .any(|node| node.folder && node.name == "folds"));
    }

    #[test]
    fn selected_work_root_does_not_block_default_knowledge_files() {
        let data_dir = temp_data_dir("work-root-with-default-knowledge");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let work_root = data_dir.join("user-works");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::write(
            workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let knowledge_root = default_knowledge_root(&data_dir);
        let note = knowledge_root.join("00知识库治理").join("使用说明.md");
        let text = knowledge_root.join("01原始资料").join("素材.txt");
        let pdf = knowledge_root.join("01原始资料").join("资料.pdf");
        fs::create_dir_all(text.parent().unwrap()).expect("create source folder");
        fs::write(&note, "知识库说明").expect("write knowledge md");
        fs::write(&text, "原始素材").expect("write knowledge txt");
        fs::write(&pdf, b"%PDF-1.4").expect("write knowledge pdf");

        assert!(allowed_work_roots(&data_dir)
            .expect("allowed roots")
            .iter()
            .any(|root| knowledge_root.canonicalize().unwrap().starts_with(root)));
        assert_eq!(
            read_editable_file_content(
                &resolve_allowed_editable_file(&data_dir, &note.to_string_lossy())
                    .expect("knowledge markdown editable")
            )
            .expect("read knowledge markdown"),
            "知识库说明"
        );
        assert_eq!(
            read_workspace_text_content(
                &resolve_allowed_workspace_file(&data_dir, &text.to_string_lossy())
                    .expect("knowledge text preview")
            )
            .expect("read knowledge text"),
            "原始素材"
        );
        assert!(resolve_allowed_workspace_file(&data_dir, &pdf.to_string_lossy()).is_ok());
        assert!(resolve_allowed_folder(
            &data_dir,
            &knowledge_root.join("01原始资料").to_string_lossy()
        )
        .is_ok());
    }

    fn write_minimal_test_docx(path: &Path, content: &str) -> Result<(), String> {
        write_test_docx_document_xml(path, &crate::docx_xml::minimal_docx_document_xml(content))
    }

    fn write_test_docx_document_xml(path: &Path, document_xml: &str) -> Result<(), String> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut output);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            writer
                .start_file("[Content_Types].xml", options)
                .map_err(|error| error.to_string())?;
            writer
                .write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#)
                .map_err(|error| error.to_string())?;
            writer
                .start_file("word/document.xml", options)
                .map_err(|error| error.to_string())?;
            writer
                .write_all(document_xml.as_bytes())
                .map_err(|error| error.to_string())?;
            writer.finish().map_err(|error| error.to_string())?;
        }
        fs::write(path, output.into_inner()).map_err(|error| error.to_string())
    }

    #[cfg(windows)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(windows)]
    fn create_file_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }

    #[cfg(unix)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(unix)]
    fn create_file_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }
}
