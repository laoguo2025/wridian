use crate::path_safety::safe_child_path;
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RULE_FILE_BYTES: u64 = 128 * 1024;
const MAX_RULE_BLOCK_CHARS: usize = 5200;
const RULE_FILE_NAMES: [&str; 3] = ["WRIDIAN.md", "AGENT.md", "AGENTS.md"];
const INDEX_FILE_NAMES: [&str; 2] = ["index.md", "hot.md"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleRouteContext {
    pub(crate) block: String,
    pub(crate) item_count: usize,
    pub(crate) truncated: bool,
}

pub(crate) fn read_rule_route_context(data_dir: &Path) -> Result<RuleRouteContext, String> {
    let mut entries = Vec::new();

    if let Some(root) = read_active_work_root(data_dir)? {
        let root = PathBuf::from(root);
        if root.is_dir() {
            collect_root_rule_entries("works", "作品库", &root, &mut entries)?;
        }
    }

    let knowledge_root = resolved_knowledge_root(data_dir)?;
    if knowledge_root.is_dir() {
        collect_root_rule_entries("knowledge", "知识库", &knowledge_root, &mut entries)?;
    }

    Ok(render_rule_entries(&entries))
}

fn collect_root_rule_entries(
    library: &str,
    label: &str,
    root: &Path,
    entries: &mut Vec<RuleRouteEntry>,
) -> Result<(), String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("{label}规则目录解析失败：{error}"))?;

    for file_name in RULE_FILE_NAMES {
        push_rule_entry(
            library,
            label,
            &canonical_root,
            file_name,
            "规则路由",
            entries,
        )?;
    }
    for file_name in INDEX_FILE_NAMES {
        push_rule_entry(library, label, &canonical_root, file_name, "索引", entries)?;
    }

    Ok(())
}

fn push_rule_entry(
    library: &str,
    label: &str,
    root: &Path,
    relative: &str,
    role: &str,
    entries: &mut Vec<RuleRouteEntry>,
) -> Result<(), String> {
    let target = root.join(relative);
    if !target.exists() {
        return Ok(());
    }
    let Some(safe_target) = safe_child_path(root, &target, "规则路由")? else {
        return Ok(());
    };
    let metadata = fs::symlink_metadata(&safe_target)
        .map_err(|error| format!("{label}规则文件信息读取失败：{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.len() > MAX_RULE_FILE_BYTES {
        return Ok(());
    }
    let content = fs::read_to_string(&safe_target)
        .map_err(|error| format!("{label}规则文件读取失败：{error}"))?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(());
    }
    entries.push(RuleRouteEntry {
        library: library.to_string(),
        label: label.to_string(),
        relative_path: relative.to_string(),
        role: role.to_string(),
        content: content.to_string(),
    });
    Ok(())
}

fn render_rule_entries(entries: &[RuleRouteEntry]) -> RuleRouteContext {
    let mut truncated = false;
    let mut rendered = String::new();

    for entry in entries {
        let header = format!(
            "【{}｜{}｜{}｜{}】\n",
            entry.library, entry.label, entry.role, entry.relative_path
        );
        let remaining = MAX_RULE_BLOCK_CHARS.saturating_sub(rendered.chars().count());
        if remaining <= header.chars().count() {
            truncated = true;
            break;
        }
        rendered.push_str(&header);
        let remaining = MAX_RULE_BLOCK_CHARS.saturating_sub(rendered.chars().count());
        let content = take_chars(&entry.content, remaining);
        if content.chars().count() < entry.content.chars().count() {
            truncated = true;
        }
        rendered.push_str(&content);
        rendered.push_str("\n\n");
        if truncated {
            break;
        }
    }

    RuleRouteContext {
        block: rendered.trim().to_string(),
        item_count: entries.len(),
        truncated,
    }
}

fn take_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

#[derive(Debug)]
struct RuleRouteEntry {
    library: String,
    label: String,
    relative_path: String,
    role: String,
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::unique_test_suffix;
    use serde_json::json;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-rule-router-test-{}-{}",
            name,
            unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn rule_router_reads_work_and_knowledge_rules_without_absolute_paths() {
        let data_dir = temp_data_dir("reads-rules");
        let works = data_dir.join("works");
        let knowledge = data_dir.join("knowledge");
        fs::create_dir_all(&works).expect("create works");
        fs::create_dir_all(&knowledge).expect("create knowledge");
        fs::write(works.join("WRIDIAN.md"), "作品规则：只写当前项目。").expect("write works rules");
        fs::write(works.join("index.md"), "作品索引：第一章。").expect("write works index");
        fs::write(knowledge.join("AGENT.md"), "知识规则：先看来源。")
            .expect("write knowledge rules");
        fs::write(knowledge.join("hot.md"), "近期知识：反链。").expect("write knowledge hot");
        let config_path = crate::runtime::workspace_config_path(&data_dir);
        fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create runtime root");
        fs::write(
            config_path,
            serde_json::to_string(&json!({
                "activeWorkRoot": works.to_string_lossy(),
                "knowledgeRoot": knowledge.to_string_lossy()
            }))
            .expect("serialize config"),
        )
        .expect("write config");

        let context = read_rule_route_context(&data_dir).expect("read context");

        assert!(context
            .block
            .contains("【works｜作品库｜规则路由｜WRIDIAN.md】"));
        assert!(context.block.contains("作品规则：只写当前项目。"));
        assert!(context.block.contains("【works｜作品库｜索引｜index.md】"));
        assert!(context
            .block
            .contains("【knowledge｜知识库｜规则路由｜AGENT.md】"));
        assert!(context.block.contains("近期知识：反链。"));
        assert!(!context
            .block
            .contains(&data_dir.to_string_lossy().to_string()));
        assert_eq!(context.item_count, 4);
        assert!(!context.truncated);
    }
}
