use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const WRIDIAN_DATA_DIR_NAME: &str = "Wridian";
const WRIDIAN_VAULT_DIR_NAME: &str = "Wridian Vault";
const WRIDIAN_RUNTIME_DIR_NAME: &str = ".wridian";

pub(crate) fn wridian_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|path| path.join(WRIDIAN_DATA_DIR_NAME))
        .ok_or_else(|| "无法定位 Wridian 数据目录。".to_string())
}

pub(crate) fn vault_root(data_dir: &Path) -> PathBuf {
    data_dir.join(WRIDIAN_VAULT_DIR_NAME)
}

pub(crate) fn runtime_root(data_dir: &Path) -> PathBuf {
    data_dir.join(WRIDIAN_RUNTIME_DIR_NAME)
}

pub(crate) fn workspace_config_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("workspace.json")
}

pub(crate) fn model_accounts_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("model-accounts.json")
}

pub(crate) fn memory_tree_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("memory-tree.json")
}

pub(crate) fn candidates_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("candidates.json")
}

pub(crate) fn memory_folder_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir)
}

pub(crate) fn memory_wiki_root(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("wiki")
}

pub(crate) fn ensure_workspace(data_dir: &Path) -> Result<(), String> {
    let vault = vault_root(data_dir);
    let works = vault.join("works");
    let runtime = runtime_root(data_dir);
    let sessions = runtime.join("sessions");
    let episodes = runtime.join("episodes");
    let chat = runtime.join("chat");
    let wiki = memory_wiki_root(data_dir);
    let wiki_sources = wiki.join("sources");
    let wiki_entities = wiki.join("entities");
    let wiki_concepts = wiki.join("concepts");

    for dir in [
        &vault,
        &works,
        &runtime,
        &sessions,
        &episodes,
        &chat,
        &wiki,
        &wiki_sources,
        &wiki_entities,
        &wiki_concepts,
    ] {
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
        &memory_tree_path(data_dir),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "memories": []
        }))
        .map_err(|error| error.to_string())?,
    )?;
    write_if_missing(
        &candidates_path(data_dir),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "items": []
        }))
        .map_err(|error| error.to_string())?,
    )?;
    write_if_missing(&wiki.join("index.md"), "# Wridian 记忆索引\n\n")?;
    write_if_missing(&wiki.join("hot.md"), "# Hot Context\n\n")?;
    write_if_missing(&wiki.join("log.md"), "# 记忆同步日志\n\n")?;
    Ok(())
}

pub(crate) fn iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

pub(crate) fn next_runtime_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos}")
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
