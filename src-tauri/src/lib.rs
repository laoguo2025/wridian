use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const WRIDIAN_DATA_DIR_NAME: &str = "Wridian";
const WRIDIAN_VAULT_DIR_NAME: &str = "Wridian Vault";
const WRIDIAN_RUNTIME_DIR_NAME: &str = ".wridian";

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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WorkFileNode {
    name: String,
    path: String,
    folder: bool,
    children: Vec<WorkFileNode>,
}

#[tauri::command]
fn wridian_init_workspace() -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    Ok(WorkspaceInfo {
        vault_path: vault_root(&data_dir).to_string_lossy().into_owned(),
        runtime_path: runtime_root(&data_dir).to_string_lossy().into_owned(),
        active_work_root: read_active_work_root(&data_dir)?,
        files: read_workspace_files(&data_dir)?,
    })
}

#[tauri::command]
fn wridian_set_work_root(input: SetWorkRootInput) -> Result<WorkspaceInfo, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = PathBuf::from(input.path.trim());
    if !root.is_dir() {
        return Err("请选择一个存在的本地文件夹。".to_string());
    }
    write_workspace_config(&data_dir, &root)?;
    Ok(WorkspaceInfo {
        vault_path: vault_root(&data_dir).to_string_lossy().into_owned(),
        runtime_path: runtime_root(&data_dir).to_string_lossy().into_owned(),
        active_work_root: Some(root.to_string_lossy().into_owned()),
        files: read_work_tree(&root)?,
    })
}

fn wridian_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|path| path.join(WRIDIAN_DATA_DIR_NAME))
        .ok_or_else(|| "无法定位 Wridian 数据目录。".to_string())
}

fn vault_root(data_dir: &Path) -> PathBuf {
    data_dir.join(WRIDIAN_VAULT_DIR_NAME)
}

fn runtime_root(data_dir: &Path) -> PathBuf {
    data_dir.join(WRIDIAN_RUNTIME_DIR_NAME)
}

fn ensure_workspace(data_dir: &Path) -> Result<(), String> {
    let vault = vault_root(data_dir);
    let works = vault.join("works");
    let runtime = runtime_root(data_dir);
    let sessions = runtime.join("sessions");
    let episodes = runtime.join("episodes");

    for dir in [&vault, &works, &runtime, &sessions, &episodes] {
        fs::create_dir_all(dir).map_err(|error| format!("Wridian 目录创建失败：{error}"))?;
    }

    write_if_missing(
        &vault.join("user.md"),
        "# 关于你\n\n这里记录长期稳定的用户偏好、称呼、写作方向和沟通习惯。\n",
    )?;
    write_if_missing(
        &vault.join("creative.md"),
        "# 创作记忆\n\n## 方法\n\n## 审美\n\n## 禁区\n",
    )?;
    write_if_missing(
        &works.join("雾城手记.md"),
        "# 雾城手记\n\n## 作品状态\n\n- 当前示例章节：第三章：雨夜。\n\n## 人物\n\n## 设定\n\n## 伏笔\n\n## 开放问题\n",
    )?;
    write_if_missing(
        &runtime.join("active-context.json"),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "currentWork": "雾城手记",
            "currentChapter": "第三章：雨夜",
            "lastUserIntent": null,
            "lastAssistantJudgement": null,
            "nextStep": null
        }))
        .map_err(|error| error.to_string())?,
    )?;
    write_if_missing(
        &runtime.join("memory-tree.json"),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "nodes": [
                { "id": "work-wucheng", "kind": "work", "title": "雾城手记", "source": "works/雾城手记.md" }
            ]
        }))
        .map_err(|error| error.to_string())?,
    )?;
    write_if_missing(
        &runtime.join("candidates.json"),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "items": []
        }))
        .map_err(|error| error.to_string())?,
    )?;
    Ok(())
}

fn workspace_config_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("workspace.json")
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

fn write_if_missing(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Wridian 目录创建失败：{error}"))?;
    }
    fs::write(path, content).map_err(|error| format!("Wridian 文件写入失败：{error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            wridian_init_workspace,
            wridian_set_work_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
