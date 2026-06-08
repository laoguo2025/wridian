use crate::runtime::{
    ensure_workspace, iso_timestamp, vault_root, workspace_config_path, wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInfo {
    vault_path: String,
    runtime_path: String,
    files_root_path: String,
    active_work_root: Option<String>,
    files: Vec<WorkFileNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetWorkRootInput {
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveFileResponse {
    ok: bool,
    saved_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkFileNode {
    name: String,
    path: String,
    folder: bool,
    children: Vec<WorkFileNode>,
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
    write_workspace_config(&data_dir, &root)?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_open_file(input: FilePathInput) -> Result<OpenFileResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_writing_file(&data_dir, &input.path)?;
    let content = fs::read_to_string(&path).map_err(|error| format!("文件读取失败：{error}"))?;
    Ok(OpenFileResponse {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "未命名".to_string()),
        path: path.to_string_lossy().into_owned(),
        content,
    })
}

#[tauri::command]
pub(crate) fn wridian_save_file(input: SaveFileInput) -> Result<SaveFileResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = resolve_allowed_writing_file(&data_dir, &input.path)?;
    fs::write(&path, input.content).map_err(|error| format!("文件保存失败：{error}"))?;
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
        copy_dir_recursive(&source, &target)?;
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
        normalize_file_name(&input.new_name)?
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
    let source = resolve_allowed_existing_node(&data_dir, &input.path)?;
    let root = containing_work_root(&data_dir, &source)?
        .ok_or_else(|| "文件不在当前 Wridian 工作目录内。".to_string())?;
    if source == root {
        return Err("不能移动工作区根目录。".to_string());
    }
    let trash = root.join(".wridian-trash");
    fs::create_dir_all(&trash).map_err(|error| format!("回收站创建失败：{error}"))?;
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| "不能移动工作区根目录。".to_string())?;
    let target = unique_child_path(&trash, &format!("{}-{name}", iso_timestamp()));
    fs::rename(&source, &target).map_err(|error| format!("移到回收站失败：{error}"))?;
    workspace_info(&data_dir)
}

fn workspace_info(data_dir: &Path) -> Result<WorkspaceInfo, String> {
    let files_root = files_root(data_dir)?;
    Ok(WorkspaceInfo {
        vault_path: vault_root(data_dir).to_string_lossy().into_owned(),
        runtime_path: crate::runtime::runtime_root(data_dir)
            .to_string_lossy()
            .into_owned(),
        files_root_path: files_root.to_string_lossy().into_owned(),
        active_work_root: read_active_work_root(data_dir)?,
        files: read_work_tree(&files_root)?,
    })
}

fn write_workspace_config(data_dir: &Path, root: &Path) -> Result<(), String> {
    let config = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "activeWorkRoot": root.to_string_lossy()
    }))
    .map_err(|error| error.to_string())?;
    fs::write(workspace_config_path(data_dir), config)
        .map_err(|error| format!("Wridian 工作区配置写入失败：{error}"))
}

fn read_active_work_root(data_dir: &Path) -> Result<Option<String>, String> {
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

fn files_root(data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return Ok(path);
        }
    }
    Ok(vault_root(data_dir).join("works"))
}

fn read_work_tree(root: &Path) -> Result<Vec<WorkFileNode>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut nodes = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| format!("作品目录读取失败：{error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("作品目录读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if should_skip_entry(&name) {
            continue;
        }
        if path.is_dir() {
            let children = read_work_tree(&path)?;
            if !children.is_empty() {
                nodes.push(WorkFileNode {
                    name,
                    path: path.to_string_lossy().into_owned(),
                    folder: true,
                    children,
                });
            }
        } else if is_supported_writing_file(&path) {
            nodes.push(WorkFileNode {
                name,
                path: path.to_string_lossy().into_owned(),
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

fn is_supported_writing_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "txt" | "fountain"
            )
        })
        .unwrap_or(false)
}

fn resolve_allowed_writing_file(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_file() || !is_supported_writing_file(&path) {
        return Err("只能打开和保存 md、txt 或 fountain 写作文件。".to_string());
    }
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
    if path.is_file() && !is_supported_writing_file(&path) {
        return Err("只能操作 md、txt 或 fountain 写作文件。".to_string());
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

fn allowed_work_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = vec![vault_root(data_dir)
        .canonicalize()
        .map_err(|error| format!("默认写作目录解析失败：{error}"))?];
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            roots.push(
                path.canonicalize()
                    .map_err(|error| format!("作品目录解析失败：{error}"))?,
            );
        }
    }
    Ok(roots)
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
    if !is_supported_writing_file(Path::new(&normalized)) {
        return Err("文件名只支持 md、markdown、txt 或 fountain 后缀。".to_string());
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
    parent.join(format!("{stem} {}{extension}", iso_timestamp()))
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| format!("文件夹副本创建失败：{error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("文件夹读取失败：{error}"))? {
        let entry = entry.map_err(|error| format!("文件夹读取失败：{error}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)
                .map_err(|error| format!("文件副本创建失败：{error}"))?;
        }
    }
    Ok(())
}
