use crate::runtime::{ensure_workspace, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
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
    relationships: Vec<KnowledgeGraphRelation>,
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
    title_key: String,
    updated_at: Option<String>,
    type_icon: Option<String>,
    type_color: Option<String>,
    type_sort: Option<String>,
    default_fields: Vec<String>,
    has_source: bool,
    adopted_but_not_distilled: bool,
    inbound_count: usize,
    outbound_count: usize,
    referenced_by: Vec<String>,
    used_by_works: Vec<String>,
    duplicate_title: bool,
    duplicate_concept: bool,
    stale_high_reference: bool,
    review_status: Option<String>,
    has_conflict: bool,
    has_uncertainty: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphEdge {
    source: String,
    target: String,
    kind: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeGraphRelation {
    field_name: String,
    source_file: String,
    target_file: String,
    relation_type: String,
    bidirectional: bool,
}

#[derive(Debug, Clone)]
struct KnowledgeCardMeta {
    kind: String,
    concept_keys: Vec<String>,
}

#[derive(Debug, Clone)]
struct KnowledgeTypeDefinition {
    icon: Option<String>,
    color: Option<String>,
    sort: Option<String>,
    default_fields: Vec<String>,
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
            relationships: Vec::new(),
            warnings: Vec::new(),
        });
    }
    let work_root = read_active_work_root(&data_dir)?.map(PathBuf::from);
    read_knowledge_graph_for_roots(&root, work_root.as_deref())
}

#[cfg(test)]
fn read_knowledge_graph(root: &Path) -> Result<KnowledgeGraphResponse, String> {
    read_knowledge_graph_for_roots(root, None)
}

fn read_knowledge_graph_for_roots(
    root: &Path,
    work_root: Option<&Path>,
) -> Result<KnowledgeGraphResponse, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("知识库目录解析失败：{error}"))?;
    let work_root = work_root
        .filter(|path| path.is_dir())
        .and_then(|path| path.canonicalize().ok());
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut warnings = Vec::new();
    let mut card_by_stem = HashMap::new();
    let mut card_paths = Vec::new();
    let mut card_meta = HashMap::new();
    let mut type_definitions = HashMap::new();

    collect_graph_nodes(
        &root,
        &root,
        0,
        &mut nodes,
        &mut edges,
        &mut card_by_stem,
        &mut card_paths,
        &mut card_meta,
        &mut type_definitions,
        &mut warnings,
    )?;
    let mut relationships = Vec::new();
    collect_wikilink_edges(&root, &card_paths, &card_by_stem, &mut edges, &mut warnings);
    collect_frontmatter_relation_edges(
        &root,
        &card_paths,
        &card_by_stem,
        &mut edges,
        &mut relationships,
        &mut warnings,
    );
    dedupe_edges(&mut edges);
    dedupe_relationships(&mut relationships);
    mark_bidirectional_relationships(&mut relationships);
    let work_references = work_root
        .as_deref()
        .map(|root| collect_work_references(root, &card_by_stem, &mut warnings))
        .unwrap_or_default();
    enrich_nodes_with_graph_metadata(
        &mut nodes,
        &edges,
        &card_meta,
        &type_definitions,
        &work_references,
    );

    Ok(KnowledgeGraphResponse {
        nodes,
        edges,
        relationships,
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
    card_meta: &mut HashMap<String, KnowledgeCardMeta>,
    type_definitions: &mut HashMap<String, KnowledgeTypeDefinition>,
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
                title_key: relative.to_lowercase(),
                updated_at: None,
                type_icon: None,
                type_color: None,
                type_sort: None,
                default_fields: Vec::new(),
                has_source: false,
                adopted_but_not_distilled: false,
                inbound_count: 0,
                outbound_count: 0,
                duplicate_title: false,
                stale_high_reference: false,
                referenced_by: Vec::new(),
                used_by_works: Vec::new(),
                duplicate_concept: false,
                review_status: None,
                has_conflict: false,
                has_uncertainty: false,
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
                &path,
                depth + 1,
                nodes,
                edges,
                card_by_stem,
                card_paths,
                card_meta,
                type_definitions,
                warnings,
            )?;
        } else if is_markdown(&path) {
            if card_paths.len() >= MAX_GRAPH_FILES {
                push_warning(
                    warnings,
                    format!("知识图谱已达到最多 {MAX_GRAPH_FILES} 个 Markdown 文件上限。"),
                );
                break;
            }
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                if metadata.file_type().is_symlink() {
                    push_warning(
                        warnings,
                        format!("知识图谱已跳过符号链接：{}", path.to_string_lossy()),
                    );
                    continue;
                }
                if metadata.len() > MAX_GRAPH_FILE_BYTES {
                    push_warning(
                        warnings,
                        format!("知识图谱已跳过过大文件：{}", path.to_string_lossy()),
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
            let fields = frontmatter
                .map(parse_frontmatter_fields)
                .unwrap_or_default();
            let node_kind = read_frontmatter_node_kind_from_fields(&fields)
                .unwrap_or_else(|| infer_node_kind_from_path(&relative));
            if is_type_definition_fields(&fields) {
                let type_name =
                    frontmatter_field_first(&fields, "title").unwrap_or_else(|| title.clone());
                type_definitions.insert(
                    normalize_type_name(&type_name),
                    read_type_definition_from_fields(&fields),
                );
            }
            let title_key = title.to_lowercase();
            let concept_keys = frontmatter_concept_keys(&fields, &title);
            nodes.push(KnowledgeGraphNode {
                id: id.clone(),
                label: title.clone(),
                kind: node_kind,
                path: Some(path.to_string_lossy().into_owned()),
                group: parent_group(&relative),
                size: 6 + extract_wikilinks(&content).len().min(10),
                title_key: title_key.clone(),
                updated_at: frontmatter_field_first(&fields, "updated_at"),
                type_icon: None,
                type_color: None,
                type_sort: None,
                default_fields: Vec::new(),
                has_source: frontmatter_has_source(&fields),
                adopted_but_not_distilled: frontmatter_has_nonempty_field(
                    &fields,
                    "adopts_knowledge",
                ) && !frontmatter_has_nonempty_field(
                    &fields,
                    "derived_from_knowledge",
                ),
                inbound_count: 0,
                outbound_count: 0,
                referenced_by: Vec::new(),
                used_by_works: Vec::new(),
                duplicate_title: false,
                duplicate_concept: false,
                stale_high_reference: false,
                review_status: frontmatter_review_status(&fields),
                has_conflict: frontmatter_has_any_field(
                    &fields,
                    &["conflicts_with", "冲突对象", "冲突卡片"],
                ) || content.contains("[!contradiction]"),
                has_uncertainty: frontmatter_has_any_field(
                    &fields,
                    &["uncertainty", "不确定性", "待核查", "open_questions"],
                ) || content.contains("[!gap]"),
            });
            card_by_stem.insert(title_key.clone(), id.clone());
            card_by_stem.insert(file_stem_key(&path), id.clone());
            card_by_stem.insert(relative.to_lowercase(), id.clone());
            card_by_stem.insert(
                strip_markdown_extension(&relative).to_lowercase(),
                id.clone(),
            );
            card_meta.insert(
                id.clone(),
                KnowledgeCardMeta {
                    kind: nodes
                        .last()
                        .map(|node| node.kind.clone())
                        .unwrap_or_else(|| "card".to_string()),
                    concept_keys,
                },
            );
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
    root: &Path,
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
        let Some(source_id) = card_id_for_path(root, path) else {
            continue;
        };
        for link in extract_wikilinks(&content) {
            if let Some(target_id) = resolve_wikilink_target(&link, card_by_stem) {
                if *target_id != source_id {
                    edges.push(KnowledgeGraphEdge {
                        source: source_id.clone(),
                        target: (*target_id).clone(),
                        kind: "wikilink".to_string(),
                    });
                }
            }
        }
    }
}

fn collect_frontmatter_relation_edges(
    root: &Path,
    card_paths: &[PathBuf],
    card_by_stem: &HashMap<String, String>,
    edges: &mut Vec<KnowledgeGraphEdge>,
    relationships: &mut Vec<KnowledgeGraphRelation>,
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
        let Some(source_id) = card_id_for_path(root, path) else {
            continue;
        };
        for (field, link) in extract_frontmatter_relation_links(&content) {
            if let Some(target_id) = resolve_wikilink_target(&link, card_by_stem) {
                if *target_id != source_id {
                    edges.push(KnowledgeGraphEdge {
                        source: source_id.clone(),
                        target: (*target_id).clone(),
                        kind: format!("frontmatter:{field}"),
                    });
                    relationships.push(KnowledgeGraphRelation {
                        field_name: field.clone(),
                        source_file: source_id.trim_start_matches("card:").to_string(),
                        target_file: target_id.trim_start_matches("card:").to_string(),
                        relation_type: relation_type_for_field(&field),
                        bidirectional: false,
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

fn parse_frontmatter_fields(frontmatter: &str) -> HashMap<String, Vec<String>> {
    let mut fields: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_key = String::new();
    for line in frontmatter.lines() {
        if let Some((key, value)) = frontmatter_key_value(line) {
            current_key = normalize_frontmatter_field(key);
            let values = parse_frontmatter_value_tokens(value);
            if !values.is_empty() {
                fields
                    .entry(current_key.clone())
                    .or_default()
                    .extend(values);
            } else {
                fields.entry(current_key.clone()).or_default();
            }
            continue;
        }
        if current_key.is_empty() || !frontmatter_list_item(line) {
            continue;
        }
        let value = line
            .trim_start()
            .trim_start_matches('-')
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if !value.is_empty() {
            fields.entry(current_key.clone()).or_default().push(value);
        }
    }
    fields
}

fn parse_frontmatter_value_tokens(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return trimmed
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(clean_frontmatter_scalar)
            .filter(|value| !value.is_empty())
            .collect();
    }
    vec![clean_frontmatter_scalar(trimmed)]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

fn clean_frontmatter_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn frontmatter_field_first(fields: &HashMap<String, Vec<String>>, field: &str) -> Option<String> {
    fields
        .get(field)
        .and_then(|values| values.iter().find(|value| !value.trim().is_empty()))
        .cloned()
}

fn frontmatter_has_nonempty_field(fields: &HashMap<String, Vec<String>>, field: &str) -> bool {
    fields
        .get(field)
        .map(|values| values.is_empty() || values.iter().any(|value| !value.trim().is_empty()))
        .unwrap_or(false)
}

fn frontmatter_has_any_field(fields: &HashMap<String, Vec<String>>, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|field| frontmatter_has_nonempty_field(fields, &normalize_frontmatter_field(field)))
}

fn frontmatter_review_status(fields: &HashMap<String, Vec<String>>) -> Option<String> {
    [
        "review_status",
        "governance_status",
        "体检状态",
        "治理状态",
        "核查状态",
    ]
    .iter()
    .find_map(|field| frontmatter_field_first(fields, &normalize_frontmatter_field(field)))
}

fn frontmatter_has_source(fields: &HashMap<String, Vec<String>>) -> bool {
    [
        "source",
        "derived_from",
        "quotes",
        "evidence",
        "source_refs",
        "source_ref",
        "source_url",
        "source_title",
        "excerpted_from_project",
        "abstracted_from_draft",
        "distilled_from_memory",
    ]
    .iter()
    .any(|field| frontmatter_has_nonempty_field(fields, field))
}

fn read_frontmatter_node_kind_from_fields(fields: &HashMap<String, Vec<String>>) -> Option<String> {
    for field in ["type", "kind", "card_type", "wridian_type"] {
        let Some(value) = frontmatter_field_first(fields, field) else {
            continue;
        };
        let normalized = normalize_node_kind_value(&value);
        if !normalized.is_empty() {
            return Some(if normalized == "type" {
                "type-definition".to_string()
            } else {
                canonical_node_kind(&normalized)
            });
        }
    }
    None
}

fn is_type_definition_fields(fields: &HashMap<String, Vec<String>>) -> bool {
    frontmatter_field_first(fields, "type")
        .map(|value| normalize_node_kind_value(&value) == "type")
        .unwrap_or(false)
}

fn read_type_definition_from_fields(
    fields: &HashMap<String, Vec<String>>,
) -> KnowledgeTypeDefinition {
    KnowledgeTypeDefinition {
        icon: frontmatter_field_first(fields, "icon"),
        color: frontmatter_field_first(fields, "color").filter(|value| is_safe_color(value)),
        sort: frontmatter_field_first(fields, "sort"),
        default_fields: fields
            .get("default_fields")
            .cloned()
            .or_else(|| fields.get("fields").cloned())
            .unwrap_or_default(),
    }
}

fn is_safe_color(value: &str) -> bool {
    let value = value.trim();
    value.starts_with('#')
        && (value.len() == 4 || value.len() == 7)
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
}

fn normalize_node_kind_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace([' ', '_'], "-")
        .to_lowercase()
}

fn canonical_node_kind(kind: &str) -> String {
    match kind {
        "knowledge-source" => "source".to_string(),
        "knowledge-entity" => "entity".to_string(),
        "knowledge-concept" => "concept".to_string(),
        "knowledge-card" => "knowledge_card".to_string(),
        "skill" | "skill-output" | "analysis" | "report" | "decomposition" | "distillation" => {
            "skill_output".to_string()
        }
        "plain" | "ordinary" => "note".to_string(),
        _ => kind.to_string(),
    }
}

fn infer_node_kind_from_path(relative: &str) -> String {
    let top = relative.split('/').next().unwrap_or("").trim();
    let top_lower = top.to_lowercase();
    if top_lower == "sources" || top.starts_with("01") {
        return "source".to_string();
    }
    if top_lower == "entities" {
        return "entity".to_string();
    }
    if top_lower == "concepts" {
        return "concept".to_string();
    }
    if top.starts_with("02") || top.starts_with("08") {
        return "skill_output".to_string();
    }
    if top.starts_with("03")
        || top.starts_with("04")
        || top.starts_with("05")
        || top.starts_with("06")
        || top.starts_with("07")
    {
        return "knowledge_card".to_string();
    }
    "note".to_string()
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

fn dedupe_relationships(relationships: &mut Vec<KnowledgeGraphRelation>) {
    let mut seen = HashSet::new();
    relationships.retain(|relation| {
        seen.insert(format!(
            "{}>{}>{}",
            relation.source_file, relation.target_file, relation.field_name
        ))
    });
}

fn mark_bidirectional_relationships(relationships: &mut [KnowledgeGraphRelation]) {
    let pairs = relationships
        .iter()
        .map(|relation| (relation.source_file.clone(), relation.target_file.clone()))
        .collect::<HashSet<_>>();
    for relation in relationships {
        relation.bidirectional =
            pairs.contains(&(relation.target_file.clone(), relation.source_file.clone()));
    }
}

fn enrich_nodes_with_graph_metadata(
    nodes: &mut [KnowledgeGraphNode],
    edges: &[KnowledgeGraphEdge],
    card_meta: &HashMap<String, KnowledgeCardMeta>,
    type_definitions: &HashMap<String, KnowledgeTypeDefinition>,
    work_references: &HashMap<String, Vec<String>>,
) {
    let mut inbound = HashMap::<String, usize>::new();
    let mut outbound = HashMap::<String, usize>::new();
    let mut referenced_by = HashMap::<String, Vec<String>>::new();
    for edge in edges {
        if edge.kind == "contains" {
            continue;
        }
        *outbound.entry(edge.source.clone()).or_default() += 1;
        *inbound.entry(edge.target.clone()).or_default() += 1;
        if edge.source.starts_with("card:") && edge.target.starts_with("card:") {
            referenced_by
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.trim_start_matches("card:").to_string());
        }
    }
    let mut title_counts = HashMap::<String, usize>::new();
    for node in nodes.iter().filter(|node| node.id.starts_with("card:")) {
        *title_counts.entry(node.title_key.clone()).or_default() += 1;
    }
    let mut concept_counts = HashMap::<String, usize>::new();
    for meta in card_meta.values() {
        for key in &meta.concept_keys {
            *concept_counts.entry(key.clone()).or_default() += 1;
        }
    }

    for node in nodes {
        let Some(meta) = card_meta.get(&node.id) else {
            continue;
        };
        node.inbound_count = inbound.get(&node.id).copied().unwrap_or(0);
        node.outbound_count = outbound.get(&node.id).copied().unwrap_or(0);
        node.referenced_by = sorted_limited(referenced_by.remove(&node.id).unwrap_or_default(), 8);
        node.used_by_works = sorted_limited(
            work_references.get(&node.id).cloned().unwrap_or_default(),
            8,
        );
        node.duplicate_title = title_counts.get(&node.title_key).copied().unwrap_or(0) > 1;
        node.duplicate_concept = meta
            .concept_keys
            .iter()
            .any(|key| concept_counts.get(key).copied().unwrap_or(0) > 1);
        node.stale_high_reference = node.inbound_count >= 3
            && node
                .updated_at
                .as_deref()
                .map(is_stale_year)
                .unwrap_or(true);
        if let Some(definition) = type_definitions.get(&normalize_type_name(&meta.kind)) {
            node.type_icon = definition.icon.clone();
            node.type_color = definition.color.clone();
            node.type_sort = definition.sort.clone();
            node.default_fields = definition.default_fields.clone();
        }
    }
}

fn is_stale_year(value: &str) -> bool {
    let Some(year) = value
        .get(0..4)
        .and_then(|prefix| prefix.parse::<u64>().ok())
    else {
        return false;
    };
    current_approx_year().saturating_sub(year) >= 1
}

fn current_approx_year() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    1970 + seconds / 31_557_600
}

fn relation_type_for_field(field: &str) -> String {
    match field {
        "source"
        | "derived_from"
        | "quotes"
        | "evidence"
        | "references_knowledge"
        | "adopts_knowledge"
        | "derived_from_knowledge"
        | "excerpted_from_project"
        | "abstracted_from_draft"
        | "distilled_from_memory" => field.to_string(),
        _ => "generic".to_string(),
    }
}

fn frontmatter_concept_keys(fields: &HashMap<String, Vec<String>>, title: &str) -> Vec<String> {
    let mut keys = HashSet::new();
    for value in [title.to_string()]
        .into_iter()
        .chain(frontmatter_field_values(fields, "title"))
        .chain(frontmatter_field_values(fields, "alias"))
        .chain(frontmatter_field_values(fields, "aliases"))
        .chain(frontmatter_field_values(fields, "concept"))
        .chain(frontmatter_field_values(fields, "concepts"))
    {
        for token in concept_key_candidates(&value) {
            keys.insert(token);
        }
    }
    keys.into_iter().collect()
}

fn frontmatter_field_values(fields: &HashMap<String, Vec<String>>, field: &str) -> Vec<String> {
    fields.get(field).cloned().unwrap_or_default()
}

fn concept_key_candidates(value: &str) -> Vec<String> {
    let cleaned = strip_markdown_extension(value)
        .trim_matches(|ch: char| ch == '[' || ch == ']' || ch == '"' || ch == '\'')
        .to_lowercase();
    let compact = cleaned
        .chars()
        .filter(|ch| ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect::<String>();
    if compact.chars().count() >= 2 {
        vec![compact]
    } else {
        Vec::new()
    }
}

fn collect_work_references(
    work_root: &Path,
    card_by_stem: &HashMap<String, String>,
    warnings: &mut Vec<String>,
) -> HashMap<String, Vec<String>> {
    let mut references = HashMap::<String, Vec<String>>::new();
    let mut visited = 0;
    collect_work_references_in_dir(
        work_root,
        work_root,
        0,
        &mut visited,
        card_by_stem,
        &mut references,
        warnings,
    );
    references
}

fn collect_work_references_in_dir(
    root: &Path,
    current: &Path,
    depth: usize,
    visited: &mut usize,
    card_by_stem: &HashMap<String, String>,
    references: &mut HashMap<String, Vec<String>>,
    warnings: &mut Vec<String>,
) {
    if depth > MAX_GRAPH_DEPTH || *visited >= MAX_GRAPH_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_work_references_in_dir(
                root,
                &path,
                depth + 1,
                visited,
                card_by_stem,
                references,
                warnings,
            );
            continue;
        }
        if !is_markdown(&path) || *visited >= MAX_GRAPH_FILES {
            continue;
        }
        *visited += 1;
        if fs::symlink_metadata(&path)
            .map(|metadata| metadata.len() > MAX_GRAPH_FILE_BYTES)
            .unwrap_or(false)
        {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                push_warning(
                    warnings,
                    format!(
                        "作品引用扫描读取失败（{}）：{error}",
                        path.to_string_lossy()
                    ),
                );
                continue;
            }
        };
        let relative = relative_path(root, &path);
        let mut linked_card_ids = HashSet::new();
        for link in extract_wikilinks(&content) {
            if let Some(target_id) = resolve_wikilink_target(&link, card_by_stem) {
                linked_card_ids.insert(target_id.clone());
            }
        }
        for (_, link) in extract_frontmatter_relation_links(&content) {
            if let Some(target_id) = resolve_wikilink_target(&link, card_by_stem) {
                linked_card_ids.insert(target_id.clone());
            }
        }
        for card_id in linked_card_ids {
            references
                .entry(card_id)
                .or_default()
                .push(relative.clone());
        }
    }
}

fn sorted_limited(mut values: Vec<String>, limit: usize) -> Vec<String> {
    values.sort();
    values.dedup();
    values.truncate(limit);
    values
}

fn resolve_wikilink_target<'a>(
    link: &str,
    card_by_stem: &'a HashMap<String, String>,
) -> Option<&'a String> {
    let normalized = strip_markdown_extension(&link.replace('\\', "/")).to_lowercase();
    card_by_stem.get(&normalized).or_else(|| {
        card_by_stem
            .iter()
            .find(|(candidate, _)| {
                candidate.ends_with(&format!("/{normalized}"))
                    || strip_markdown_extension(candidate).ends_with(&format!("/{normalized}"))
            })
            .map(|(_, id)| id)
    })
}

fn card_id_for_path(root: &Path, path: &Path) -> Option<String> {
    Some(format!("card:{}", relative_path(root, path)))
}

fn strip_markdown_extension(value: &str) -> String {
    let lower = value.to_lowercase();
    if lower.ends_with(".markdown") {
        return value[..value.len() - ".markdown".len()].to_string();
    }
    if lower.ends_with(".md") {
        return value[..value.len() - ".md".len()].to_string();
    }
    value.to_string()
}

fn file_stem_key(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn normalize_type_name(value: &str) -> String {
    normalize_node_kind_value(value)
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
        fs::write(
            root.join("人物").join("阿宁.md"),
            "关联 [[城市]] 和 [[地点/地标]]",
        )
        .expect("write card");
        fs::write(root.join("城市.md"), "地点").expect("write target");
        fs::create_dir_all(root.join("地点")).expect("create place folder");
        fs::write(root.join("地点").join("地标.md"), "路径地点").expect("write path target");

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
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.source == "card:人物/阿宁.md" && edge.target == "card:地点/地标.md"));
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
        fs::create_dir_all(root.join("type")).expect("create type folder");
        fs::write(
            root.join("type").join("method.md"),
            "---\ntype: Type\ntitle: method\nicon: M\ncolor: \"#336699\"\ndefault_fields:\n  - source_refs\n---\n",
        )
        .expect("write type");
        fs::write(
            root.join("03故事模型").join("技法.md"),
            "---\ntype: method\nrelated_to:\n  - \"[[来源]]\"\nadopts knowledge: \"[[作者Skill]]\"\nstatus: active\n---\n正文 [[来源]]",
        )
        .expect("write source");
        fs::write(root.join("来源.md"), "source").expect("write target");
        fs::write(root.join("08大神蒸馏").join("作者Skill.md"), "skill").expect("write skill");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:03故事模型/技法.md"
                && node.kind == "method"
                && node.type_icon.as_deref() == Some("M")
                && node.type_color.as_deref() == Some("#336699")
                && node.default_fields == vec!["source_refs"]
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:08大神蒸馏/作者Skill.md" && node.kind == "skill_output"
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
        assert!(graph.relationships.iter().any(|relation| {
            relation.source_file == "03故事模型/技法.md"
                && relation.target_file == "08大神蒸馏/作者Skill.md"
                && relation.field_name == "adopts_knowledge"
                && relation.relation_type == "adopts_knowledge"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:03故事模型/技法.md"
                && edge.target == "card:来源.md"
                && edge.kind == "wikilink"
        }));
    }

    #[test]
    fn graph_promotes_knowledge_infrastructure_and_work_backlinks() {
        let root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-infra-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let work_root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-work-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&work_root);
        fs::create_dir_all(root.join("sources")).expect("create sources");
        fs::create_dir_all(root.join("concepts")).expect("create concepts");
        fs::create_dir_all(root.join("entities")).expect("create entities");
        fs::create_dir_all(root.join("02拆解报告")).expect("create output");
        fs::create_dir_all(&work_root).expect("create work root");
        fs::write(
            root.join("sources").join("原始资料.md"),
            "---\ntype: knowledge_source\nsource_url: https://example.test\n---\n来源",
        )
        .expect("write source");
        fs::write(
            root.join("entities").join("作者原型.md"),
            "---\ntype: entity\nsource: \"[[原始资料]]\"\n---\n实体",
        )
        .expect("write entity");
        fs::write(
            root.join("concepts").join("反转钩子.md"),
            "---\ntype: concept\naliases: [场尾反转]\nevidence: \"[[原始资料]]\"\n---\n概念",
        )
        .expect("write concept");
        fs::write(
            root.join("concepts").join("场尾反转.md"),
            "---\ntype: knowledge_concept\nconcepts: [场尾反转]\nderived_from: \"[[原始资料]]\"\n---\n概念重复",
        )
        .expect("write duplicate concept");
        fs::write(root.join("02拆解报告").join("拆解.md"), "产物").expect("write output");
        fs::write(
            work_root.join("第一章.md"),
            "---\nreferences_knowledge:\n  - \"[[反转钩子]]\"\n---\n正文引用 [[作者原型]]",
        )
        .expect("write work");

        let graph = read_knowledge_graph_for_roots(&root, Some(&work_root)).expect("graph");

        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:sources/原始资料.md" && node.kind == "source" && node.has_source
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:entities/作者原型.md"
                && node.kind == "entity"
                && node.used_by_works == vec!["第一章.md"]
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:concepts/反转钩子.md"
                && node.kind == "concept"
                && node.has_source
                && node.used_by_works == vec!["第一章.md"]
                && node.duplicate_concept
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:02拆解报告/拆解.md" && node.kind == "skill_output"
        }));
        assert!(graph.relationships.iter().any(|relation| {
            relation.source_file == "concepts/反转钩子.md"
                && relation.target_file == "sources/原始资料.md"
                && relation.field_name == "evidence"
                && relation.relation_type == "evidence"
        }));
    }

    #[test]
    fn graph_surfaces_skill_review_marks_without_judging_them() {
        let root = std::env::temp_dir().join(format!(
            "wridian-knowledge-graph-review-test-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("03故事模型")).expect("create cards");
        fs::write(root.join("03故事模型").join("旧观点.md"), "旧观点").expect("write old card");
        fs::write(
            root.join("03故事模型").join("新观点.md"),
            "---\ntype: knowledge_card\n体检状态: 待核查\n冲突对象: [[旧观点]]\n不确定性: 只在单一案例里出现\n---\n> [!gap] 需要更多案例验证。\n",
        )
        .expect("write marked card");

        let graph = read_knowledge_graph(&root).expect("graph");

        assert!(graph.nodes.iter().any(|node| {
            node.id == "card:03故事模型/新观点.md"
                && node.review_status.as_deref() == Some("待核查")
                && node.has_conflict
                && node.has_uncertainty
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source == "card:03故事模型/新观点.md"
                && edge.target == "card:03故事模型/旧观点.md"
                && edge.kind == "frontmatter:冲突对象"
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
