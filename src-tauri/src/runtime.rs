use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const WRIDIAN_DATA_DIR_NAME: &str = "Wridian";
const WRIDIAN_VAULT_DIR_NAME: &str = "Wridian Vault";
const WRIDIAN_RUNTIME_DIR_NAME: &str = ".wridian";
const DEFAULT_KNOWLEDGE_DIR_NAME: &str = "Wridian知识库";
const DEFAULT_KNOWLEDGE_CATEGORIES: &[&str] = &[
    "00知识库治理",
    "01原始资料",
    "02拆解报告",
    "03故事模型",
    "04人物原型",
    "05情节方程",
    "06写作技法",
    "07综合素材",
    "08大神蒸馏",
    "09文件归档",
];
const DEFAULT_KNOWLEDGE_GOVERNANCE: &str = "# 知识库治理说明\n\n## 常用口令\n\n```text\n搭建知识库\n拆解作品\n提炼知识卡\n蒸馏大神作者\n安装大神skill\n体检知识库\n进化skill\n清理知识库\n```\n\n## 一级目录\n\n- `00知识库治理`：治理说明、调用记录台账、体检结果和运营记录。\n- `01原始资料`：待处理素材。\n- `02拆解报告`：作品拆解笔记与综合报告。\n- `03故事模型`：可复用故事运行机制。\n- `04人物原型`：人物位置、关系功能和精神内核。\n- `05情节方程`：场景、桥段和情绪触发公式。\n- `06写作技法`：可执行写作技法、组合流程和审美法则。\n- `07综合素材`：设定、道具、机构、术语、场景和金句。\n- `08大神蒸馏`：作者方法论和可复用 skill。\n- `09文件归档`：备份、迁移记录、待清理文件和旧版本。\n\n## 知识治理\n\n1. 文件系统是唯一事实来源。\n2. 用户可以增、改、删分类目录，体检时按实际目录修正。\n3. 知识卡可以被多个作品引用，但不会自动变成作品记忆。\n4. 从知识到作品，通过引用、采纳或改写成作品设定进入项目。\n5. 从作品到知识，通过摘录、抽象或沉淀为知识卡离开项目。\n\n## 知识卡结构\n\n知识卡应写清四件事：\n\n- 输入：什么场景、材料或问题可以调用这张卡。\n- 处理逻辑：卡片如何判断、拆解或生成方案。\n- 输出：调用后能得到什么结果。\n- 边界：什么情况下会失效、误用或需要回源复查。\n\n## 旧目录处理\n\n发现旧版 00-11、重名目录或已被合并的分类时，先迁移到当前 00-09 结构；不能确认归属的文件先放入 `09文件归档`，不要直接物理删除。\n\n## 重要规则\n\n- 拆解产物进 `02拆解报告`。\n- 知识卡进 `03-07`，只保留 S 级。\n- 作者 skill 由蒸馏流程生成，存入 `08大神蒸馏`。\n- 清理默认归档，不直接物理删除。\n";
const DEFAULT_KNOWLEDGE_CALL_LOG: &str = "# Wridian知识库 · 调用记录台账\n\n> 本台账记录知识卡被真实使用后的表现，用于统计调用频率、最近调用、调用表现和进化方向。字段是管理台账字段，不是知识卡 frontmatter。\n\n## 记录规则\n\n1. 只有知识卡被真实用于拆解、创作、诊断、改写或方案判断时，才记录一次调用。\n2. 同一任务里同一张知识卡多次被参考，默认记为一次；若不同环节发挥不同作用，可以拆成多条。\n3. 调用表现只看本次任务效果，不看卡片文字是否漂亮。\n4. 进化方向必须写成可执行动作，避免只写“优化”“完善”。\n5. 如果一张卡误导判断，必须记录，后续全库体检优先处理。\n\n## 调用记录表\n\n| 日期 | 调用作品 | 知识卡 | 调用频率 | 最近调用 | 调用表现 | 关联卡片 | 进化方向 | 备注 |\n|---|---|---|---:|---|---|---|---|---|\n|  |  |  |  |  |  |  |  |  |\n\n## 质量 × 频率四象限\n\n| 质量 | 频率 | 结果 | 处理原则 |\n|---|---|---|---|\n| 低质量 | 低频 | 垃圾 | 废弃、合并或封存。 |\n| 低质量 | 高频 | 内耗 | 优先重写或降评级。 |\n| 高质量 | 低频 | 浪费 | 补入口、补关联卡片、补适用场景。 |\n| 高质量 | 高频 | 杠杆 | 评为金卡，重点维护，优先产品化和系统化。 |\n";
const DEFAULT_DASHEN_INDEX: &str = "# 大神索引\n\n记录由 `zhengliu-skill` 蒸馏出的作者小 skill。\n\n| 作者 | Skill | 状态 | 安装位置 | 更新时间 |\n|---|---|---|---|---|\n";
const DEFAULT_DASHEN_INSTALL_LOG: &str = "# 安装记录\n\n记录从 `08大神蒸馏` 安装到 skill 根目录的作者小 skill。\n\n| 时间 | Skill | 来源 | 目标 | 操作 |\n|---|---|---|---|---|\n";

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
    for name in DEFAULT_KNOWLEDGE_CATEGORIES {
        let dir = root.join(name);
        fs::create_dir_all(&dir).map_err(|error| format!("知识库分类目录创建失败：{error}"))?;
    }
    write_if_missing(
        &root.join("00知识库治理").join("治理说明.md"),
        DEFAULT_KNOWLEDGE_GOVERNANCE,
    )?;
    write_if_missing(
        &root.join("00知识库治理").join("调用记录台账.md"),
        DEFAULT_KNOWLEDGE_CALL_LOG,
    )?;
    write_if_missing(
        &root.join("08大神蒸馏").join("大神索引.md"),
        DEFAULT_DASHEN_INDEX,
    )?;
    write_if_missing(
        &root.join("08大神蒸馏").join("_安装记录.md"),
        DEFAULT_DASHEN_INSTALL_LOG,
    )?;
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
        assert!(root.join("00知识库治理").join("治理说明.md").is_file());
        assert!(root.join("00知识库治理").join("调用记录台账.md").is_file());
        assert!(root.join("08大神蒸馏").join("大神索引.md").is_file());
        assert!(root.join("08大神蒸馏").join("_安装记录.md").is_file());
        assert!(!root.join("知识库使用说明.md").exists());

        let _ = fs::remove_dir_all(&root);
    }
}
