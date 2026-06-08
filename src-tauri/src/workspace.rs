use crate::runtime::{
    ensure_workspace, iso_timestamp, vault_root, workspace_config_path, wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    vault_path: String,
    runtime_path: String,
    active_work_root: Option<String>,
    files: Vec<WorkFileNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetWorkRootInput {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilePathInput {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenFileResponse {
    path: String,
    name: String,
    content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveFileResponse {
    ok: bool,
    saved_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WorkFileNode {
    name: String,
    path: String,
    folder: bool,
    children: Vec<WorkFileNode>,
}

#[tauri::command]
pub(crate) fn wridian_init_workspace() -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    Ok(WorkspaceInfo {
        vault_path: vault_root(&data_dir).to_string_lossy().into_owned(),
        runtime_path: crate::runtime::runtime_root(&data_dir)
            .to_string_lossy()
            .into_owned(),
        active_work_root: read_active_work_root(&data_dir)?,
        files: read_workspace_files(&data_dir)?,
    })
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
    Ok(WorkspaceInfo {
        vault_path: vault_root(&data_dir).to_string_lossy().into_owned(),
        runtime_path: crate::runtime::runtime_root(&data_dir)
            .to_string_lossy()
            .into_owned(),
        active_work_root: Some(root.to_string_lossy().into_owned()),
        files: read_work_tree(&root)?,
    })
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

fn read_workspace_files(data_dir: &Path) -> Result<Vec<WorkFileNode>, String> {
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return read_work_tree(&path);
        }
    }
    read_work_tree(&vault_root(data_dir).join("works"))
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
    matches!(name, ".git" | "node_modules" | ".wridian") || name.starts_with('.')
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
