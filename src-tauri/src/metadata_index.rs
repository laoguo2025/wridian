use crate::path_safety::safe_child_path;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_INDEX_FILES: usize = 1200;
const MAX_INDEX_DEPTH: usize = 10;
const MAX_INDEX_FILE_BYTES: u64 = 512 * 1024;
const MAX_INDEX_WARNINGS: usize = 40;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataIndexResponse {
    pub(crate) libraries: Vec<MetadataLibraryIndex>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataLibraryIndex {
    pub(crate) library: String,
    pub(crate) root_path: Option<String>,
    pub(crate) files: Vec<MetadataFile>,
    pub(crate) links: Vec<MetadataLink>,
    pub(crate) backlinks: Vec<MetadataBacklink>,
    pub(crate) unresolved_links: Vec<MetadataUnresolvedLink>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataFile {
    pub(crate) id: String,
    pub(crate) library: String,
    pub(crate) path: String,
    pub(crate) relative_path: String,
    pub(crate) title: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) frontmatter: BTreeMap<String, Vec<String>>,
    pub(crate) outgoing_links: Vec<MetadataLink>,
    pub(crate) backlinks: Vec<MetadataBacklink>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataLink {
    pub(crate) source_id: String,
    pub(crate) source_library: String,
    pub(crate) source_path: String,
    pub(crate) source_relative_path: String,
    pub(crate) raw_target: String,
    pub(crate) normalized_target: String,
    pub(crate) display_text: Option<String>,
    pub(crate) section: Option<String>,
    pub(crate) embed: bool,
    pub(crate) frontmatter_field: Option<String>,
    pub(crate) target_id: Option<String>,
    pub(crate) target_library: Option<String>,
    pub(crate) target_path: Option<String>,
    pub(crate) target_relative_path: Option<String>,
    pub(crate) resolved: bool,
    pub(crate) ambiguous: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataBacklink {
    pub(crate) target_id: String,
    pub(crate) target_library: String,
    pub(crate) target_path: String,
    pub(crate) target_relative_path: String,
    pub(crate) source_id: String,
    pub(crate) source_library: String,
    pub(crate) source_path: String,
    pub(crate) source_relative_path: String,
    pub(crate) raw_target: String,
    pub(crate) frontmatter_field: Option<String>,
    pub(crate) embed: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataUnresolvedLink {
    pub(crate) source_id: String,
    pub(crate) source_library: String,
    pub(crate) source_path: String,
    pub(crate) source_relative_path: String,
    pub(crate) raw_target: String,
    pub(crate) normalized_target: String,
    pub(crate) frontmatter_field: Option<String>,
    pub(crate) embed: bool,
    pub(crate) reason: String,
}

#[derive(Debug, Clone)]
struct ParsedFile {
    file: MetadataFile,
    raw_links: Vec<RawLink>,
}

#[derive(Debug, Clone)]
struct RawLink {
    raw_target: String,
    normalized_target: String,
    display_text: Option<String>,
    section: Option<String>,
    embed: bool,
    frontmatter_field: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolveCandidate {
    id: String,
    library: String,
    path: String,
    relative_path: String,
}

pub(crate) fn read_metadata_index_for_roots(
    roots: Vec<(&str, PathBuf)>,
) -> Result<MetadataIndexResponse, String> {
    let mut parsed = Vec::new();
    let mut warnings = Vec::new();

    for (library, root) in roots {
        if !root.is_dir() {
            continue;
        }
        let root = root
            .canonicalize()
            .map_err(|error| format!("{library} 元数据根目录解析失败：{error}"))?;
        collect_markdown_metadata(library, &root, &root, 0, &mut parsed, &mut warnings)?;
    }

    let resolver = build_resolver(&parsed);
    let mut files_by_id = parsed
        .iter()
        .map(|item| (item.file.id.clone(), item.file.clone()))
        .collect::<HashMap<_, _>>();
    let mut links = Vec::new();
    let mut backlinks = Vec::new();
    let mut unresolved = Vec::new();

    for item in &parsed {
        for raw in &item.raw_links {
            let resolution = resolve_link(&resolver, item, raw);
            let link = MetadataLink {
                source_id: item.file.id.clone(),
                source_library: item.file.library.clone(),
                source_path: item.file.path.clone(),
                source_relative_path: item.file.relative_path.clone(),
                raw_target: raw.raw_target.clone(),
                normalized_target: raw.normalized_target.clone(),
                display_text: raw.display_text.clone(),
                section: raw.section.clone(),
                embed: raw.embed,
                frontmatter_field: raw.frontmatter_field.clone(),
                target_id: resolution
                    .as_ref()
                    .and_then(|value| value.candidate.as_ref())
                    .map(|value| value.id.clone()),
                target_library: resolution
                    .as_ref()
                    .and_then(|value| value.candidate.as_ref())
                    .map(|value| value.library.clone()),
                target_path: resolution
                    .as_ref()
                    .and_then(|value| value.candidate.as_ref())
                    .map(|value| value.path.clone()),
                target_relative_path: resolution
                    .as_ref()
                    .and_then(|value| value.candidate.as_ref())
                    .map(|value| value.relative_path.clone()),
                resolved: resolution
                    .as_ref()
                    .and_then(|value| value.candidate.as_ref())
                    .is_some(),
                ambiguous: resolution
                    .as_ref()
                    .map(|value| value.ambiguous)
                    .unwrap_or(false),
            };

            if let Some(source) = files_by_id.get_mut(&item.file.id) {
                source.outgoing_links.push(link.clone());
            }

            if let Some(target) = resolution
                .as_ref()
                .and_then(|value| value.candidate.as_ref())
            {
                let backlink = MetadataBacklink {
                    target_id: target.id.clone(),
                    target_library: target.library.clone(),
                    target_path: target.path.clone(),
                    target_relative_path: target.relative_path.clone(),
                    source_id: item.file.id.clone(),
                    source_library: item.file.library.clone(),
                    source_path: item.file.path.clone(),
                    source_relative_path: item.file.relative_path.clone(),
                    raw_target: raw.raw_target.clone(),
                    frontmatter_field: raw.frontmatter_field.clone(),
                    embed: raw.embed,
                };
                if let Some(file) = files_by_id.get_mut(&target.id) {
                    file.backlinks.push(backlink.clone());
                }
                backlinks.push(backlink);
            } else {
                unresolved.push(MetadataUnresolvedLink {
                    source_id: item.file.id.clone(),
                    source_library: item.file.library.clone(),
                    source_path: item.file.path.clone(),
                    source_relative_path: item.file.relative_path.clone(),
                    raw_target: raw.raw_target.clone(),
                    normalized_target: raw.normalized_target.clone(),
                    frontmatter_field: raw.frontmatter_field.clone(),
                    embed: raw.embed,
                    reason: resolution
                        .map(|value| value.reason)
                        .unwrap_or_else(|| "not_found".to_string()),
                });
            }
            links.push(link);
        }
    }

    let mut by_library = HashMap::<String, MetadataLibraryIndex>::new();
    for (_, root) in files_by_id
        .values()
        .map(|file| (file.library.clone(), file.path.clone()))
    {
        let _ = root;
    }
    for (library, root) in parsed
        .iter()
        .map(|item| (item.file.library.clone(), root_path_from_file(&item.file)))
    {
        by_library
            .entry(library.clone())
            .or_insert(MetadataLibraryIndex {
                library,
                root_path: root,
                files: Vec::new(),
                links: Vec::new(),
                backlinks: Vec::new(),
                unresolved_links: Vec::new(),
            });
    }

    for file in files_by_id.into_values() {
        if let Some(library) = by_library.get_mut(&file.library) {
            library.files.push(file);
        }
    }
    for link in links {
        if let Some(library) = by_library.get_mut(&link.source_library) {
            library.links.push(link);
        }
    }
    for backlink in backlinks {
        if let Some(library) = by_library.get_mut(&backlink.target_library) {
            library.backlinks.push(backlink);
        }
    }
    for item in unresolved {
        if let Some(library) = by_library.get_mut(&item.source_library) {
            library.unresolved_links.push(item);
        }
    }

    let mut libraries = by_library.into_values().collect::<Vec<_>>();
    for library in &mut libraries {
        library
            .files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        library.links.sort_by(|left, right| {
            left.source_relative_path
                .cmp(&right.source_relative_path)
                .then_with(|| left.raw_target.cmp(&right.raw_target))
        });
        library.backlinks.sort_by(|left, right| {
            left.target_relative_path
                .cmp(&right.target_relative_path)
                .then_with(|| left.source_relative_path.cmp(&right.source_relative_path))
        });
        library.unresolved_links.sort_by(|left, right| {
            left.source_relative_path
                .cmp(&right.source_relative_path)
                .then_with(|| left.raw_target.cmp(&right.raw_target))
        });
    }
    libraries.sort_by(|left, right| left.library.cmp(&right.library));

    Ok(MetadataIndexResponse {
        libraries,
        warnings,
    })
}

pub(crate) fn read_library_metadata_index(
    library: &str,
    root: &Path,
) -> Result<MetadataLibraryIndex, String> {
    let response = read_metadata_index_for_roots(vec![(library, root.to_path_buf())])?;
    Ok(response
        .libraries
        .into_iter()
        .find(|item| item.library == library)
        .unwrap_or(MetadataLibraryIndex {
            library: library.to_string(),
            root_path: Some(root.to_string_lossy().into_owned()),
            files: Vec::new(),
            links: Vec::new(),
            backlinks: Vec::new(),
            unresolved_links: Vec::new(),
        }))
}

fn collect_markdown_metadata(
    library: &str,
    root: &Path,
    current: &Path,
    depth: usize,
    parsed: &mut Vec<ParsedFile>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    if depth > MAX_INDEX_DEPTH {
        push_warning(
            warnings,
            format!("元数据索引已跳过过深目录：{}", current.to_string_lossy()),
        );
        return Ok(());
    }
    if parsed.len() >= MAX_INDEX_FILES {
        push_warning(
            warnings,
            format!("元数据索引已达到最多 {MAX_INDEX_FILES} 个 Markdown 文件上限。"),
        );
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in
        fs::read_dir(current).map_err(|error| format!("元数据索引目录读取失败：{error}"))?
    {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => push_warning(warnings, format!("元数据索引目录项读取失败：{error}")),
        }
    }
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    for entry in entries {
        if parsed.len() >= MAX_INDEX_FILES {
            break;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if should_skip_entry(&name) {
            continue;
        }
        let Some(safe_path) = safe_child_path(root, &path, "元数据索引")? else {
            push_warning(
                warnings,
                format!(
                    "元数据索引已跳过越界或符号链接路径：{}",
                    path.to_string_lossy()
                ),
            );
            continue;
        };
        if safe_path.is_dir() {
            collect_markdown_metadata(library, root, &safe_path, depth + 1, parsed, warnings)?;
            continue;
        }
        if !is_markdown(&safe_path) {
            continue;
        }
        let metadata = fs::symlink_metadata(&safe_path)
            .map_err(|error| format!("元数据索引文件信息读取失败：{error}"))?;
        if metadata.file_type().is_symlink() || metadata.len() > MAX_INDEX_FILE_BYTES {
            push_warning(
                warnings,
                format!(
                    "元数据索引已跳过过大或链接文件：{}",
                    safe_path.to_string_lossy()
                ),
            );
            continue;
        }
        let content = match fs::read_to_string(&safe_path) {
            Ok(content) => content,
            Err(error) => {
                push_warning(
                    warnings,
                    format!(
                        "元数据索引读取失败（{}）：{error}",
                        safe_path.to_string_lossy()
                    ),
                );
                continue;
            }
        };
        parsed.push(parse_file_metadata(library, root, &safe_path, &content));
    }
    Ok(())
}

fn parse_file_metadata(library: &str, root: &Path, path: &Path, content: &str) -> ParsedFile {
    let relative_path = relative_path(root, path);
    let title = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| relative_path.clone());
    let (frontmatter, body) = split_frontmatter(content);
    let frontmatter = frontmatter.map(parse_frontmatter).unwrap_or_default();
    let aliases = collect_named_values(&frontmatter, &["alias", "aliases"]);
    let tags = collect_named_values(&frontmatter, &["tag", "tags"]);
    let mut raw_links = Vec::new();

    for (field, values) in &frontmatter {
        if is_system_field(field) {
            continue;
        }
        for value in values {
            raw_links.extend(extract_wikilinks(value).into_iter().map(|mut link| {
                link.frontmatter_field = Some(field.clone());
                link
            }));
        }
    }
    raw_links.extend(extract_wikilinks(body));
    dedupe_raw_links(&mut raw_links);

    ParsedFile {
        file: MetadataFile {
            id: file_id(library, &relative_path),
            library: library.to_string(),
            path: path.to_string_lossy().into_owned(),
            relative_path,
            title,
            aliases,
            tags,
            frontmatter,
            outgoing_links: Vec::new(),
            backlinks: Vec::new(),
        },
        raw_links,
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

fn parse_frontmatter(frontmatter: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    let mut active_key = String::new();
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            active_key = normalize_field(key);
            if active_key.is_empty() {
                continue;
            }
            let parsed = parse_frontmatter_value(value);
            fields.entry(active_key.clone()).or_default().extend(parsed);
            continue;
        }
        if active_key.is_empty() || !trimmed.starts_with('-') {
            continue;
        }
        fields
            .entry(active_key.clone())
            .or_default()
            .extend(parse_frontmatter_value(trimmed.trim_start_matches('-')));
    }
    fields
}

fn parse_frontmatter_value(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return trimmed
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(clean_scalar_value)
            .filter(|value| !value.is_empty())
            .collect();
    }
    vec![clean_scalar_value(trimmed)]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

fn clean_scalar_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn collect_named_values(frontmatter: &BTreeMap<String, Vec<String>>, keys: &[&str]) -> Vec<String> {
    let mut values = keys
        .iter()
        .flat_map(|key| frontmatter.get(*key).into_iter().flatten())
        .map(|value| value.trim().trim_start_matches('#').to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn extract_wikilinks(text: &str) -> Vec<RawLink> {
    let mut links = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while let Some(start_offset) = text[index..].find("[[") {
        let start = index + start_offset;
        let Some(end_offset) = text[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + end_offset;
        let embed = start > 0 && bytes.get(start - 1) == Some(&b'!');
        let inner = text[start + 2..end].trim();
        if !inner.is_empty() {
            if let Some(link) = parse_wikilink_inner(inner, embed) {
                links.push(link);
            }
        }
        index = end + 2;
    }
    links
}

fn parse_wikilink_inner(inner: &str, embed: bool) -> Option<RawLink> {
    let (target_part, display_text) = inner
        .split_once('|')
        .map(|(target, display)| (target.trim(), Some(display.trim().to_string())))
        .unwrap_or((inner.trim(), None));
    let (target, section) = target_part
        .split_once('#')
        .map(|(target, section)| (target.trim(), Some(section.trim().to_string())))
        .unwrap_or((target_part.trim(), None));
    if target.is_empty() {
        return None;
    }
    Some(RawLink {
        raw_target: target.to_string(),
        normalized_target: normalize_link_target(target),
        display_text: display_text.filter(|value| !value.is_empty()),
        section: section.filter(|value| !value.is_empty()),
        embed,
        frontmatter_field: None,
    })
}

fn build_resolver(parsed: &[ParsedFile]) -> HashMap<String, Vec<ResolveCandidate>> {
    let mut resolver = HashMap::<String, Vec<ResolveCandidate>>::new();
    for item in parsed {
        let candidate = ResolveCandidate {
            id: item.file.id.clone(),
            library: item.file.library.clone(),
            path: item.file.path.clone(),
            relative_path: item.file.relative_path.clone(),
        };
        for key in resolver_keys(&item.file) {
            resolver.entry(key).or_default().push(candidate.clone());
        }
    }
    for values in resolver.values_mut() {
        values.sort_by(|left, right| left.id.cmp(&right.id));
        values.dedup_by(|left, right| left.id == right.id);
    }
    resolver
}

struct Resolution {
    candidate: Option<ResolveCandidate>,
    ambiguous: bool,
    reason: String,
}

fn resolve_link(
    resolver: &HashMap<String, Vec<ResolveCandidate>>,
    source: &ParsedFile,
    raw: &RawLink,
) -> Option<Resolution> {
    let mut keys = Vec::new();
    if let Some(parent) = source.file.relative_path.rsplit_once('/') {
        keys.push(normalize_link_target(&format!(
            "{}/{}",
            parent.0, raw.raw_target
        )));
    }
    keys.push(raw.normalized_target.clone());
    keys.push(normalize_link_target(
        raw.raw_target.split('/').last().unwrap_or(&raw.raw_target),
    ));

    let mut candidates = Vec::<ResolveCandidate>::new();
    for key in keys {
        if let Some(values) = resolver.get(&key) {
            for value in values {
                if !candidates.iter().any(|candidate| candidate.id == value.id) {
                    candidates.push(value.clone());
                }
            }
            if values
                .iter()
                .any(|value| value.library == source.file.library)
            {
                break;
            }
        }
    }

    if candidates.is_empty() {
        return Some(Resolution {
            candidate: None,
            ambiguous: false,
            reason: "not_found".to_string(),
        });
    }
    candidates.sort_by_key(|candidate| {
        if candidate.library == source.file.library {
            0
        } else {
            1
        }
    });
    let same_priority = candidates
        .iter()
        .filter(|candidate| candidate.library == candidates[0].library)
        .cloned()
        .collect::<Vec<_>>();
    if same_priority.len() == 1 {
        return Some(Resolution {
            candidate: Some(same_priority[0].clone()),
            ambiguous: false,
            reason: "resolved".to_string(),
        });
    }
    Some(Resolution {
        candidate: None,
        ambiguous: true,
        reason: "ambiguous".to_string(),
    })
}

fn resolver_keys(file: &MetadataFile) -> Vec<String> {
    let mut keys = vec![
        normalize_link_target(&file.relative_path),
        normalize_link_target(strip_markdown_extension(&file.relative_path)),
        normalize_link_target(&file.title),
    ];
    keys.extend(
        file.aliases
            .iter()
            .map(|alias| normalize_link_target(alias)),
    );
    keys.sort();
    keys.dedup();
    keys
}

fn normalize_field(field: &str) -> String {
    field.trim().replace(' ', "_").to_lowercase()
}

fn is_system_field(field: &str) -> bool {
    field.starts_with('_')
        || matches!(
            field,
            "id" | "type"
                | "kind"
                | "card_type"
                | "wridian_type"
                | "category"
                | "status"
                | "title"
                | "created_at"
                | "updated_at"
                | "created"
                | "updated"
                | "tags"
                | "tag"
                | "aliases"
                | "alias"
        )
}

fn normalize_link_target(value: &str) -> String {
    strip_markdown_extension(&value.replace('\\', "/"))
        .trim()
        .trim_start_matches("./")
        .to_lowercase()
}

fn strip_markdown_extension(value: &str) -> &str {
    value
        .strip_suffix(".markdown")
        .or_else(|| value.strip_suffix(".md"))
        .unwrap_or(value)
}

fn file_id(library: &str, relative_path: &str) -> String {
    format!("{library}:{relative_path}")
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn root_path_from_file(file: &MetadataFile) -> Option<String> {
    let path = Path::new(&file.path);
    let relative = Path::new(&file.relative_path);
    let mut root = path.to_path_buf();
    for _ in relative.components() {
        root.pop();
    }
    Some(root.to_string_lossy().into_owned())
}

fn should_skip_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | ".wridian" | ".wridian-trash"
    ) || name.starts_with('.')
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn dedupe_raw_links(links: &mut Vec<RawLink>) {
    let mut seen = HashSet::new();
    links.retain(|link| {
        seen.insert(format!(
            "{}|{}|{}|{}",
            link.raw_target,
            link.frontmatter_field.as_deref().unwrap_or_default(),
            link.section.as_deref().unwrap_or_default(),
            link.embed
        ))
    });
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_INDEX_WARNINGS {
        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-metadata-index-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn library<'a>(index: &'a MetadataIndexResponse, name: &str) -> &'a MetadataLibraryIndex {
        index
            .libraries
            .iter()
            .find(|library| library.library == name)
            .expect("library exists")
    }

    #[test]
    fn metadata_index_resolves_alias_backlinks_and_unresolved_links() {
        let works = temp_dir("works");
        let knowledge = temp_dir("knowledge");
        fs::write(
            works.join("第一章.md"),
            "---\nreferences_knowledge:\n  - \"[[三幕式]]\"\n---\n正文 [[不存在]]",
        )
        .expect("write work");
        fs::write(
            knowledge.join("故事结构.md"),
            "---\naliases: [三幕式]\ntags: [story, method]\n---\n# 故事结构",
        )
        .expect("write knowledge");

        let index = read_metadata_index_for_roots(vec![("works", works), ("knowledge", knowledge)])
            .expect("read index");
        let works_index = library(&index, "works");
        let knowledge_index = library(&index, "knowledge");

        assert_eq!(works_index.links.len(), 2);
        assert!(works_index.links.iter().any(|link| {
            link.raw_target == "三幕式"
                && link.resolved
                && link.target_library.as_deref() == Some("knowledge")
                && link.frontmatter_field.as_deref() == Some("references_knowledge")
        }));
        assert!(works_index
            .unresolved_links
            .iter()
            .any(|link| link.raw_target == "不存在" && link.reason == "not_found"));
        assert!(knowledge_index.backlinks.iter().any(|backlink| {
            backlink.source_library == "works" && backlink.raw_target == "三幕式"
        }));
        assert_eq!(knowledge_index.files[0].aliases, vec!["三幕式"]);
    }

    #[test]
    fn metadata_index_resolves_relative_links_before_global_titles() {
        let works = temp_dir("relative");
        fs::create_dir_all(works.join("A")).expect("create A");
        fs::create_dir_all(works.join("B")).expect("create B");
        fs::write(works.join("A").join("目标.md"), "A target").expect("write A target");
        fs::write(works.join("B").join("目标.md"), "B target").expect("write B target");
        fs::write(works.join("A").join("来源.md"), "[[目标]]").expect("write source");

        let index = read_metadata_index_for_roots(vec![("works", works)]).expect("read index");
        let works_index = library(&index, "works");
        let link = works_index
            .links
            .iter()
            .find(|link| link.source_relative_path == "A/来源.md")
            .expect("link exists");

        assert!(link.resolved);
        assert_eq!(link.target_relative_path.as_deref(), Some("A/目标.md"));
    }

    #[test]
    fn metadata_index_reports_ambiguous_global_links() {
        let works = temp_dir("ambiguous");
        fs::create_dir_all(works.join("A")).expect("create A");
        fs::create_dir_all(works.join("B")).expect("create B");
        fs::write(works.join("A").join("目标.md"), "A target").expect("write A target");
        fs::write(works.join("B").join("目标.md"), "B target").expect("write B target");
        fs::write(works.join("来源.md"), "[[目标]]").expect("write source");

        let index = read_metadata_index_for_roots(vec![("works", works)]).expect("read index");
        let works_index = library(&index, "works");

        assert!(works_index
            .unresolved_links
            .iter()
            .any(|link| link.raw_target == "目标" && link.reason == "ambiguous"));
    }
}
