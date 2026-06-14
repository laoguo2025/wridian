use crate::atomic_write::atomic_write_text;
use crate::path_safety::safe_child_path;
use crate::runtime::{ensure_workspace, wridian_data_dir};
use crate::workspace::{resolved_knowledge_root, works_root};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BridgeRelationInput {
    action: String,
    target_library: String,
    target_path: String,
    source_library: String,
    source_relative_path: String,
    source_title: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BridgeRelationResponse {
    ok: bool,
    target_path: String,
    field: String,
    value: String,
    inserted: bool,
    message: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Library {
    Works,
    Knowledge,
    CreativeMemory,
}

#[derive(Debug)]
struct RelationSpec {
    field: &'static str,
    value_prefix: &'static str,
}

#[tauri::command]
pub(crate) fn wridian_apply_bridge_relation(
    input: BridgeRelationInput,
) -> Result<BridgeRelationResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let target_library = parse_library(&input.target_library)?;
    let source_library = parse_library(&input.source_library)?;
    let spec = relation_spec(&input.action, target_library, source_library)?;
    let root = root_for_target(&data_dir, target_library)?;
    let target_path = resolve_target_markdown(&root, &input.target_path)?;
    let source_relative_path = normalize_source_relative_path(&input.source_relative_path)?;
    let value = relation_value(spec.value_prefix, &source_relative_path);

    let content = fs::read_to_string(&target_path).map_err(|error| {
        format!(
            "桥接关系读取目标文件失败（{}）：{error}",
            target_path.to_string_lossy()
        )
    })?;
    let update = apply_frontmatter_relation(&content, spec.field, &value);
    if update.inserted {
        atomic_write_text(&target_path, &update.content).map_err(|error| {
            format!(
                "桥接关系写入目标文件失败（{}）：{error}",
                target_path.to_string_lossy()
            )
        })?;
    }

    let source_label = input
        .source_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&source_relative_path);
    let message = if update.inserted {
        format!("已写入桥接关系：{} -> {}", spec.field, source_label)
    } else {
        format!("桥接关系已存在：{} -> {}", spec.field, source_label)
    };

    Ok(BridgeRelationResponse {
        ok: true,
        target_path: target_path.to_string_lossy().into_owned(),
        field: spec.field.to_string(),
        value,
        inserted: update.inserted,
        message,
        warnings: update.warnings,
    })
}

fn parse_library(value: &str) -> Result<Library, String> {
    match value.trim() {
        "works" => Ok(Library::Works),
        "knowledge" => Ok(Library::Knowledge),
        "creative_memory" | "creativeMemory" => Ok(Library::CreativeMemory),
        _ => Err("桥接关系只支持作品库、知识库和创作记忆来源。".to_string()),
    }
}

fn relation_spec(
    action: &str,
    target_library: Library,
    source_library: Library,
) -> Result<RelationSpec, String> {
    match (action.trim(), target_library, source_library) {
        ("referencesKnowledge", Library::Works, Library::Knowledge) => Ok(RelationSpec {
            field: "references_knowledge",
            value_prefix: "knowledge",
        }),
        ("adoptsKnowledge", Library::Works, Library::Knowledge) => Ok(RelationSpec {
            field: "adopts_knowledge",
            value_prefix: "knowledge",
        }),
        ("derivedFromKnowledge", Library::Works, Library::Knowledge) => Ok(RelationSpec {
            field: "derived_from_knowledge",
            value_prefix: "knowledge",
        }),
        ("abstractedFromDraft", Library::Knowledge, Library::Works) => Ok(RelationSpec {
            field: "abstracted_from_draft",
            value_prefix: "draft",
        }),
        ("excerptedFromProject", Library::Knowledge, Library::Works) => Ok(RelationSpec {
            field: "excerpted_from_project",
            value_prefix: "project",
        }),
        ("distilledFromMemory", Library::Knowledge, Library::CreativeMemory) => Ok(RelationSpec {
            field: "distilled_from_memory",
            value_prefix: "creative_memory",
        }),
        _ => Err("该桥接动作不符合作品域和知识域的跨域关系规则。".to_string()),
    }
}

fn root_for_target(data_dir: &Path, library: Library) -> Result<PathBuf, String> {
    match library {
        Library::Works => works_root(data_dir),
        Library::Knowledge => resolved_knowledge_root(data_dir),
        Library::CreativeMemory => Err("创作记忆不能作为桥接写入目标。".to_string()),
    }
}

fn resolve_target_markdown(root: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path.trim());
    if !path.is_file() || !is_markdown(&path) {
        return Err("桥接关系只能写入作品库或知识库内的 Markdown 文件。".to_string());
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("桥接目标根目录解析失败：{error}"))?;
    safe_child_path(&root, &path, "桥接目标")?
        .ok_or_else(|| "桥接目标不在对应库根目录内，或目标是符号链接。".to_string())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn normalize_source_relative_path(value: &str) -> Result<String, String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("桥接来源必须是库内相对路径。".to_string());
    }
    Ok(normalized)
}

fn relation_value(prefix: &str, source_relative_path: &str) -> String {
    if prefix == "project" {
        let project = source_relative_path
            .split('/')
            .next()
            .and_then(|value| Path::new(value).file_stem())
            .and_then(|value| value.to_str())
            .unwrap_or(source_relative_path);
        return format!("project:{project}");
    }
    format!("{prefix}:{source_relative_path}")
}

#[derive(Debug)]
struct FrontmatterUpdate {
    content: String,
    inserted: bool,
    warnings: Vec<String>,
}

fn apply_frontmatter_relation(content: &str, field: &str, value: &str) -> FrontmatterUpdate {
    let (frontmatter, body) = split_frontmatter(content);
    let escaped = yaml_quote(value);
    let mut warnings = Vec::new();

    if let Some(frontmatter) = frontmatter {
        let field_update = upsert_frontmatter_field(frontmatter, field, value, &escaped);
        warnings.extend(field_update.warnings);
        if !field_update.inserted {
            return FrontmatterUpdate {
                content: content.to_string(),
                inserted: false,
                warnings,
            };
        }
        return FrontmatterUpdate {
            content: format!("---\n{}---\n{}", field_update.frontmatter, body),
            inserted: true,
            warnings,
        };
    }

    FrontmatterUpdate {
        content: format!("---\n{field}:\n  - {escaped}\n---\n\n{content}"),
        inserted: true,
        warnings,
    }
}

#[derive(Debug)]
struct FieldUpdate {
    frontmatter: String,
    inserted: bool,
    warnings: Vec<String>,
}

fn upsert_frontmatter_field(
    frontmatter: &str,
    field: &str,
    value: &str,
    escaped: &str,
) -> FieldUpdate {
    let lines = frontmatter.lines().collect::<Vec<_>>();
    let Some(index) = lines
        .iter()
        .position(|line| top_level_field_name(line) == Some(field))
    else {
        let mut next = normalize_frontmatter_end(frontmatter);
        next.push_str(field);
        next.push_str(":\n  - ");
        next.push_str(escaped);
        next.push('\n');
        return FieldUpdate {
            frontmatter: next,
            inserted: true,
            warnings: Vec::new(),
        };
    };

    let (end, existing_values) = field_block(&lines, index);
    if existing_values.iter().any(|item| item == value) {
        return FieldUpdate {
            frontmatter: normalize_frontmatter_end(frontmatter),
            inserted: false,
            warnings: Vec::new(),
        };
    }

    let mut out = String::new();
    for line in &lines[..index] {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(field);
    out.push_str(":\n");
    for item in existing_values {
        out.push_str("  - ");
        out.push_str(&yaml_quote(&item));
        out.push('\n');
    }
    out.push_str("  - ");
    out.push_str(escaped);
    out.push('\n');
    for line in &lines[end..] {
        out.push_str(line);
        out.push('\n');
    }

    FieldUpdate {
        frontmatter: out,
        inserted: true,
        warnings: Vec::new(),
    }
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let Some(rest) = content.strip_prefix("---") else {
        return (None, content);
    };
    let Some(rest) = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
    else {
        return (None, content);
    };
    for marker in ["\n---", "\r\n---", "\n...", "\r\n..."] {
        if let Some(index) = rest.find(marker) {
            let after = index + marker.len();
            let body = rest[after..]
                .strip_prefix("\r\n")
                .or_else(|| rest[after..].strip_prefix('\n'))
                .unwrap_or(&rest[after..]);
            return (Some(&rest[..index]), body);
        }
    }
    (None, content)
}

fn top_level_field_name(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let (name, _) = line.split_once(':')?;
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn field_block(lines: &[&str], index: usize) -> (usize, Vec<String>) {
    let line = lines[index];
    let inline = line
        .split_once(':')
        .map(|(_, value)| parse_inline_values(value))
        .unwrap_or_default();
    let mut values = inline;
    let mut end = index + 1;
    while end < lines.len() && top_level_field_name(lines[end]).is_none() {
        if let Some(value) = parse_list_item(lines[end]) {
            values.push(value);
        }
        end += 1;
    }
    (end, dedupe_values(values))
}

fn parse_inline_values(raw: &str) -> Vec<String> {
    let value = raw.trim();
    if value.is_empty() || value == "[]" {
        return Vec::new();
    }
    if value.starts_with('[') && value.ends_with(']') {
        return value
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(clean_scalar)
            .filter(|item| !item.is_empty())
            .collect();
    }
    vec![clean_scalar(value)]
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_list_item(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let value = trimmed.strip_prefix("- ")?;
    let cleaned = clean_scalar(value);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn dedupe_values(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !deduped.iter().any(|item| item == &value) {
            deduped.push(value);
        }
    }
    deduped
}

fn normalize_frontmatter_end(frontmatter: &str) -> String {
    let mut value = frontmatter.trim_end().to_string();
    if !value.is_empty() {
        value.push('\n');
    }
    value
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-bridge-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn applies_relation_to_file_without_frontmatter() {
        let update =
            apply_frontmatter_relation("# 标题\n", "references_knowledge", "knowledge:a.md");

        assert!(update.inserted);
        assert_eq!(
            update.content,
            "---\nreferences_knowledge:\n  - \"knowledge:a.md\"\n---\n\n# 标题\n"
        );
    }

    #[test]
    fn appends_relation_to_existing_field() {
        let input = "---\ntype: draft\nreferences_knowledge:\n  - \"knowledge:a.md\"\n---\n\n正文";
        let update = apply_frontmatter_relation(input, "references_knowledge", "knowledge:b.md");

        assert!(update.inserted);
        assert!(update
            .content
            .contains("references_knowledge:\n  - \"knowledge:a.md\"\n  - \"knowledge:b.md\""));
    }

    #[test]
    fn does_not_duplicate_existing_relation() {
        let input = "---\nreferences_knowledge: [\"knowledge:a.md\"]\n---\n\n正文";
        let update = apply_frontmatter_relation(input, "references_knowledge", "knowledge:a.md");

        assert!(!update.inserted);
        assert_eq!(update.content, input);
    }

    #[test]
    fn converts_scalar_field_to_list() {
        let input = "---\nreferences_knowledge: \"knowledge:a.md\"\n---\n\n正文";
        let update = apply_frontmatter_relation(input, "references_knowledge", "knowledge:b.md");

        assert!(update.inserted);
        assert!(update
            .content
            .contains("references_knowledge:\n  - \"knowledge:a.md\"\n  - \"knowledge:b.md\""));
    }

    #[test]
    fn rejects_same_domain_bridge_action() {
        let error = relation_spec("referencesKnowledge", Library::Works, Library::Works)
            .expect_err("same domain rejected");

        assert!(error.contains("跨域关系规则"));
    }

    #[test]
    fn rejects_target_outside_root() {
        let root = temp_dir("root");
        let outside = temp_dir("outside");
        let target = outside.join("note.md");
        fs::write(&target, "").expect("write outside");

        let error = resolve_target_markdown(&root, &target.to_string_lossy())
            .expect_err("outside target rejected");

        assert!(error.contains("不在对应库根目录"));
    }
}
