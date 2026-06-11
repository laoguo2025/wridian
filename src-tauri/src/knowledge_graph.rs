use crate::path_safety::safe_child_path;
use crate::runtime::{ensure_workspace, wridian_data_dir};
use crate::workspace::resolved_knowledge_root;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_GRAPH_FILES: usize = 800;
const MAX_GRAPH_DEPTH: usize = 8;
const MAX_GRAPH_FILE_BYTES: u64 = 512 * 1024;
const MAX_GRAPH_WARNINGS: usize = 20;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphResponse {
    nodes: Vec<KnowledgeGraphNode>,
    edges: Vec<KnowledgeGraphEdge>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphNode {
    id: String,
    label: String,
    kind: String,
    path: Option<String>,
    group: String,
    size: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphEdge {
    source: String,
    target: String,
    kind: String,
}

#[tauri::command]
pub(crate) fn wridian_get_knowledge_graph() -> Result<KnowledgeGraphResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = resolved_knowledge_root(&data_dir)?;
    if !root.is_dir() {
        return Ok(KnowledgeGraphResponse {
            nodes: Vec::new(),
            edges: Vec::new(),
            warnings: Vec::new(),
        });
    }
    read_knowledge_graph(&root)
}

fn read_knowledge_graph(root: &Path) -> Result<KnowledgeGraphResponse, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("知识库目录解析失败：{error}"))?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    let mut card_by_stem = HashMap::new();
    let mut card_paths = Vec::new();

    collect_graph_nodes(
        &root,
        &root,
        0,
        &mut nodes,
        &mut edges,
        &mut card_by_stem,
        &mut card_paths,
        &mut warnings,
    )?;
    collect_wikilink_edges(&card_paths, &card_by_stem, &mut edges, &mut warnings);
    collect_frontmatter_relation_edges(&card_paths, &card_by_stem, &mut edges, &mut warnings);
    dedupe_edges(&mut edges);

    Ok(KnowledgeGraphResponse {
        nodes,
        edges,
        warnings,
    })
}

fn collect_graph_nodes(
    root: &Path,
    current: &Path,
    depth: usize,
    nodes: &mut Vec<KnowledgeGraphNode>,
    edges: &mut Vec<KnowledgeGraphEdge>,
    card_by_stem: &mut HashMap<String, String>,
    card_paths: &mut Vec<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if depth > MAX_GRAPH_DEPTH {
        push_warning(
            warnings,
            format!("知识图谱已跳过过深目录：{}", current.to_string_lossy()),
        );
        return Ok(());
    }
    if card_paths.len() >= MAX_GRAPH_FILES {
        push_warning(
            warnings,
            format!("知识图谱已达到最多 {MAX_GRAPH_FILES} 个 Markdown 文件上限。"),
        );
        return Ok(());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(current).map_err(|error| format!("知识图谱目录读取失败：{error}"))?
    {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => push_warning(warnings, format!("知识图谱目录项读取失败：{error}")),
        }
    }
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Some(safe_path) = safe_child_path(root, &path, "知识图谱")? else {
            push_warning(
                warnings,
                format!(
                    "知识图谱已跳过越界或符号链接路径：{}",
                    path.to_string_lossy()
                ),
            );
            continue;
        };
        if safe_path.is_dir() {
            let relative = relative_path(root, &safe_path);
            let folder_id = format!("folder:{relative}");
            nodes.push(KnowledgeGraphNode {
                id: folder_id.clone(),
                label: name,
                kind: "folder".to_string(),
                path: Some(safe_path.to_string_lossy().into_owned()),
                group: parent_group(&relative),
                size: 10,
            });
            if let Some(parent_id) = parent_folder_id(&relative) {
                edges.push(KnowledgeGraphEdge {
                    source: parent_id,
                    target: folder_id,
                    kind: "contains".to_string(),
                });
            }
            collect_graph_nodes(
                root,
                &safe_path,
                depth + 1,
                nodes,
                edges,
                card_by_stem,
                card_paths,
                warnings,
            )?;
        } else if is_markdown(&safe_path) {
            if card_paths.len() >= MAX_GRAPH_FILES {
                push_warning(
                    warnings,
                    format!("知识图谱已达到最多 {MAX_GRAPH_FILES} 个 Markdown 文件上限。"),
                );
                break;
            }
            if let Ok(metadata) = fs::symlink_metadata(&safe_path) {
                if metadata.len() > MAX_GRAPH_FILE_BYTES {
                    push_warning(
                        warnings,
                        format!("知识图谱已跳过过大文件：{}", safe_path.to_string_lossy()),
                    );
                    continue;
                }
            }
            let relative = relative_path(root, &path);
            let id = format!("card:{relative}");
            let title = name
                .trim_end_matches(".markdown")
                .trim_end_matches(".md")
                .to_string();
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(error) => {
                    push_warning(
                        warnings,
                        format!("知识卡读取失败（{}）：{error}", path.to_string_lossy()),
                    );
                    continue;
                }
            };
            let frontmatter = extract_frontmatter_block(&content);
            let node_kind = frontmatter
                .and_then(read_frontmatter_node_kind)
                .unwrap_or_else(|| infer_node_kind_from_path(&relative));
            nodes.push(KnowledgeGraphNode {
                id: id.clone(),
                label: title.clone(),
                kind: node_kind,
                path: Some(path.to_string_lossy().into_owned()),
                group: parent_group(&relative),
                size: 6 + extract_wikilinks(&content).len().min(10),
            });
            card_by_stem.insert(title.to_lowercase(), id.clone());
            card_by_stem.insert(relative.to_lowercase(), id.clone());
            if let Some(parent_id) = parent_folder_id(&relative) {
                edges.push(KnowledgeGraphEdge {
                    source: parent_id,
                    target: id,
                    kind: "contains".to_string(),
                });
            }
            card_paths.push(path);
        }
    }
    Ok(())
}

fn collect_wikilink_edges(
    card_paths: &[PathBuf],
    card_by_stem: &HashMap<String, String>,
    edges: &mut Vec<KnowledgeGraphEdge>,
    warnings: &mut Vec<String>,
) {
    for path in card_paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                push_warning(
                    warnings,
                    format!("知识卡读取失败（{}）：{error}", path.to_string_lossy()),
                );
                continue;
            }
        };
        let Some(source_id) = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .and_then(|stem| card_by_stem.get(&stem).cloned())
        else {
            continue;
        };
        for link in extract_wikilinks(&content) {
            if let Some(target_id) = card_by_stem.get(&link) {
                if *target_id != source_id {
                    edges.push(KnowledgeGraphEdge {
                        source: source_id.clone(),
                        target: target_id.clone(),
                        kind: "wikilink".to_string(),
                    });
                }
            }
        }
    }
}

fn collect_frontmatter_relation_edges(
    card_paths: &[PathBuf],
    card_by_stem: &HashMap<String, String>,
    edges: &mut Vec<KnowledgeGraphEdge>,
    warnings: &mut Vec<String>,
) {
    for path in card_paths {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                push_warning(
                    warnings,
                    format!("知识卡读取失败（{}）：{error}", path.to_string_lossy()),
                );
                continue;
            }
        };
        let Some(source_id) = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .and_then(|stem| card_by_stem.get(&stem).cloned())
        else {
            continue;
        };
        for (field, link) in extract_frontmatter_relation_links(&content) {
            if let Some(target_id) = card_by_stem.get(&link) {
                if *target_id != source_id {
                    edges.push(KnowledgeGraphEdge {
                        source: source_id.clone(),
                        target: target_id.clone(),
                        kind: format!("frontmatter:{field}"),
                    });
                }
            }
        }
    }
}

fn extract_frontmatter_relation_links(text: &str) -> Vec<(String, String)> {
    let Some(frontmatter) = extract_frontmatter_block(text) else {
        return Vec::new();
    };
    let mut relations = Vec::new();
    let mut current_key = String::new();
    for line in frontmatter.lines() {
        if let Some((key, value)) = frontmatter_key_value(line) {
            current_key = normalize_frontmatter_field(key);
            if is_system_frontmatter_field(&current_key) {
                current_key.clear();
                continue;
            }
            for link in extract_wikilinks(value) {
                relations.push((current_key.clone(), link));
            }
            continue;
        }
        if current_key.is_empty() || !frontmatter_list_item(line) {
            continue;
        }
        for link in extract_wikilinks(line) {
            relations.push((current_key.clone(), link));
        }
    }
    relations
}

fn extract_frontmatter_block(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    rest.split_once("\n---")
        .or_else(|| rest.split_once("\r\n---"))
        .map(|(frontmatter, _)| frontmatter)
}

fn frontmatter_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
        return None;
    }
    let (key, value) = trimmed.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value.trim()))
}

fn frontmatter_list_item(line: &str) -> bool {
    line.trim_start().starts_with('-')
}

fn normalize_frontmatter_field(field: &str) -> String {
    field.trim().replace(' ', "_").to_lowercase()
}

fn is_system_frontmatter_field(field: &str) -> bool {
    field.starts_with('_') || matches!(field, "type" | "kind" | "category" | "status" | "tags")
}

fn read_frontmatter_node_kind(frontmatter: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let (key, value) = frontmatter_key_value(line)?;
        let field = normalize_frontmatter_field(key);
        if !matches!(
            field.as_str(),
            "type" | "kind" | "card_type" | "wridian_type"
        ) {
            continue;
        }
        let normalized = normalize_node_kind_value(value);
        if !normalized.is_empty() {
            return Some(normalized);
        }
    }
    None
}

fn normalize_node_kind_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace(' ', "-")
        .to_lowercase()
}

fn infer_node_kind_from_path(relative: &str) -> String {
    let top = relative.split('/').next().unwrap_or("").trim();
    if top.starts_with("01") {
        return "source".to_string();
    }
    if top.starts_with("02") {
        return "analysis".to_string();
    }
    if top.starts_with("08") {
        return "skill".to_string();
    }
    if top.starts_with("03")
        || top.starts_with("04")
        || top.starts_with("05")
        || top.starts_with("06")
        || top.starts_with("07")
    {
        return "knowledge-card".to_string();
    }
    "card".to_string()
}

fn extract_wikilinks(text: &str) -> HashSet<String> {
    let mut links = HashSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let link = rest[..end]
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches(".md")
            .trim_end_matches(".markdown")
            .to_lowercase();
        if !link.is_empty() {
            links.insert(link);
        }
        rest = &rest[end + 2..];
    }
    links
}

fn dedupe_edges(edges: &mut Vec<KnowledgeGraphEdge>) {
    let mut seen = HashSet::new();
    edges.retain(|edge| seen.insert(format!("{}>{}>{}", edge.source, edge.target, edge.kind)));
}

fn parent_folder_id(relative: &str) -> Option<String> {
    let mut parts = relative.rsplitn(2, '/');
    let _name = parts.next()?;
    let parent = parts.next()?.trim();
    (!parent.is_empty()).then(|| format!("folder:{parent}"))
}

fn parent_group(relative: &str) -> String {
    relative
        .split('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("知识库")
        .to_string()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_GRAPH_WARNINGS {
        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_links_folders_cards_and_wikilinks() {
        let root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-test-{}",
            crate::runtime::iso_timestamp()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("人物")).expect("create folder");
        fs::write(root.join("人物").join("阿宁.md"), "关联 [[城市]]").expect("write card");
        fs::write(root.join("城市.md"), "地点").expect("write target");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| node.id == "folder:人物"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.id == "card:人物/阿宁.md"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.source == "card:人物/阿宁.md" && edge.target == "card:城市.md"));
        assert!(graph.warnings.is_empty());
    }

    #[test]
    fn graph_reads_frontmatter_relations_and_node_kinds() {
        let root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-frontmatter-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("03故事模型")).expect("create knowledge folder");
        fs::create_dir_all(root.join("08大神蒸馏")).expect("create skill folder");
        fs::write(
            root.join("03故事模型").join("技法.md"),
            "---\ntype: method\nrelated_to:\n  - \"[[来源]]\"\nadopts knowledge: \"[[作者Skill]]\"\nstatus: active\n---\n正文 [[来源]]",
        )
        .expect("write source");
        fs::write(root.join("来源.md"), "source").expect("write target");
        fs::write(root.join("08大神蒸馏").join("作者Skill.md"), "skill").expect("write skill");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph
            .nodes
            .iter()
            .any(|node| { node.id == "card:03故事模型/技法.md" && node.kind == "method" }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:08大神蒸馏/作者Skill.md" && node.kind == "skill"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:03故事模型/技法.md"
                && edge.target == "card:来源.md"
                && edge.kind == "frontmatter:related_to"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:03故事模型/技法.md"
                && edge.target == "card:08大神蒸馏/作者Skill.md"
                && edge.kind == "frontmatter:adopts_knowledge"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:03故事模型/技法.md"
                && edge.target == "card:来源.md"
                && edge.kind == "wikilink"
        }));
    }

    #[test]
    fn graph_skips_oversized_cards_with_warning() {
        let root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-large-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("small.md"), "关联 [[large]]").expect("write small");
        fs::write(
            root.join("large.md"),
            "x".repeat((MAX_GRAPH_FILE_BYTES as usize) + 1),
        )
        .expect("write large");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| node.id == "card:small.md"));
        assert!(!graph.nodes.iter().any(|node| node.id == "card:large.md"));
        assert!(graph
            .warnings
            .iter()
            .any(|warning| warning.contains("过大文件")));
    }
}
