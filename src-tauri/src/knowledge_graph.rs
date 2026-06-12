use crate::metadata_index::{read_library_metadata_index, MetadataFile, MetadataLibraryIndex};
use crate::runtime::{ensure_workspace, wridian_data_dir};
use crate::workspace::resolved_knowledge_root;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

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
    relative_path: Option<String>,
    group: String,
    size: usize,
    aliases: Vec<String>,
    tags: Vec<String>,
    source_refs: Vec<String>,
    outgoing_count: usize,
    backlink_count: usize,
    unresolved_count: usize,
    backlink_sources: Vec<String>,
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
    let index = read_library_metadata_index("knowledge", root)?;
    Ok(graph_from_metadata_index(&index))
}

fn graph_from_metadata_index(index: &MetadataLibraryIndex) -> KnowledgeGraphResponse {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_nodes = HashSet::new();
    let generated_paths = generated_relative_paths(index);
    let unresolved_nodes = unresolved_graph_nodes(index, &generated_paths);

    for folder in folder_paths(index, &generated_paths) {
        let id = format!("folder:{folder}");
        if seen_nodes.insert(id.clone()) {
            nodes.push(KnowledgeGraphNode {
                id: id.clone(),
                label: folder
                    .rsplit('/')
                    .next()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("知识库")
                    .to_string(),
                kind: "folder".to_string(),
                path: index.root_path.as_deref().map(|root| {
                    PathBuf::from(root)
                        .join(&folder)
                        .to_string_lossy()
                        .into_owned()
                }),
                relative_path: Some(folder.clone()),
                group: parent_group(&folder),
                size: 10,
                aliases: Vec::new(),
                tags: Vec::new(),
                source_refs: Vec::new(),
                outgoing_count: 0,
                backlink_count: 0,
                unresolved_count: 0,
                backlink_sources: Vec::new(),
            });
        }
        if let Some(parent_id) = parent_folder_id(&folder) {
            edges.push(KnowledgeGraphEdge {
                source: parent_id,
                target: id,
                kind: "contains".to_string(),
            });
        }
    }

    for file in &index.files {
        if generated_paths.contains(&file.relative_path) {
            continue;
        }
        let id = graph_card_id(&file.relative_path);
        if seen_nodes.insert(id.clone()) {
            nodes.push(KnowledgeGraphNode {
                id: id.clone(),
                label: file.title.clone(),
                kind: node_kind(file),
                path: Some(file.path.clone()),
                relative_path: Some(file.relative_path.clone()),
                group: parent_group(&file.relative_path),
                size: 6 + file.outgoing_links.len().min(10) + file.backlinks.len().min(8),
                aliases: file.aliases.clone(),
                tags: file.tags.clone(),
                source_refs: relation_values(
                    &file.frontmatter,
                    &[
                        "source_refs",
                        "source_ref",
                        "sources",
                        "source",
                        "origin",
                        "origins",
                    ],
                ),
                outgoing_count: file
                    .outgoing_links
                    .iter()
                    .filter(|link| link.resolved)
                    .count(),
                backlink_count: file.backlinks.len(),
                unresolved_count: file
                    .outgoing_links
                    .iter()
                    .filter(|link| !link.resolved)
                    .count(),
                backlink_sources: backlink_sources(file),
            });
        }
        if let Some(parent_id) = parent_folder_id(&file.relative_path) {
            edges.push(KnowledgeGraphEdge {
                source: parent_id,
                target: id,
                kind: "contains".to_string(),
            });
        }
    }

    for link in &index.links {
        if generated_paths.contains(&link.source_relative_path) {
            continue;
        }
        let Some(target_relative_path) = link.target_relative_path.as_deref() else {
            continue;
        };
        if generated_paths.contains(target_relative_path) {
            continue;
        }
        if link.source_relative_path == target_relative_path {
            continue;
        }
        edges.push(KnowledgeGraphEdge {
            source: graph_card_id(&link.source_relative_path),
            target: graph_card_id(target_relative_path),
            kind: link
                .frontmatter_field
                .as_deref()
                .map(|field| format!("frontmatter:{field}"))
                .unwrap_or_else(|| {
                    if link.embed {
                        "embed".to_string()
                    } else {
                        "wikilink".to_string()
                    }
                }),
        });
    }

    for unresolved in unresolved_nodes.values() {
        let id = unresolved_node_id(&unresolved.normalized_target);
        if seen_nodes.insert(id.clone()) {
            nodes.push(KnowledgeGraphNode {
                id,
                label: unresolved.label.clone(),
                kind: "unresolved".to_string(),
                path: None,
                relative_path: None,
                group: "未解析链接".to_string(),
                size: 5 + unresolved.count.min(12),
                aliases: Vec::new(),
                tags: Vec::new(),
                source_refs: Vec::new(),
                outgoing_count: 0,
                backlink_count: unresolved.count,
                unresolved_count: unresolved.count,
                backlink_sources: unresolved.sources.iter().take(8).cloned().collect(),
            });
        }
    }

    for link in &index.unresolved_links {
        if generated_paths.contains(&link.source_relative_path) {
            continue;
        }
        edges.push(KnowledgeGraphEdge {
            source: graph_card_id(&link.source_relative_path),
            target: unresolved_node_id(&link.normalized_target),
            kind: link
                .frontmatter_field
                .as_deref()
                .map(|field| format!("frontmatter:{field}:unresolved"))
                .unwrap_or_else(|| {
                    if link.embed {
                        "embed:unresolved".to_string()
                    } else {
                        "unresolved".to_string()
                    }
                }),
        });
    }

    dedupe_edges(&mut edges);
    push_metadata_warnings(index, &generated_paths, &mut warnings);

    KnowledgeGraphResponse {
        nodes,
        edges,
        warnings,
    }
}

fn folder_paths(index: &MetadataLibraryIndex, generated_paths: &HashSet<String>) -> Vec<String> {
    let mut folders = HashSet::new();
    for file in &index.files {
        if generated_paths.contains(&file.relative_path) {
            continue;
        }
        let mut current = file.relative_path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            if parent.is_empty() {
                break;
            }
            folders.insert(parent.to_string());
            current = parent;
        }
    }
    let mut ordered = folders.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.matches('/')
            .count()
            .cmp(&right.matches('/').count())
            .then_with(|| left.cmp(right))
    });
    ordered
}

#[derive(Default)]
struct UnresolvedGraphNode {
    label: String,
    normalized_target: String,
    count: usize,
    sources: Vec<String>,
}

fn unresolved_graph_nodes(
    index: &MetadataLibraryIndex,
    generated_paths: &HashSet<String>,
) -> BTreeMap<String, UnresolvedGraphNode> {
    let mut nodes = BTreeMap::<String, UnresolvedGraphNode>::new();
    for link in &index.unresolved_links {
        if generated_paths.contains(&link.source_relative_path) {
            continue;
        }
        let key = if link.normalized_target.trim().is_empty() {
            link.raw_target.to_lowercase()
        } else {
            link.normalized_target.clone()
        };
        let node = nodes
            .entry(key.clone())
            .or_insert_with(|| UnresolvedGraphNode {
                label: link.raw_target.clone(),
                normalized_target: key,
                count: 0,
                sources: Vec::new(),
            });
        node.count += 1;
        if !node.sources.contains(&link.source_relative_path) {
            node.sources.push(link.source_relative_path.clone());
        }
    }
    nodes
}

fn node_kind(file: &MetadataFile) -> String {
    for field in ["type", "kind", "card_type", "wridian_type"] {
        if let Some(values) = file.frontmatter.get(field) {
            if let Some(value) = values.iter().find(|value| !value.trim().is_empty()) {
                return normalize_node_kind_value(value);
            }
        }
    }
    infer_node_kind_from_path(&file.relative_path)
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

fn relation_values(frontmatter: &BTreeMap<String, Vec<String>>, fields: &[&str]) -> Vec<String> {
    let mut values = fields
        .iter()
        .flat_map(|field| frontmatter.get(*field).into_iter().flatten())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn generated_relative_paths(index: &MetadataLibraryIndex) -> HashSet<String> {
    index
        .files
        .iter()
        .filter(|file| is_generated_knowledge_file(file))
        .map(|file| file.relative_path.clone())
        .collect()
}

fn is_generated_knowledge_file(file: &MetadataFile) -> bool {
    has_frontmatter_value(file, "wridian_generated", &["true", "yes", "1"])
        || has_frontmatter_value(
            file,
            "wridian_type",
            &["knowledge_hot_cache", "knowledge_fold"],
        )
}

fn has_frontmatter_value(file: &MetadataFile, field: &str, expected: &[&str]) -> bool {
    file.frontmatter.get(field).is_some_and(|values| {
        values.iter().any(|value| {
            let normalized = value.trim().to_lowercase();
            expected
                .iter()
                .any(|expected| normalized == *expected || normalized.contains(expected))
        })
    })
}

fn backlink_sources(file: &MetadataFile) -> Vec<String> {
    let mut sources = file
        .backlinks
        .iter()
        .map(|backlink| backlink.source_relative_path.clone())
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    sources.truncate(8);
    sources
}

fn push_metadata_warnings(
    index: &MetadataLibraryIndex,
    generated_paths: &HashSet<String>,
    warnings: &mut Vec<String>,
) {
    let unresolved_count = index
        .unresolved_links
        .iter()
        .filter(|link| !generated_paths.contains(&link.source_relative_path))
        .count();
    if unresolved_count > 0 {
        push_warning(
            warnings,
            format!(
                "知识图谱发现 {} 条未解析链接，可在知识库体检中修复。",
                unresolved_count
            ),
        );
    }
    for link in index
        .unresolved_links
        .iter()
        .filter(|link| !generated_paths.contains(&link.source_relative_path))
        .take(MAX_GRAPH_WARNINGS.saturating_sub(warnings.len()))
    {
        push_warning(
            warnings,
            format!(
                "未解析链接：{} -> [[{}]]（{}）",
                link.source_relative_path, link.raw_target, link.reason
            ),
        );
    }
}

fn graph_card_id(relative_path: &str) -> String {
    format!("card:{relative_path}")
}

fn unresolved_node_id(normalized_target: &str) -> String {
    format!("unresolved:{normalized_target}")
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

fn dedupe_edges(edges: &mut Vec<KnowledgeGraphEdge>) {
    let mut seen = HashSet::new();
    edges.retain(|edge| seen.insert(format!("{}>{}>{}", edge.source, edge.target, edge.kind)));
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_GRAPH_WARNINGS {
        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_index::read_metadata_index_for_roots;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn graph_links_folders_cards_aliases_and_wikilinks() {
        let root = temp_dir("links");
        fs::create_dir_all(root.join("人物")).expect("create folder");
        fs::write(root.join("人物").join("阿宁.md"), "关联 [[城]]").expect("write card");
        fs::write(
            root.join("城市.md"),
            "---\naliases: [城]\ntags: [place, setting]\nsource_refs: [\"[[设定集]]\"]\n---\n地点",
        )
        .expect("write target");
        fs::write(
            root.join("设定集.md"),
            "---\ntype: knowledge_source\n---\n来源",
        )
        .expect("write source ref");

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
        let city = graph
            .nodes
            .iter()
            .find(|node| node.id == "card:城市.md")
            .expect("city node");
        assert_eq!(city.aliases, vec!["城"]);
        assert_eq!(city.tags, vec!["place", "setting"]);
        assert_eq!(city.source_refs, vec!["[[设定集]]"]);
        assert_eq!(city.backlink_count, 1);
        assert_eq!(city.backlink_sources, vec!["人物/阿宁.md"]);
        assert!(graph.warnings.is_empty());
    }

    #[test]
    fn graph_reads_frontmatter_relations_and_node_kinds_from_metadata_index() {
        let root = temp_dir("frontmatter");
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
    fn graph_surfaces_unresolved_links_from_metadata_index() {
        let root = temp_dir("unresolved");
        fs::write(root.join("来源.md"), "缺失 [[不存在]]").expect("write source");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| node.id == "card:来源.md"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.id == "unresolved:不存在" && node.kind == "unresolved"));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:来源.md"
                && edge.target == "unresolved:不存在"
                && edge.kind == "unresolved"
        }));
        assert!(graph
            .warnings
            .iter()
            .any(|warning| warning.contains("未解析链接")));
    }

    #[test]
    fn graph_excludes_generated_hot_and_fold_files() {
        let root = temp_dir("generated");
        fs::create_dir_all(root.join("00知识库治理").join("folds")).expect("create folds");
        fs::write(root.join("方法.md"), "方法").expect("write card");
        fs::write(
            root.join("hot.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_hot_cache\n---\n[[不存在]]",
        )
        .expect("write hot");
        fs::write(
            root.join("00知识库治理")
                .join("folds")
                .join("knowledge-fold.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_fold\n---\n[[方法]]",
        )
        .expect("write fold");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| node.id == "card:方法.md"));
        assert!(!graph.nodes.iter().any(|node| node.id == "card:hot.md"));
        assert!(!graph
            .nodes
            .iter()
            .any(|node| node.id == "card:00知识库治理/folds/knowledge-fold.md"));
        assert!(!graph.nodes.iter().any(|node| node.kind == "unresolved"));
        assert!(graph.warnings.is_empty());
    }

    #[test]
    fn graph_keeps_response_shape_when_built_from_index() {
        let root = temp_dir("shape");
        fs::write(root.join("A.md"), "[[B]]").expect("write A");
        fs::write(root.join("B.md"), "B").expect("write B");
        let index = read_metadata_index_for_roots(vec![("knowledge", root)]).expect("index");
        let knowledge = index.libraries.first().expect("knowledge index");

        let graph = graph_from_metadata_index(knowledge);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}
