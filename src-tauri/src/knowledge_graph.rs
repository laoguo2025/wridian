use crate::runtime::{ensure_workspace, wridian_data_dir};
use crate::workspace::read_active_knowledge_root;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphResponse {
    nodes: Vec<KnowledgeGraphNode>,
    edges: Vec<KnowledgeGraphEdge>,
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
    let Some(root) = read_active_knowledge_root(&data_dir)? else {
        return Ok(KnowledgeGraphResponse {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    };
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Ok(KnowledgeGraphResponse {
            nodes: Vec::new(),
            edges: Vec::new(),
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
    let mut card_by_stem = HashMap::new();
    let mut card_paths = Vec::new();

    collect_graph_nodes(
        &root,
        &root,
        &mut nodes,
        &mut edges,
        &mut card_by_stem,
        &mut card_paths,
    )?;
    collect_wikilink_edges(&card_paths, &card_by_stem, &mut edges)?;
    dedupe_edges(&mut edges);

    Ok(KnowledgeGraphResponse { nodes, edges })
}

fn collect_graph_nodes(
    root: &Path,
    current: &Path,
    nodes: &mut Vec<KnowledgeGraphNode>,
    edges: &mut Vec<KnowledgeGraphEdge>,
    card_by_stem: &mut HashMap<String, String>,
    card_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("知识图谱目录读取失败：{error}"))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            let relative = relative_path(root, &path);
            let folder_id = format!("folder:{relative}");
            nodes.push(KnowledgeGraphNode {
                id: folder_id.clone(),
                label: name,
                kind: "folder".to_string(),
                path: Some(path.to_string_lossy().into_owned()),
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
            collect_graph_nodes(root, &path, nodes, edges, card_by_stem, card_paths)?;
        } else if is_markdown(&path) {
            let relative = relative_path(root, &path);
            let id = format!("card:{relative}");
            let title = name
                .trim_end_matches(".markdown")
                .trim_end_matches(".md")
                .to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            nodes.push(KnowledgeGraphNode {
                id: id.clone(),
                label: title.clone(),
                kind: "card".to_string(),
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
) -> Result<(), String> {
    for path in card_paths {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("知识卡读取失败（{}）：{error}", path.to_string_lossy()))?;
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
    Ok(())
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
        assert!(graph.nodes.iter().any(|node| node.id == "card:人物/阿宁.md"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.source == "card:人物/阿宁.md" && edge.target == "card:城市.md"));
    }
}
