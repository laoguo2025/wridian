use crate::runtime::{ensure_workspace, runtime_root, vault_root, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenLocalPathInput {
    path: String,
}

#[tauri::command]
pub(crate) fn wridian_open_local_path(input: OpenLocalPathInput) -> Result<(), String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = allowed_local_open_path(&data_dir, &input.path)?;
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|error| format!("打开本地路径失败：{error}"))
}

#[tauri::command]
pub(crate) fn wridian_open_memory_tree_folder() -> Result<(), String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let path = runtime_root(&data_dir).join("memory-tree");
    fs::create_dir_all(&path).map_err(|error| format!("记忆文件夹创建失败：{error}"))?;
    tauri_plugin_opener::open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| format!("打开记忆文件夹失败：{error}"))
}

fn allowed_local_open_path(data_dir: &Path, requested: &str) -> Result<String, String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err("打开路径不能为空。".to_string());
    }
    let requested_path = PathBuf::from(requested);
    if !requested_path.is_absolute() {
        return Err("只能打开绝对路径。".to_string());
    }
    let canonical_requested = requested_path
        .canonicalize()
        .map_err(|error| format!("打开路径解析失败：{error}"))?;
    let allowed_roots = allowed_open_roots(data_dir)?;
    if allowed_roots
        .iter()
        .any(|root| canonical_requested.starts_with(root))
    {
        return Ok(canonical_requested.to_string_lossy().into_owned());
    }
    Err("只能打开 Wridian 当前作品库、知识库或运行数据目录内的文件。".to_string())
}

fn allowed_open_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    let runtime = runtime_root(data_dir);
    if runtime.is_dir() {
        roots.push(canonical_dir(&runtime, "Wridian 运行目录")?);
    }
    if let Some(work_root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(work_root);
        if path.is_dir() {
            roots.push(canonical_dir(&path, "作品库目录")?);
        }
    }
    let default_work_root = vault_root(data_dir).join("works");
    if default_work_root.is_dir() {
        roots.push(canonical_dir(&default_work_root, "默认作品库目录")?);
    }
    let knowledge_root = resolved_knowledge_root(data_dir)?;
    if knowledge_root.is_dir() {
        roots.push(canonical_dir(&knowledge_root, "知识库目录")?);
    }
    Ok(roots)
}

fn canonical_dir(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("{label}解析失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-opener-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn allowed_local_open_path_accepts_runtime_child() {
        let data_dir = temp_dir("runtime");
        let runtime = runtime_root(&data_dir);
        fs::create_dir_all(&runtime).expect("create runtime");
        let file = runtime.join("memory-tree").join("SOUL.md");
        fs::create_dir_all(file.parent().unwrap()).expect("create parent");
        fs::write(&file, "ok").expect("write file");

        let opened = allowed_local_open_path(&data_dir, &file.to_string_lossy()).expect("allowed");

        assert!(opened.ends_with("SOUL.md"));
    }

    #[test]
    fn allowed_local_open_path_rejects_outside_path() {
        let data_dir = temp_dir("outside-data");
        fs::create_dir_all(runtime_root(&data_dir)).expect("create runtime");
        let outside = temp_dir("outside").join("secret.md");
        fs::write(&outside, "secret").expect("write outside");

        let result = allowed_local_open_path(&data_dir, &outside.to_string_lossy());

        assert!(result.is_err());
    }
}
