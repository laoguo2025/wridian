use crate::file_lock::with_file_write_lock;
use crate::runtime::{
    default_knowledge_root, ensure_workspace, iso_timestamp, vault_root, workspace_config_path,
    wridian_data_dir,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINK_REPAIR_FILES: usize = 1000;
const MAX_LINK_REPAIR_DEPTH: usize = 10;
const MAX_LINK_REPAIR_FILE_BYTES: u64 = 512 * 1024;

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
    last_link_repair: Option<WikilinkRepairReport>,
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
    relative_path: String,
    library: String,
    folder: bool,
    children: Vec<WorkFileNode>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WikilinkRepairReport {
    renamed_from: String,
    renamed_to: String,
    changed_file_count: usize,
    changed_link_count: usize,
    rollback_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WikilinkRepairRollback {
    schema_version: u8,
    created_at: String,
    root_path: String,
    renamed_from: String,
    renamed_to: String,
    changed_files: Vec<WikilinkRepairRollbackFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WikilinkRepairRollbackFile {
    path: String,
    before: String,
    after: String,
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
    with_file_write_lock(&data_dir, &path, || {
        fs::write(&path, input.content).map_err(|error| format!("文件保存失败：{error}"))
    })?;
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
    with_file_write_lock(&data_dir, &path, || {
        fs::write(&path, "").map_err(|error| format!("文件创建失败：{error}"))
    })?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_create_work_folder(input: CreateNodeInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let parent = resolve_allowed_folder(&data_dir, &input.parent_path)?;
    let folder_name = normalize_node_name(&input.name)?;
    let path = unique_child_path(&parent, &folder_name);
    with_file_write_lock(&data_dir, &path, || {
        fs::create_dir_all(&path).map_err(|error| format!("文件夹创建失败：{error}"))
    })?;
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
    with_file_write_lock(&data_dir, &target, || {
        if source.is_dir() {
            copy_dir_recursive(&source, &target)
        } else {
            fs::copy(&source, &target)
                .map(|_| ())
                .map_err(|error| format!("文件副本创建失败：{error}"))
        }
    })?;
    workspace_info(&data_dir)
}

#[tauri::command]
pub(crate) fn wridian_rename_work_node(input: RenameNodeInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let source = resolve_allowed_existing_node(&data_dir, &input.path)?;
    let source_is_file = source.is_file();
    let root = containing_work_root(&data_dir, &source)?
        .ok_or_else(|| "文件不在当前 Wridian 工作目录内。".to_string())?;
    let old_relative = relative_path(&root, &source);
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
    let new_relative = relative_path(&root, &target);
    with_file_write_lock(&data_dir, &source, || {
        fs::rename(&source, &target).map_err(|error| format!("重命名失败：{error}"))
    })?;
    let link_repair = repair_wikilinks_after_node_move(
        &data_dir,
        &root,
        &old_relative,
        &new_relative,
        source_is_file,
    )?;
    let mut info = workspace_info(&data_dir)?;
    info.last_link_repair = Some(link_repair);
    Ok(info)
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
    with_file_write_lock(&data_dir, &source, || {
        fs::rename(&source, &target).map_err(|error| format!("移到回收站失败：{error}"))
    })?;
    workspace_info(&data_dir)
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
        last_link_repair: None,
    })
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
    let path = workspace_config_path(data_dir);
    with_file_write_lock(data_dir, &path, || {
        fs::write(&path, config).map_err(|error| format!("Wridian 工作区配置写入失败：{error}"))
    })
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
            let children = read_work_tree(&path, base, library)?;
            nodes.push(WorkFileNode {
                name,
                path: path.to_string_lossy().into_owned(),
                relative_path: relative_path(base, &path),
                library: library.to_string(),
                folder: true,
                children,
            });
        } else if is_supported_workspace_file(&path) {
            nodes.push(WorkFileNode {
                name,
                path: path.to_string_lossy().into_owned(),
                relative_path: relative_path(base, &path),
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

pub(crate) fn is_supported_writing_file(path: &Path) -> bool {
    file_extension(path)
        .map(|extension| matches!(extension.as_str(), "md" | "markdown" | "txt"))
        .unwrap_or(false)
}

fn is_supported_workspace_file(path: &Path) -> bool {
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

fn resolve_allowed_writing_file(data_dir: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_file() || !is_supported_writing_file(&path) {
        return Err("文件编辑区只能打开和保存 md、txt 文件。".to_string());
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

pub(crate) fn allowed_work_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            roots.push(
                path.canonicalize()
                    .map_err(|error| format!("作品目录解析失败：{error}"))?,
            );
        }
    }
    if let Some(root) = read_active_knowledge_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            roots.push(
                path.canonicalize()
                    .map_err(|error| format!("知识库目录解析失败：{error}"))?,
            );
        }
    }
    if roots.is_empty() {
        roots.push(
            vault_root(data_dir)
                .canonicalize()
                .map_err(|error| format!("默认写作目录解析失败：{error}"))?,
        );
        let knowledge = default_knowledge_root(data_dir);
        if knowledge.is_dir() {
            roots.push(
                knowledge
                    .canonicalize()
                    .map_err(|error| format!("知识库目录解析失败：{error}"))?,
            );
        }
    }
    Ok(roots)
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
    if !is_supported_writing_file(Path::new(&normalized)) {
        return Err("文件名只支持 md、markdown 或 txt 后缀。".to_string());
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
    parent.join(format!("{stem} {}{extension}", iso_timestamp()))
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
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

fn repair_wikilinks_after_node_move(
    data_dir: &Path,
    root: &Path,
    old_relative: &str,
    new_relative: &str,
    source_is_file: bool,
) -> Result<WikilinkRepairReport, String> {
    let markdown_files = collect_markdown_files_for_link_repair(root)?;
    let mut changed_files = Vec::new();
    let mut changed_link_count = 0;

    for path in markdown_files {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let (next_content, replacements) =
            rewrite_wikilinks_for_move(&content, old_relative, new_relative, source_is_file);
        if replacements == 0 {
            continue;
        }
        with_file_write_lock(data_dir, &path, || {
            fs::write(&path, &next_content)
                .map_err(|error| format!("链接修复写入失败（{}）：{error}", path.to_string_lossy()))
        })?;
        changed_link_count += replacements;
        changed_files.push(WikilinkRepairRollbackFile {
            path: path.to_string_lossy().into_owned(),
            before: content,
            after: next_content,
        });
    }

    let changed_file_count = changed_files.len();
    let rollback_path = if changed_files.is_empty() {
        None
    } else {
        Some(write_wikilink_repair_rollback(
            data_dir,
            root,
            old_relative,
            new_relative,
            changed_files,
        )?)
    };

    Ok(WikilinkRepairReport {
        renamed_from: old_relative.to_string(),
        renamed_to: new_relative.to_string(),
        changed_file_count,
        changed_link_count,
        rollback_path,
    })
}

fn write_wikilink_repair_rollback(
    data_dir: &Path,
    root: &Path,
    old_relative: &str,
    new_relative: &str,
    changed_files: Vec<WikilinkRepairRollbackFile>,
) -> Result<String, String> {
    let rollback_dir = root.join(".wridian").join("link-repair");
    fs::create_dir_all(&rollback_dir)
        .map_err(|error| format!("链接修复回滚目录创建失败：{error}"))?;
    let path = rollback_dir.join(format!("{}.json", iso_timestamp()));
    let rollback = WikilinkRepairRollback {
        schema_version: 1,
        created_at: iso_timestamp(),
        root_path: root.to_string_lossy().into_owned(),
        renamed_from: old_relative.to_string(),
        renamed_to: new_relative.to_string(),
        changed_files,
    };
    let content = serde_json::to_string_pretty(&rollback).map_err(|error| error.to_string())?;
    with_file_write_lock(data_dir, &path, || {
        fs::write(&path, content).map_err(|error| format!("链接修复回滚记录写入失败：{error}"))
    })?;
    Ok(path.to_string_lossy().into_owned())
}

fn collect_markdown_files_for_link_repair(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_markdown_files_for_link_repair_recursive(root, 0, &mut files)?;
    Ok(files)
}

fn collect_markdown_files_for_link_repair_recursive(
    current: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if depth > MAX_LINK_REPAIR_DEPTH || files.len() >= MAX_LINK_REPAIR_FILES || !current.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|error| format!("链接修复目录读取失败：{error}"))?
    {
        if files.len() >= MAX_LINK_REPAIR_FILES {
            break;
        }
        let entry = entry.map_err(|error| format!("链接修复目录项读取失败：{error}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if should_skip_entry(&name) {
            continue;
        }
        if path.is_dir() {
            collect_markdown_files_for_link_repair_recursive(&path, depth + 1, files)?;
            continue;
        }
        if !is_markdown_workspace_file(&path) {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() || metadata.len() > MAX_LINK_REPAIR_FILE_BYTES {
            continue;
        }
        files.push(path);
    }
    Ok(())
}

fn rewrite_wikilinks_for_move(
    content: &str,
    old_relative: &str,
    new_relative: &str,
    source_is_file: bool,
) -> (String, usize) {
    let mut output = String::with_capacity(content.len());
    let mut rest = content;
    let mut replacements = 0;

    while let Some(start) = rest.find("[[") {
        output.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            output.push_str("[[");
            output.push_str(rest);
            return (output, replacements);
        };
        let inner = &rest[..end];
        if let Some(next_inner) =
            rewrite_wikilink_inner_for_move(inner, old_relative, new_relative, source_is_file)
        {
            output.push_str("[[");
            output.push_str(&next_inner);
            output.push_str("]]");
            replacements += 1;
        } else {
            output.push_str("[[");
            output.push_str(inner);
            output.push_str("]]");
        }
        rest = &rest[end + 2..];
    }
    output.push_str(rest);
    (output, replacements)
}

fn rewrite_wikilink_inner_for_move(
    inner: &str,
    old_relative: &str,
    new_relative: &str,
    source_is_file: bool,
) -> Option<String> {
    let (target_and_suffix, alias) = split_once(inner, '|');
    let (target, suffix) = split_once(target_and_suffix, '#');
    let next_target =
        next_wikilink_target_for_move(target, old_relative, new_relative, source_is_file)?;
    let mut output = next_target;
    if let Some(suffix) = suffix {
        output.push('#');
        output.push_str(suffix);
    }
    if let Some(alias) = alias {
        output.push('|');
        output.push_str(alias);
    }
    Some(output)
}

fn split_once(value: &str, delimiter: char) -> (&str, Option<&str>) {
    match value.split_once(delimiter) {
        Some((left, right)) => (left, Some(right)),
        None => (value, None),
    }
}

fn next_wikilink_target_for_move(
    target: &str,
    old_relative: &str,
    new_relative: &str,
    source_is_file: bool,
) -> Option<String> {
    let normalized = normalize_wikilink_target(target);
    if normalized.is_empty() {
        return None;
    }
    let old_path = strip_markdown_extension(&normalize_relative_link_path(old_relative));
    let new_path = strip_markdown_extension(&relative_link_output_path(new_relative));
    if source_is_file {
        if normalized == old_path.to_lowercase() {
            return Some(new_path.clone());
        }
        let old_stem = file_stem_text(old_relative)?;
        let new_stem = file_stem_text(new_relative)?;
        if normalized == old_stem.to_lowercase() {
            return Some(new_stem);
        }
        return None;
    }

    if normalized == old_path.to_lowercase() {
        return Some(new_path.clone());
    }
    let old_prefix = format!("{}/", old_path.to_lowercase());
    if normalized.starts_with(&old_prefix) {
        return Some(format!("{}{}", new_path, &normalized[old_path.len()..]));
    }
    None
}

fn normalize_wikilink_target(target: &str) -> String {
    strip_markdown_extension(
        target
            .trim()
            .trim_start_matches("./")
            .replace('\\', "/")
            .to_lowercase()
            .as_str(),
    )
}

fn normalize_relative_link_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_lowercase()
}

fn relative_link_output_path(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

fn strip_markdown_extension(value: &str) -> String {
    let lower = value.to_lowercase();
    if lower.ends_with(".markdown") {
        return value[..value.len() - ".markdown".len()].to_string();
    }
    if lower.ends_with(".md") {
        return value[..value.len() - ".md".len()].to_string();
    }
    value.to_string()
}

fn file_stem_text(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
}

fn is_markdown_workspace_file(path: &Path) -> bool {
    file_extension(path)
        .map(|extension| matches!(extension.as_str(), "md" | "markdown"))
        .unwrap_or(false)
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
    fn workspace_tree_displays_common_files_but_edits_only_text_notes() {
        let data_dir = temp_data_dir("common-files");
        let work_root = data_dir.join("user-works");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::write(work_root.join("方案.md"), "正文").expect("write md");
        fs::write(work_root.join("资料.txt"), "文本").expect("write txt");
        fs::write(work_root.join("报告.pdf"), b"pdf").expect("write pdf");
        fs::write(work_root.join("合同.docx"), b"docx").expect("write docx");
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
        assert!(resolve_allowed_writing_file(
            &data_dir,
            &work_root.join("方案.md").to_string_lossy()
        )
        .is_ok());
        assert!(resolve_allowed_writing_file(
            &data_dir,
            &work_root.join("资料.txt").to_string_lossy()
        )
        .is_ok());
        assert!(resolve_allowed_writing_file(
            &data_dir,
            &work_root.join("报告.pdf").to_string_lossy()
        )
        .is_err());
        assert!(resolve_allowed_writing_file(
            &data_dir,
            &work_root.join("合同.docx").to_string_lossy()
        )
        .is_err());
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
    fn wikilink_rewrite_preserves_aliases_and_repairs_path_links() {
        let content = "[[A]] [[A|别名]] [[旧目录/A]] [[旧目录/A#段落|显示]]";

        let (rewritten, count) =
            rewrite_wikilinks_for_move(content, "旧目录/A.md", "新目录/B.md", true);

        assert_eq!(count, 4);
        assert_eq!(
            rewritten,
            "[[B]] [[B|别名]] [[新目录/B]] [[新目录/B#段落|显示]]"
        );
    }

    #[test]
    fn repair_wikilinks_writes_rollback_record() {
        let data_dir = temp_data_dir("link-repair-data");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let root = data_dir.join("work");
        fs::create_dir_all(&root).expect("create work root");
        fs::write(root.join("A.md"), "正文").expect("write target");
        fs::write(root.join("引用.md"), "[[A]] 和 [[A|别名]]").expect("write reference");

        let report = repair_wikilinks_after_node_move(&data_dir, &root, "A.md", "B.md", true)
            .expect("repair links");
        let rewritten = fs::read_to_string(root.join("引用.md")).expect("read reference");

        assert_eq!(report.changed_file_count, 1);
        assert_eq!(report.changed_link_count, 2);
        assert_eq!(rewritten, "[[B]] 和 [[B|别名]]");
        let rollback_path = report.rollback_path.expect("rollback path");
        let rollback = fs::read_to_string(rollback_path).expect("read rollback");
        assert!(rollback.contains("[[A]] 和 [[A|别名]]"));
        assert!(rollback.contains("[[B]] 和 [[B|别名]]"));

        let _ = fs::remove_dir_all(&root);
    }
}
