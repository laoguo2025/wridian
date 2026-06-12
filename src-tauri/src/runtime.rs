use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const WRIDIAN_DATA_DIR_NAME: &str = "Wridian";
const WRIDIAN_VAULT_DIR_NAME: &str = "Wridian Vault";
const WRIDIAN_RUNTIME_DIR_NAME: &str = ".wridian";
const DEFAULT_KNOWLEDGE_DIR_NAME: &str = "Wridian知识库";
const DEFAULT_KNOWLEDGE_CATEGORIES: &[(&str, Option<&str>)] = &[
    (
        "00知识库治理",
        Some(
            "# Wridian 知识库使用说明\n\n这个文件夹用于保存知识库规则、运营记录、体检结果和分类说明。\n\n- `01原始资料`：存放未加工的一手资料。\n- `02拆解报告`：存放作品拆解、案例分析和中间报告。\n- `03故事模型`：存放可复用的故事结构、叙事模型和判断框架。\n- `04人物原型`：存放人物类型、关系模式、角色弧光和人设参考。\n- `05情节方程`：存放冲突、反转、钩子、节奏和情节公式。\n- `06写作技法`：存放对白、场景、风格、爽点、短剧卡点等技法卡。\n- `07综合素材`：存放可复用素材、清单、灵感和跨分类资料。\n- `08大神蒸馏`：存放作者方法论、作者 skill 和版本记录。\n- `09文件归档`：存放废弃、过期或待删除文件。\n\n这些分类只是默认模板。你可以在 Wridian 里新增、改名或移除分类文件夹；知识库体检会按实际目录给出修复建议。\n",
        ),
    ),
    ("01原始资料", None),
    ("02拆解报告", None),
    ("03故事模型", None),
    ("04人物原型", None),
    ("05情节方程", None),
    ("06写作技法", None),
    ("07综合素材", None),
    ("08大神蒸馏", None),
    ("09文件归档", None),
];

pub(crate) fn wridian_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|path| path.join(WRIDIAN_DATA_DIR_NAME))
        .ok_or_else(|| "无法定位 Wridian 数据目录。".to_string())
}

pub(crate) fn vault_root(data_dir: &Path) -> PathBuf {
    data_dir.join(WRIDIAN_VAULT_DIR_NAME)
}

pub(crate) fn knowledge_root(data_dir: &Path) -> PathBuf {
    vault_root(data_dir).join("knowledge")
}

pub(crate) fn default_knowledge_root(data_dir: &Path) -> PathBuf {
    #[cfg(test)]
    {
        data_dir.join(DEFAULT_KNOWLEDGE_DIR_NAME)
    }
    #[cfg(not(test))]
    {
        let d_drive = PathBuf::from(r"D:\");
        if d_drive.is_dir() {
            d_drive.join(DEFAULT_KNOWLEDGE_DIR_NAME)
        } else {
            knowledge_root(data_dir)
        }
    }
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

pub(crate) fn memory_folder_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir)
}

pub(crate) fn memory_wiki_root(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("wiki")
}

pub(crate) fn ensure_workspace(data_dir: &Path) -> Result<(), String> {
    let vault = vault_root(data_dir);
    let works = vault.join("works");
    let knowledge = default_knowledge_root(data_dir);
    let runtime = runtime_root(data_dir);
    let sessions = runtime.join("sessions");
    let episodes = runtime.join("episodes");
    let chat = runtime.join("chat");
    let wiki = memory_wiki_root(data_dir);
    let memory_tree = runtime.join("memory-tree");
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
        &memory_tree,
        &wiki,
        &wiki_sources,
        &wiki_entities,
        &wiki_concepts,
    ] {
        fs::create_dir_all(dir).map_err(|error| format!("Wridian 目录创建失败：{error}"))?;
    }

    write_if_missing(
        &memory_tree.join("global").join("AGENTS.md"),
        "# AGENTS.md\n\n这里记录 Wridian 全局工作区规则、上下文边界和不可违反的长期协作原则。\n",
    )?;
    write_if_missing(
        &memory_tree.join("global").join("MEMORY.md"),
        "# MEMORY.md\n\n这里记录普通聊天的全局长期记忆，不归属于任何单个作品。\n",
    )?;
    write_if_missing(
        &memory_tree.join("global").join("AWARENESS.md"),
        "# AWARENESS.md\n\n这里记录长期反思、稳定变化和跨作品意识线索。\n",
    )?;
    write_if_missing(
        &memory_tree.join("partner").join("soul.md"),
        "# soul.md\n\n这里定义 Wridian 作为对话伙伴的底层人格、判断原则和表达气质。\n",
    )?;
    write_if_missing(
        &memory_tree.join("partner").join("user.md"),
        "# user.md\n\n这里记录用户画像、创作身份、工作节奏、语言偏好和审美偏好。\n",
    )?;
    write_if_missing(
        &memory_tree.join("partner").join("relationship.md"),
        "# relationship.md\n\n这里记录你和 Wridian 的关系校准。用户的关系校准优先于默认人格。\n\n## Names\n\n## Register\n\n## Drift Warnings\n\n## Canonical Anchor\n",
    )?;
    write_if_missing(
        &memory_tree.join("partner").join("partnermemory.md"),
        "# partnermemory.md\n\n这里记录 Wridian 与用户长期对话过程中形成的伙伴记忆。\n",
    )?;
    write_if_missing(
        &vault.join("user.md"),
        "# 关于你\n\n这里记录长期稳定的用户偏好、称呼、写作方向和沟通习惯。\n",
    )?;
    write_if_missing(
        &vault.join("creative.md"),
        "# 创作记忆\n\n## 方法\n\n## 审美\n\n## 禁区\n",
    )?;
    ensure_default_knowledge_categories(&knowledge)?;
    write_if_missing(
        &runtime.join("active-context.json"),
        &serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "currentWork": null,
            "currentChapter": null,
            "lastUserIntent": null,
            "lastAssistantJudgement": null,
            "nextStep": null
        }))
        .map_err(|error| error.to_string())?,
    )?;
    write_if_missing(&wiki.join("index.md"), "# Wridian 记忆索引\n\n")?;
    write_if_missing(&wiki.join("hot.md"), "# Hot Context\n\n")?;
    write_if_missing(&wiki.join("log.md"), "# 记忆同步日志\n\n")?;
    Ok(())
}

pub(crate) fn ensure_default_knowledge_categories(root: &Path) -> Result<(), String> {
    let should_seed = !root.exists();
    fs::create_dir_all(root).map_err(|error| format!("知识库目录创建失败：{error}"))?;
    if !should_seed {
        return Ok(());
    }
    for (name, readme) in DEFAULT_KNOWLEDGE_CATEGORIES {
        let dir = root.join(name);
        fs::create_dir_all(&dir).map_err(|error| format!("知识库分类目录创建失败：{error}"))?;
        if let Some(content) = readme {
            write_if_missing(&dir.join("使用说明.md"), content)?;
        }
    }
    Ok(())
}

pub(crate) fn iso_timestamp() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

pub(crate) fn filename_timestamp() -> String {
    chrono::Local::now().format("%Y%m%dT%H%M%S").to_string()
}

pub(crate) fn unix_timestamp_seconds() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
pub(crate) fn unique_test_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        nanos,
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_knowledge_categories_seed_only_missing_roots() {
        let root = std::env::temp_dir().join(format!(
            "wridian-runtime-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create root");
        fs::create_dir_all(root.join("自定义分类")).expect("create custom folder");

        ensure_default_knowledge_categories(&root).expect("ensure categories");

        assert!(root.join("自定义分类").is_dir());
        assert!(!root.join("00知识库治理").exists());

        let _ = fs::remove_dir_all(&root);

        ensure_default_knowledge_categories(&root).expect("seed missing root");
        assert!(root.join("00知识库治理").join("使用说明.md").is_file());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn timestamps_distinguish_iso_filename_and_unix_seconds() {
        let iso = iso_timestamp();
        assert!(iso.contains('T'));
        assert!(iso.contains('-'));
        assert!(!iso.chars().all(|ch| ch.is_ascii_digit()));

        let filename = filename_timestamp();
        assert_eq!(filename.len(), "20260612T193001".len());
        assert!(filename.chars().all(|ch| ch.is_ascii_digit() || ch == 'T'));

        assert!(unix_timestamp_seconds() > 1_700_000_000);
    }
}
