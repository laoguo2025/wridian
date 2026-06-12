use crate::metadata_index::{read_library_metadata_index, MetadataFile, MetadataLibraryIndex};
use crate::path_safety::is_symlink_or_reparse;
use crate::runtime::{ensure_workspace, iso_timestamp, wridian_data_dir};
use crate::workspace::resolved_knowledge_root;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const CACHE_SCHEMA_VERSION: u32 = 1;
const CACHE_DIR: &str = ".wridian";
const MANIFEST_FILE: &str = "knowledge-manifest.json";
const MAX_SEARCH_LIMIT: usize = 20;
const DEFAULT_SEARCH_LIMIT: usize = 8;
const MAX_SEARCH_FILE_BYTES: u64 = 512 * 1024;
const HOT_TOP_LIMIT: usize = 12;
const HEALTH_ISSUE_LIMIT: usize = 18;
const HEALTH_SKILL_CANDIDATE_LIMIT: usize = 12;
const REQUIRED_KNOWLEDGE_DIRS: &[&str] = &[
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
const KNOWLEDGE_USAGE_GUIDE: &str = "# 知识库使用说明\n\n## 知识库体检\n\nWridian 的知识库体检会刷新索引、更新 hot、生成 fold、扫描结构和关系，并把报告写入 `00知识库治理`。\n\n## 一级目录\n\n- `00知识库治理`：体检报告、调用记录台账、治理说明。\n- `01原始资料`：未加工的一手资料。\n- `02拆解报告`：作品拆解、案例分析和中间报告。\n- `03故事模型`：可复用故事结构、叙事模型和判断框架。\n- `04人物原型`：人物类型、关系模式、角色弧光和人设参考。\n- `05情节方程`：冲突、反转、钩子、节奏和情节公式。\n- `06写作技法`：对白、场景、风格、爽点、短剧卡点等技法卡。\n- `07综合素材`：可复用素材、清单、灵感和跨分类资料。\n- `08大神蒸馏`：作者方法论、作者 skill 和版本记录。\n- `09文件归档`：废弃、过期或待删除文件。\n\n这些分类是默认模板。可以在 Wridian 里调整目录；体检会按现场结构给出修复建议。\n";
const CALL_LOG_TEMPLATE: &str = "# Wridian知识库 · 调用记录台账\n\n> 本台账记录知识卡被真实使用后的表现，用于统计调用频率、最近调用、调用表现和进化方向。\n\n## 调用记录表\n\n| 日期 | 调用作品 | 知识卡 | 调用频率 | 最近调用 | 调用表现 | 关联卡片 | 进化方向 | 备注 |\n|---|---|---|---:|---|---|---|---|---|\n|  |  |  |  |  |  |  |  |  |\n\n## 质量 × 频率四象限\n\n| 质量 | 频率 | 结果 | 处理原则 |\n|---|---|---|---|\n| 低质量 | 低频 | 垃圾 | 废弃、合并或封存。 |\n| 低质量 | 高频 | 内耗 | 优先重写或降评级。 |\n| 高质量 | 低频 | 浪费 | 补入口、补关联卡片、补适用场景。 |\n| 高质量 | 高频 | 杠杆 | 评为金卡，重点维护，优先产品化和系统化。 |\n";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeCacheManifest {
    schema_version: u32,
    generated_at: String,
    root_path: String,
    files: Vec<KnowledgeCacheFile>,
    link_count: usize,
    unresolved_link_count: usize,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeCacheFile {
    relative_path: String,
    path: String,
    title: String,
    aliases: Vec<String>,
    tags: Vec<String>,
    modified_ms: u128,
    len: u64,
    sha256: String,
    outgoing_count: usize,
    backlink_count: usize,
    unresolved_count: usize,
    token_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeSearchInput {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeSearchHit {
    path: String,
    relative_path: String,
    title: String,
    snippet: String,
    score: f64,
    tags: Vec<String>,
    aliases: Vec<String>,
    outgoing_count: usize,
    backlink_count: usize,
    unresolved_count: usize,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthResponse {
    ok: bool,
    checked_at: String,
    score: u8,
    skill_maturity_score: u8,
    summary: KnowledgeHealthSummary,
    issues: Vec<KnowledgeHealthIssue>,
    skill_candidates: Vec<KnowledgeSkillCandidate>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthWorkflowResponse {
    ok: bool,
    checked_at: String,
    score: u8,
    skill_maturity_score: u8,
    summary: KnowledgeHealthSummary,
    issues: Vec<KnowledgeHealthIssue>,
    skill_candidates: Vec<KnowledgeSkillCandidate>,
    warnings: Vec<String>,
    report_path: String,
    report_relative_path: String,
    manifest_path: String,
    hot_path: String,
    fold_path: String,
    auto_fixes: Vec<KnowledgeHealthFixItem>,
    pending_fixes: Vec<KnowledgeHealthFixItem>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthFixResponse {
    ok: bool,
    checked_at: String,
    score: u8,
    skill_maturity_score: u8,
    summary: KnowledgeHealthSummary,
    issues: Vec<KnowledgeHealthIssue>,
    skill_candidates: Vec<KnowledgeSkillCandidate>,
    warnings: Vec<String>,
    report_path: String,
    report_relative_path: String,
    manifest_path: String,
    hot_path: String,
    fold_path: String,
    auto_fixes: Vec<KnowledgeHealthFixItem>,
    pending_fixes: Vec<KnowledgeHealthFixItem>,
    applied_fixes: Vec<KnowledgeHealthFixItem>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthFixItem {
    id: String,
    title: String,
    detail: String,
    path: Option<String>,
    risk: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthSummary {
    file_count: usize,
    link_count: usize,
    unresolved_link_count: usize,
    frontmatter_file_count: usize,
    tagged_file_count: usize,
    source_coverage_count: usize,
    formal_skill_file_count: usize,
    skill_candidate_count: usize,
    orphan_file_count: usize,
    generated_file_count: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeHealthIssue {
    severity: String,
    title: String,
    detail: String,
    path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeSkillCandidate {
    path: String,
    relative_path: String,
    title: String,
    score: u8,
    reasons: Vec<String>,
    missing: Vec<String>,
}

#[tauri::command]
pub(crate) fn wridian_search_knowledge_bm25(
    input: KnowledgeSearchInput,
) -> Result<Vec<KnowledgeSearchHit>, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = resolved_knowledge_root(&data_dir)?;
    let index = read_knowledge_index(&root)?;
    search_knowledge_bm25(&index, &input.query, input.limit)
}

#[tauri::command]
pub(crate) fn wridian_run_knowledge_health_check() -> Result<KnowledgeHealthWorkflowResponse, String>
{
    run_knowledge_health_workflow(Vec::new())
}

#[tauri::command]
pub(crate) fn wridian_fix_knowledge_health_low_risk() -> Result<KnowledgeHealthFixResponse, String>
{
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = resolved_knowledge_root(&data_dir)?;
    let applied_fixes = apply_low_risk_knowledge_fixes(&root)?;
    let workflow = run_knowledge_health_workflow(applied_fixes.clone())?;
    Ok(KnowledgeHealthFixResponse {
        ok: workflow.ok,
        checked_at: workflow.checked_at,
        score: workflow.score,
        skill_maturity_score: workflow.skill_maturity_score,
        summary: workflow.summary,
        issues: workflow.issues,
        skill_candidates: workflow.skill_candidates,
        warnings: workflow.warnings,
        report_path: workflow.report_path,
        report_relative_path: workflow.report_relative_path,
        manifest_path: workflow.manifest_path,
        hot_path: workflow.hot_path,
        fold_path: workflow.fold_path,
        auto_fixes: workflow.auto_fixes,
        pending_fixes: workflow.pending_fixes,
        applied_fixes,
    })
}

fn read_knowledge_index(root: &Path) -> Result<MetadataLibraryIndex, String> {
    if !root.is_dir() {
        return Ok(MetadataLibraryIndex {
            library: "knowledge".to_string(),
            root_path: Some(root.to_string_lossy().into_owned()),
            files: Vec::new(),
            links: Vec::new(),
            backlinks: Vec::new(),
            unresolved_links: Vec::new(),
        });
    }
    read_library_metadata_index("knowledge", root)
}

fn audit_knowledge_health(
    index: &MetadataLibraryIndex,
    manifest: &KnowledgeCacheManifest,
    checked_at: String,
) -> KnowledgeHealthResponse {
    let generated_paths = generated_relative_paths(index);
    let semantic_files = semantic_knowledge_files(index);
    let file_count = semantic_files.len();
    let frontmatter_file_count = semantic_files
        .iter()
        .filter(|file| has_frontmatter(file))
        .count();
    let tagged_file_count = semantic_files
        .iter()
        .filter(|file| !file.tags.is_empty())
        .count();
    let source_coverage_count = semantic_files
        .iter()
        .filter(|file| has_source_signal(file))
        .count();
    let generated_file_count = index
        .files
        .iter()
        .filter(|file| is_generated_knowledge_file(file))
        .count();
    let formal_skill_file_count = semantic_files
        .iter()
        .filter(|file| is_formal_skill_file(file))
        .count();
    let orphan_file_count = semantic_files
        .iter()
        .filter(|file| {
            semantic_backlink_count(file, &generated_paths) == 0
                && semantic_outgoing_count(file, &generated_paths) == 0
        })
        .count();
    let candidates = score_skill_candidates(index, manifest);
    let summary = KnowledgeHealthSummary {
        file_count,
        link_count: semantic_link_count(index, &generated_paths),
        unresolved_link_count: semantic_unresolved_link_count(index, &generated_paths),
        frontmatter_file_count,
        tagged_file_count,
        source_coverage_count,
        formal_skill_file_count,
        skill_candidate_count: candidates.len(),
        orphan_file_count,
        generated_file_count,
    };
    let score = knowledge_health_score(&summary, manifest.warnings.len());
    let skill_maturity_score = knowledge_skill_maturity_score(&summary, &candidates);
    let mut issues = knowledge_health_issues(index, &summary);
    if manifest.warnings.len() > 0 {
        issues.push(KnowledgeHealthIssue {
            severity: "medium".to_string(),
            title: "索引过程存在跳过项".to_string(),
            detail: format!(
                "本次体检收到 {} 条扫描或读取警告。",
                manifest.warnings.len()
            ),
            path: None,
        });
    }
    issues.truncate(HEALTH_ISSUE_LIMIT);

    KnowledgeHealthResponse {
        ok: true,
        checked_at,
        score,
        skill_maturity_score,
        summary,
        issues,
        skill_candidates: candidates
            .into_iter()
            .take(HEALTH_SKILL_CANDIDATE_LIMIT)
            .collect(),
        warnings: manifest.warnings.clone(),
    }
}

fn run_knowledge_health_workflow(
    applied_fixes: Vec<KnowledgeHealthFixItem>,
) -> Result<KnowledgeHealthWorkflowResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let root = resolved_knowledge_root(&data_dir)?;
    let index = read_knowledge_index(&root)?;
    let manifest = build_manifest(&root, &index, read_manifest(&root).ok().as_ref())?;
    write_manifest(&root, &manifest)?;
    let checked_at = iso_timestamp();
    let hot_content = render_hot_cache(&manifest, &index, &checked_at);
    let hot_path = root.join("hot.md");
    safe_write_knowledge_file(&root, &hot_path, &hot_content, "知识 hot 缓存")?;
    let fold_content = render_fold(&manifest, &index, &checked_at);
    let fold_dir = root.join("00知识库治理").join("folds");
    safe_create_knowledge_dir(&root, &fold_dir, "知识 fold 目录")?;
    let fold_path = fold_dir.join(format!(
        "knowledge-fold-{}.md",
        compact_timestamp(&checked_at)
    ));
    safe_write_knowledge_file(&root, &fold_path, &fold_content, "知识 fold")?;
    let health = audit_knowledge_health(&index, &manifest, checked_at.clone());
    let auto_fixes = collect_low_risk_knowledge_fixes(&root);
    let pending_fixes = collect_pending_knowledge_fixes(&health);
    let report_relative_path = format!(
        "00知识库治理/知识库体检-{}.md",
        health_report_timestamp(&checked_at)
    );
    let report_path = root.join(&report_relative_path);
    let report = render_health_report(
        &health,
        &root,
        &manifest,
        &hot_path,
        &fold_path,
        &auto_fixes,
        &pending_fixes,
        &applied_fixes,
    );
    safe_write_knowledge_file(&root, &report_path, &report, "知识库体检报告")?;
    Ok(KnowledgeHealthWorkflowResponse {
        ok: true,
        checked_at: health.checked_at,
        score: health.score,
        skill_maturity_score: health.skill_maturity_score,
        summary: health.summary,
        issues: health.issues,
        skill_candidates: health.skill_candidates,
        warnings: health.warnings,
        report_path: report_path.to_string_lossy().into_owned(),
        report_relative_path,
        manifest_path: root
            .join(CACHE_DIR)
            .join(MANIFEST_FILE)
            .to_string_lossy()
            .into_owned(),
        hot_path: hot_path.to_string_lossy().into_owned(),
        fold_path: fold_path.to_string_lossy().into_owned(),
        auto_fixes,
        pending_fixes,
    })
}

fn collect_low_risk_knowledge_fixes(root: &Path) -> Vec<KnowledgeHealthFixItem> {
    let mut fixes = Vec::new();
    for dir in REQUIRED_KNOWLEDGE_DIRS {
        let path = root.join(dir);
        if !path.is_dir() {
            fixes.push(KnowledgeHealthFixItem {
                id: format!("create-dir:{dir}"),
                title: "补齐知识库一级目录".to_string(),
                detail: format!("创建缺失目录 `{dir}`。"),
                path: Some((*dir).to_string()),
                risk: "low".to_string(),
            });
        }
    }
    let usage = root.join("知识库使用说明.md");
    if !usage.is_file() {
        fixes.push(KnowledgeHealthFixItem {
            id: "create-file:知识库使用说明.md".to_string(),
            title: "补齐知识库使用说明".to_string(),
            detail: "写入默认目录职责和体检说明。".to_string(),
            path: Some("知识库使用说明.md".to_string()),
            risk: "low".to_string(),
        });
    }
    let call_log = root.join("00知识库治理").join("调用记录台账.md");
    if !call_log.is_file() {
        fixes.push(KnowledgeHealthFixItem {
            id: "create-file:00知识库治理/调用记录台账.md".to_string(),
            title: "补齐调用记录台账".to_string(),
            detail: "写入质量与频率四象限台账模板。".to_string(),
            path: Some("00知识库治理/调用记录台账.md".to_string()),
            risk: "low".to_string(),
        });
    }
    fixes
}

fn apply_low_risk_knowledge_fixes(root: &Path) -> Result<Vec<KnowledgeHealthFixItem>, String> {
    let planned = collect_low_risk_knowledge_fixes(root);
    for item in &planned {
        match item.id.as_str() {
            id if id.starts_with("create-dir:") => {
                if let Some(path) = &item.path {
                    safe_create_knowledge_dir(root, Path::new(path), "知识库目录修复")?;
                }
            }
            "create-file:知识库使用说明.md" => write_if_missing(
                root,
                Path::new("知识库使用说明.md"),
                KNOWLEDGE_USAGE_GUIDE,
                "知识库使用说明",
            )?,
            "create-file:00知识库治理/调用记录台账.md" => write_if_missing(
                root,
                Path::new("00知识库治理").join("调用记录台账.md").as_path(),
                CALL_LOG_TEMPLATE,
                "调用记录台账",
            )?,
            _ => {}
        }
    }
    Ok(planned)
}

fn write_if_missing(root: &Path, path: &Path, content: &str, label: &str) -> Result<(), String> {
    let target = resolve_knowledge_target(root, path, label)?;
    if target.exists() {
        ensure_safe_knowledge_write_target(root, &target, label)?;
        return Ok(());
    }
    safe_write_knowledge_file(root, &target, content, label)
}

fn safe_write_knowledge_file(
    root: &Path,
    path: &Path,
    content: &str,
    label: &str,
) -> Result<(), String> {
    let target = resolve_knowledge_target(root, path, label)?;
    ensure_safe_knowledge_parent(root, &target, label)?;
    ensure_safe_knowledge_write_target(root, &target, label)?;
    fs::write(target, content).map_err(|error| format!("{label}写入失败：{error}"))
}

fn safe_create_knowledge_dir(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    let root = canonical_knowledge_root(root, label)?;
    let target = resolve_knowledge_target_from_canonical_root(&root, path, label)?;
    let relative = target
        .strip_prefix(&root)
        .map_err(|_| format!("{label}路径不在知识库根目录内。"))?;
    let mut current = root;
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return Err(format!("{label}路径包含非法片段。"));
        };
        current.push(segment);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if is_symlink_or_reparse(&metadata) {
                    return Err(format!("{label}不能指向符号链接或重解析点。"));
                }
                if !metadata.is_dir() {
                    return Err(format!("{label}路径已存在但不是目录。"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|error| format!("{label}创建失败：{error}"))?;
            }
            Err(error) => return Err(format!("{label}路径检查失败：{error}")),
        }
    }
    Ok(())
}

fn ensure_safe_knowledge_parent(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        safe_create_knowledge_dir(root, parent, &format!("{label}目录"))?;
    }
    Ok(())
}

fn ensure_safe_knowledge_write_target(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    let target = resolve_knowledge_target(root, path, label)?;
    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if is_symlink_or_reparse(&metadata) {
            return Err(format!("{label}不能写入符号链接或重解析点。"));
        }
        if metadata.is_dir() {
            return Err(format!("{label}目标已存在但不是文件。"));
        }
    }
    Ok(())
}

fn canonical_knowledge_root(root: &Path, label: &str) -> Result<PathBuf, String> {
    root.canonicalize()
        .map_err(|error| format!("{label}知识库根目录解析失败：{error}"))
}

fn resolve_knowledge_target(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    let root = canonical_knowledge_root(root, label)?;
    resolve_knowledge_target_from_canonical_root(&root, path, label)
}

fn resolve_knowledge_target_from_canonical_root(
    root: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    Ok(root.join(relative_knowledge_path(root, path, label)?))
}

fn relative_knowledge_path(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!("{label}路径包含非法片段。"));
    }
    let relative = if path.is_absolute() {
        strip_prefix_compatible(path, root)
            .ok_or_else(|| format!("{label}路径不在知识库根目录内。"))?
    } else {
        path.to_path_buf()
    };
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("{label}路径包含非法片段。"));
    }
    Ok(relative)
}

fn strip_prefix_compatible(path: &Path, root: &Path) -> Option<PathBuf> {
    path.strip_prefix(root)
        .ok()
        .map(Path::to_path_buf)
        .or_else(|| {
            let root_text = normalize_path_prefix_text(root);
            let path_text = normalize_path_prefix_text(path);
            if path_text.eq_ignore_ascii_case(&root_text) {
                return Some(PathBuf::new());
            }
            let root_with_separator = if root_text.ends_with('\\') || root_text.ends_with('/') {
                root_text
            } else {
                format!("{root_text}\\")
            };
            if path_text
                .to_ascii_lowercase()
                .starts_with(&root_with_separator.to_ascii_lowercase())
            {
                let byte_start = root_with_separator.len();
                return Some(PathBuf::from(&path_text[byte_start..]));
            }
            None
        })
}

fn normalize_path_prefix_text(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', r"\")
}

fn collect_pending_knowledge_fixes(
    health: &KnowledgeHealthResponse,
) -> Vec<KnowledgeHealthFixItem> {
    let mut fixes = Vec::new();
    for issue in &health.issues {
        if issue.severity == "high" || issue.title.contains("未解析") {
            fixes.push(KnowledgeHealthFixItem {
                id: format!(
                    "confirm:{}:{}",
                    issue.title,
                    issue.path.as_deref().unwrap_or("global")
                ),
                title: format!("待确认：{}", issue.title),
                detail: issue.detail.clone(),
                path: issue.path.clone(),
                risk: "high".to_string(),
            });
        }
    }
    for candidate in health.skill_candidates.iter().take(8) {
        fixes.push(KnowledgeHealthFixItem {
            id: format!("skill-candidate:{}", candidate.relative_path),
            title: "待确认：skill 化候选".to_string(),
            detail: format!(
                "{}：{}{}",
                candidate.title,
                candidate.reasons.join(" / "),
                if candidate.missing.is_empty() {
                    String::new()
                } else {
                    format!("；待补 {}", candidate.missing.join("、"))
                }
            ),
            path: Some(candidate.relative_path.clone()),
            risk: "high".to_string(),
        });
    }
    fixes
}

fn render_health_report(
    health: &KnowledgeHealthResponse,
    root: &Path,
    manifest: &KnowledgeCacheManifest,
    hot_path: &Path,
    fold_path: &Path,
    auto_fixes: &[KnowledgeHealthFixItem],
    pending_fixes: &[KnowledgeHealthFixItem],
    applied_fixes: &[KnowledgeHealthFixItem],
) -> String {
    let mut lines = vec![
        "---".to_string(),
        "wridian_generated: true".to_string(),
        "wridian_type: knowledge_health_report".to_string(),
        format!("checked_at: \"{}\"", health.checked_at),
        "---".to_string(),
        String::new(),
        format!("# 知识库体检 {}", health.checked_at),
        String::new(),
        "## 总览".to_string(),
        format!("- 知识库：`{}`", root.to_string_lossy().replace('\\', "/")),
        format!("- 健康分：{}", health.score),
        format!("- skill 成熟度：{}", health.skill_maturity_score),
        format!(
            "- 文件：{}，关系：{}，断链：{}",
            health.summary.file_count,
            health.summary.link_count,
            health.summary.unresolved_link_count
        ),
        format!(
            "- frontmatter：{}，标签：{}，来源：{}",
            health.summary.frontmatter_file_count,
            health.summary.tagged_file_count,
            health.summary.source_coverage_count
        ),
        format!(
            "- 孤立知识卡：{}，skill 化候选：{}",
            health.summary.orphan_file_count, health.summary.skill_candidate_count
        ),
        String::new(),
        "## 本次自动步骤".to_string(),
        format!(
            "- 刷新缓存：`{}`，{} 个文件。",
            format!("{CACHE_DIR}/{MANIFEST_FILE}").replace('\\', "/"),
            manifest.files.len()
        ),
        format!("- 更新 hot：`{}`。", relative_display(root, hot_path)),
        format!("- 生成 fold：`{}`。", relative_display(root, fold_path)),
        "- 运行原生体检：结构、链接、frontmatter、来源、孤岛、skill 化候选。".to_string(),
        String::new(),
        "## 主要问题".to_string(),
    ];
    if health.issues.is_empty() {
        lines.push("- 暂无高优先级问题。".to_string());
    } else {
        for issue in &health.issues {
            lines.push(format!(
                "- 【{}】{}{}：{}",
                severity_label(&issue.severity),
                issue
                    .path
                    .as_deref()
                    .map(|path| format!("`{path}` "))
                    .unwrap_or_default(),
                issue.title,
                issue.detail
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 可自动修复项".to_string());
    if auto_fixes.is_empty() {
        lines.push("- 暂无待执行低风险修复。".to_string());
    } else {
        for item in auto_fixes {
            lines.push(format_health_fix_line(item));
        }
    }
    lines.push(String::new());
    lines.push("## 已执行修复".to_string());
    if applied_fixes.is_empty() {
        lines.push("- 本次未执行一键修复。".to_string());
    } else {
        for item in applied_fixes {
            lines.push(format_health_fix_line(item));
        }
    }
    lines.push(String::new());
    lines.push("## 待确认修复清单".to_string());
    if pending_fixes.is_empty() {
        lines.push("- 暂无待确认修复。".to_string());
    } else {
        for item in pending_fixes {
            lines.push(format_health_fix_line(item));
        }
    }
    lines.push(String::new());
    lines.push("## skill 化候选".to_string());
    if health.skill_candidates.is_empty() {
        lines.push("- 暂无达到阈值的候选。".to_string());
    } else {
        for candidate in &health.skill_candidates {
            lines.push(format!(
                "- `{}`：{} 分；{}{}",
                candidate.relative_path,
                candidate.score,
                candidate.reasons.join(" / "),
                if candidate.missing.is_empty() {
                    String::new()
                } else {
                    format!("；待补 {}", candidate.missing.join("、"))
                }
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 规则边界".to_string());
    lines.push("- 一键修复只执行低风险、确定性动作。".to_string());
    lines.push(
        "- 合并卡片、改写观点、归档知识卡、处理冲突和不确定性必须人工确认后再改。".to_string(),
    );
    lines.push("- 生成文件不会进入知识图谱节点、BM25 检索候选或体检分母。".to_string());
    if !health.warnings.is_empty() {
        lines.push(String::new());
        lines.push("## 扫描警告".to_string());
        for warning in &health.warnings {
            lines.push(format!("- {warning}"));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn format_health_fix_line(item: &KnowledgeHealthFixItem) -> String {
    format!(
        "- {}{}：{}",
        item.path
            .as_deref()
            .map(|path| format!("`{path}` "))
            .unwrap_or_default(),
        item.title,
        item.detail
    )
}

fn severity_label(severity: &str) -> &'static str {
    match severity {
        "high" => "高",
        "medium" => "中",
        _ => "低",
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn knowledge_health_score(summary: &KnowledgeHealthSummary, warning_count: usize) -> u8 {
    if summary.file_count == 0 {
        return 0;
    }
    let file_count = summary.file_count as f64;
    let link_base = summary.link_count.max(summary.unresolved_link_count).max(1) as f64;
    let unresolved_ratio = summary.unresolved_link_count as f64 / link_base;
    let frontmatter_missing_ratio =
        (summary.file_count - summary.frontmatter_file_count) as f64 / file_count;
    let orphan_ratio = summary.orphan_file_count as f64 / file_count;
    let tag_missing_ratio = (summary.file_count - summary.tagged_file_count) as f64 / file_count;
    let source_missing_ratio =
        (summary.file_count - summary.source_coverage_count) as f64 / file_count;
    let warning_penalty = warning_count.min(10) as f64;
    let score = 100.0
        - unresolved_ratio * 25.0
        - frontmatter_missing_ratio * 20.0
        - orphan_ratio * 15.0
        - tag_missing_ratio * 10.0
        - source_missing_ratio * 10.0
        - warning_penalty;
    clamp_score(score)
}

fn knowledge_skill_maturity_score(
    summary: &KnowledgeHealthSummary,
    candidates: &[KnowledgeSkillCandidate],
) -> u8 {
    if summary.file_count == 0 {
        return 0;
    }
    let file_count = summary.file_count as f64;
    let formal_ratio = summary.formal_skill_file_count as f64 / file_count;
    let source_ratio = summary.source_coverage_count as f64 / file_count;
    let linked_count = summary.file_count.saturating_sub(summary.orphan_file_count);
    let linked_ratio = linked_count as f64 / file_count;
    let candidate_readiness = if candidates.is_empty() {
        0.0
    } else {
        candidates
            .iter()
            .take(HEALTH_SKILL_CANDIDATE_LIMIT)
            .map(|candidate| candidate.score as f64)
            .sum::<f64>()
            / candidates.len().min(HEALTH_SKILL_CANDIDATE_LIMIT) as f64
            / 100.0
    };
    let score = formal_ratio.min(1.0) * 35.0
        + candidate_readiness * 35.0
        + source_ratio * 20.0
        + linked_ratio * 10.0;
    clamp_score(score)
}

fn knowledge_health_issues(
    index: &MetadataLibraryIndex,
    summary: &KnowledgeHealthSummary,
) -> Vec<KnowledgeHealthIssue> {
    let mut issues = Vec::new();
    if summary.file_count == 0 {
        issues.push(KnowledgeHealthIssue {
            severity: "high".to_string(),
            title: "知识库没有可审计文件".to_string(),
            detail: "当前知识库没有 Markdown 知识文件，无法形成图谱、缓存或 skill 化候选。"
                .to_string(),
            path: None,
        });
        return issues;
    }
    if summary.unresolved_link_count > 0 {
        let top = unresolved_counts(index);
        for (source, count) in top.into_iter().take(5) {
            issues.push(KnowledgeHealthIssue {
                severity: if count > 2 { "high" } else { "medium" }.to_string(),
                title: "存在未解析链接".to_string(),
                detail: format!("{count} 条 wikilink 或 frontmatter 关系未能解析。"),
                path: Some(source),
            });
        }
    }
    push_ratio_issue(
        &mut issues,
        summary.file_count - summary.frontmatter_file_count,
        summary.file_count,
        "medium",
        "frontmatter 覆盖不足",
        "部分知识卡缺少 type/status/source_refs 等结构字段，会降低图谱语义和 skill 化判断稳定性。",
    );
    push_ratio_issue(
        &mut issues,
        summary.file_count - summary.source_coverage_count,
        summary.file_count,
        "medium",
        "来源覆盖不足",
        "缺少 source_refs/source_url/source_title 的知识难以回溯，也不适合直接沉淀为可复用 skill。",
    );
    push_ratio_issue(
        &mut issues,
        summary.file_count - summary.tagged_file_count,
        summary.file_count,
        "low",
        "标签覆盖不足",
        "缺少 tags 会削弱主题聚类、BM25 召回解释和 hot cache 排序。",
    );
    if summary.orphan_file_count > 0 {
        for file in index
            .files
            .iter()
            .filter(|file| !is_generated_knowledge_file(file))
            .filter(|file| {
                file.backlinks.is_empty() && !file.outgoing_links.iter().any(|link| link.resolved)
            })
            .take(5)
        {
            issues.push(KnowledgeHealthIssue {
                severity: "low".to_string(),
                title: "孤立知识卡".to_string(),
                detail:
                    "该文件没有已解析出链或反链，建议补 related_to/source_refs 或并入相近主题。"
                        .to_string(),
                path: Some(file.relative_path.clone()),
            });
        }
    }
    if summary.formal_skill_file_count == 0 {
        issues.push(KnowledgeHealthIssue {
            severity: "medium".to_string(),
            title: "缺少正式 skill 化沉淀".to_string(),
            detail: "未发现 type/card_type/wridian_type 标记为 skill 的知识文件，建议从高分候选提炼输入、步骤、验收和反例。".to_string(),
            path: None,
        });
    }
    issues
}

fn push_ratio_issue(
    issues: &mut Vec<KnowledgeHealthIssue>,
    missing: usize,
    total: usize,
    severity: &str,
    title: &str,
    detail: &str,
) {
    if total == 0 || missing == 0 {
        return;
    }
    let ratio = missing as f64 / total as f64;
    if ratio < 0.25 {
        return;
    }
    issues.push(KnowledgeHealthIssue {
        severity: severity.to_string(),
        title: title.to_string(),
        detail: format!("{missing}/{total} 个知识文件存在该问题。{detail}"),
        path: None,
    });
}

fn score_skill_candidates(
    index: &MetadataLibraryIndex,
    manifest: &KnowledgeCacheManifest,
) -> Vec<KnowledgeSkillCandidate> {
    let manifest_by_path = manifest
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let mut candidates = Vec::new();
    for file in &index.files {
        if is_generated_knowledge_file(file) || is_formal_skill_file(file) {
            continue;
        }
        let cached = manifest_by_path.get(file.relative_path.as_str());
        let mut score: u8 = 0;
        let mut reasons = Vec::new();
        let mut missing = Vec::new();
        if has_frontmatter(file) {
            score += 18;
            reasons.push("已有 frontmatter".to_string());
        } else {
            missing.push("frontmatter".to_string());
        }
        if !file.tags.is_empty() {
            score += 14;
            reasons.push(format!("标签 {}", file.tags.join("、")));
        } else {
            missing.push("tags".to_string());
        }
        if has_source_signal(file) {
            score += 20;
            reasons.push("有来源线索".to_string());
        } else {
            missing.push("source_refs".to_string());
        }
        if file.outgoing_links.iter().any(|link| link.resolved) {
            score += 14;
            reasons.push("有已解析出链".to_string());
        } else {
            missing.push("related_to / wikilink".to_string());
        }
        if !file.backlinks.is_empty() {
            score += 14;
            reasons.push(format!("反链 {}", file.backlinks.len()));
        } else {
            missing.push("被其他知识引用".to_string());
        }
        if has_skill_shaped_type(file) {
            score += 12;
            reasons.push("类型接近方法/检查清单".to_string());
        }
        if cached.is_some_and(|file| file.token_count >= 80) {
            score += 8;
            reasons.push("内容量足够抽象成步骤".to_string());
        }
        if score < 45 {
            continue;
        }
        missing.sort();
        missing.dedup();
        candidates.push(KnowledgeSkillCandidate {
            path: file.path.clone(),
            relative_path: file.relative_path.clone(),
            title: file.title.clone(),
            score: score.min(100),
            reasons,
            missing,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    candidates
}

fn has_frontmatter(file: &MetadataFile) -> bool {
    !file.frontmatter.is_empty()
}

fn has_source_signal(file: &MetadataFile) -> bool {
    has_any_frontmatter_value(
        file,
        &[
            "source_refs",
            "source_ref",
            "source_url",
            "source_title",
            "source_kind",
            "references",
            "references_knowledge",
            "derived_from_knowledge",
            "extracts_to",
        ],
    )
}

fn is_generated_knowledge_file(file: &MetadataFile) -> bool {
    has_frontmatter_value(file, "wridian_generated", &["true", "yes", "1"])
        || has_frontmatter_value(
            file,
            "wridian_type",
            &["knowledge_hot_cache", "knowledge_fold"],
        )
}

fn semantic_knowledge_files(index: &MetadataLibraryIndex) -> Vec<&MetadataFile> {
    index
        .files
        .iter()
        .filter(|file| !is_generated_knowledge_file(file))
        .collect()
}

fn generated_relative_paths(index: &MetadataLibraryIndex) -> HashSet<String> {
    index
        .files
        .iter()
        .filter(|file| is_generated_knowledge_file(file))
        .map(|file| file.relative_path.clone())
        .collect()
}

fn semantic_link_count(index: &MetadataLibraryIndex, generated_paths: &HashSet<String>) -> usize {
    index
        .links
        .iter()
        .filter(|link| !generated_paths.contains(&link.source_relative_path))
        .filter(|link| {
            link.target_relative_path
                .as_ref()
                .is_none_or(|target| !generated_paths.contains(target))
        })
        .count()
}

fn semantic_unresolved_link_count(
    index: &MetadataLibraryIndex,
    generated_paths: &HashSet<String>,
) -> usize {
    index
        .unresolved_links
        .iter()
        .filter(|link| !generated_paths.contains(&link.source_relative_path))
        .count()
}

fn semantic_outgoing_count(file: &MetadataFile, generated_paths: &HashSet<String>) -> usize {
    file.outgoing_links
        .iter()
        .filter(|link| link.resolved)
        .filter(|link| {
            link.target_relative_path
                .as_ref()
                .is_none_or(|target| !generated_paths.contains(target))
        })
        .count()
}

fn semantic_backlink_count(file: &MetadataFile, generated_paths: &HashSet<String>) -> usize {
    file.backlinks
        .iter()
        .filter(|backlink| !generated_paths.contains(&backlink.source_relative_path))
        .count()
}

fn semantic_unresolved_count(file: &MetadataFile) -> usize {
    file.outgoing_links
        .iter()
        .filter(|link| !link.resolved)
        .count()
}

fn is_formal_skill_file(file: &MetadataFile) -> bool {
    let path = file.relative_path.replace('\\', "/").to_lowercase();
    has_frontmatter_value(file, "type", &["skill", "knowledge_skill"])
        || has_frontmatter_value(file, "kind", &["skill", "knowledge_skill"])
        || has_frontmatter_value(file, "card_type", &["skill", "knowledge_skill"])
        || has_frontmatter_value(file, "wridian_type", &["skill", "knowledge_skill"])
        || path.starts_with("08大神蒸馏/")
        || path.contains("/skill/")
        || path.contains("/skills/")
        || path.contains("技能")
}

fn has_skill_shaped_type(file: &MetadataFile) -> bool {
    has_frontmatter_value(
        file,
        "type",
        &["knowledge_card", "knowledge_concept", "method", "checklist"],
    ) || has_frontmatter_value(file, "knowledge_kind", &["method", "checklist", "style"])
        || has_frontmatter_value(file, "concept_kind", &["method", "genre_rule", "style"])
        || has_frontmatter_value(file, "card_type", &["method", "checklist"])
        || has_frontmatter_value(file, "wridian_type", &["method", "checklist"])
}

fn has_any_frontmatter_value(file: &MetadataFile, fields: &[&str]) -> bool {
    fields.iter().any(|field| {
        file.frontmatter
            .get(*field)
            .is_some_and(|values| !values.is_empty())
    })
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

fn clamp_score(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

fn build_manifest(
    root: &Path,
    index: &MetadataLibraryIndex,
    previous: Option<&KnowledgeCacheManifest>,
) -> Result<KnowledgeCacheManifest, String> {
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    let generated_paths = generated_relative_paths(index);
    let previous_files = previous
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|file| (file.relative_path.clone(), file.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    for file in &index.files {
        if is_generated_knowledge_file(file) {
            continue;
        }
        let path = PathBuf::from(&file.path);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                push_warning(
                    &mut warnings,
                    format!(
                        "缓存跳过无法读取信息的文件：{}：{error}",
                        file.relative_path
                    ),
                );
                continue;
            }
        };
        let len = metadata.len();
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let reusable = previous_files
            .get(&file.relative_path)
            .filter(|cached| cached.len == len && cached.modified_ms == modified_ms);
        let (sha256, token_count) = if let Some(cached) = reusable {
            (cached.sha256.clone(), cached.token_count)
        } else {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(error) => {
                    push_warning(
                        &mut warnings,
                        format!(
                            "缓存跳过无法读取内容的文件：{}：{error}",
                            file.relative_path
                        ),
                    );
                    continue;
                }
            };
            (
                sha256_hex(&content),
                tokenize_mixed_vec(&format!(
                    "{}\n{}\n{}\n{}",
                    file.title,
                    file.aliases.join(" "),
                    file.tags.join(" "),
                    content
                ))
                .len(),
            )
        };
        files.push(KnowledgeCacheFile {
            relative_path: file.relative_path.clone(),
            path: file.path.clone(),
            title: file.title.clone(),
            aliases: file.aliases.clone(),
            tags: file.tags.clone(),
            modified_ms,
            len,
            sha256,
            outgoing_count: semantic_outgoing_count(file, &generated_paths),
            backlink_count: semantic_backlink_count(file, &generated_paths),
            unresolved_count: semantic_unresolved_count(file),
            token_count,
        });
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(KnowledgeCacheManifest {
        schema_version: CACHE_SCHEMA_VERSION,
        generated_at: iso_timestamp(),
        root_path: root.to_string_lossy().into_owned(),
        files,
        link_count: semantic_link_count(index, &generated_paths),
        unresolved_link_count: semantic_unresolved_link_count(index, &generated_paths),
        warnings,
    })
}

fn read_manifest(root: &Path) -> Result<KnowledgeCacheManifest, String> {
    let path = root.join(CACHE_DIR).join(MANIFEST_FILE);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("知识缓存 manifest 读取失败：{error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("知识缓存 manifest 格式损坏：{error}"))
}

fn write_manifest(root: &Path, manifest: &KnowledgeCacheManifest) -> Result<(), String> {
    let dir = root.join(CACHE_DIR);
    safe_create_knowledge_dir(root, &dir, "知识缓存目录")?;
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("知识缓存序列化失败：{error}"))?;
    safe_write_knowledge_file(
        root,
        &dir.join(MANIFEST_FILE),
        &content,
        "知识缓存 manifest",
    )
}

fn search_knowledge_bm25(
    index: &MetadataLibraryIndex,
    query: &str,
    limit: Option<usize>,
) -> Result<Vec<KnowledgeSearchHit>, String> {
    let query_tokens = tokenize_mixed_vec(query);
    if query_tokens.is_empty() {
        return Ok(Vec::new());
    }
    let query_set = query_tokens.iter().cloned().collect::<HashSet<_>>();
    let mut docs = Vec::new();
    for file in &index.files {
        if is_generated_knowledge_file(file) {
            continue;
        }
        let path = PathBuf::from(&file.path);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "知识检索文件信息读取失败（{}）：{error}",
                file.relative_path
            )
        })?;
        if metadata.len() > MAX_SEARCH_FILE_BYTES || metadata.file_type().is_symlink() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("知识检索读取失败（{}）：{error}", file.relative_path))?;
        let weighted = format!(
            "{} {} {}\n{}",
            file.title,
            file.aliases.join(" "),
            file.tags.join(" "),
            content
        );
        let tokens = tokenize_mixed_vec(&weighted);
        if tokens.is_empty() {
            continue;
        }
        docs.push(SearchDoc {
            file: file.clone(),
            content,
            len: tokens.len(),
            term_freq: term_frequency(tokens),
        });
    }
    if docs.is_empty() {
        return Ok(Vec::new());
    }
    let mut doc_freq = HashMap::<String, usize>::new();
    for token in &query_set {
        let count = docs
            .iter()
            .filter(|doc| doc.term_freq.contains_key(token))
            .count();
        if count > 0 {
            doc_freq.insert(token.clone(), count);
        }
    }
    let avg_len = docs.iter().map(|doc| doc.len as f64).sum::<f64>() / docs.len() as f64;
    let mut hits = Vec::new();
    for doc in docs {
        let score = bm25_score(
            &query_set,
            &doc.term_freq,
            doc.len,
            avg_len,
            docs_len(index),
            &doc_freq,
        );
        if score <= 0.0 {
            continue;
        }
        let matched_terms = matched_terms(&query_set, &doc.term_freq);
        hits.push(KnowledgeSearchHit {
            path: doc.file.path.clone(),
            relative_path: doc.file.relative_path.clone(),
            title: doc.file.title.clone(),
            snippet: best_snippet(&doc.content, &query_set),
            score,
            tags: doc.file.tags.clone(),
            aliases: doc.file.aliases.clone(),
            outgoing_count: doc
                .file
                .outgoing_links
                .iter()
                .filter(|link| link.resolved)
                .count(),
            backlink_count: doc.file.backlinks.len(),
            unresolved_count: doc
                .file
                .outgoing_links
                .iter()
                .filter(|link| !link.resolved)
                .count(),
            reasons: search_reasons(&matched_terms, &doc.file),
        });
    }
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    hits.truncate(limit.unwrap_or(DEFAULT_SEARCH_LIMIT).min(MAX_SEARCH_LIMIT));
    Ok(hits)
}

#[derive(Clone)]
struct SearchDoc {
    file: MetadataFile,
    content: String,
    len: usize,
    term_freq: HashMap<String, usize>,
}

fn bm25_score(
    query_set: &HashSet<String>,
    term_freq: &HashMap<String, usize>,
    doc_len: usize,
    avg_len: f64,
    doc_count: usize,
    doc_freq: &HashMap<String, usize>,
) -> f64 {
    let k1 = 1.2;
    let b = 0.75;
    let safe_avg = avg_len.max(1.0);
    query_set
        .iter()
        .map(|token| {
            let Some(tf) = term_freq.get(token).copied() else {
                return 0.0;
            };
            let df = doc_freq.get(token).copied().unwrap_or(0) as f64;
            let idf = ((doc_count as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            let tf = tf as f64;
            let denominator = tf + k1 * (1.0 - b + b * (doc_len as f64 / safe_avg));
            idf * ((tf * (k1 + 1.0)) / denominator)
        })
        .sum()
}

fn docs_len(index: &MetadataLibraryIndex) -> usize {
    index.files.len().max(1)
}

fn matched_terms(query_set: &HashSet<String>, term_freq: &HashMap<String, usize>) -> Vec<String> {
    let mut terms = query_set
        .iter()
        .filter(|token| term_freq.contains_key(*token))
        .cloned()
        .collect::<Vec<_>>();
    terms.sort_by(|left, right| {
        right
            .chars()
            .count()
            .cmp(&left.chars().count())
            .then_with(|| left.cmp(right))
    });
    terms.truncate(6);
    terms
}

fn search_reasons(matched_terms: &[String], file: &MetadataFile) -> Vec<String> {
    let mut reasons = Vec::new();
    if !matched_terms.is_empty() {
        reasons.push(format!("BM25：{}", matched_terms.join("、")));
    }
    if !file.backlinks.is_empty() {
        reasons.push(format!("反链 {}", file.backlinks.len()));
    }
    if file.outgoing_links.iter().any(|link| link.resolved) {
        reasons.push("有已解析出链".to_string());
    }
    reasons
}

fn render_hot_cache(
    manifest: &KnowledgeCacheManifest,
    index: &MetadataLibraryIndex,
    updated_at: &str,
) -> String {
    let mut recent = manifest.files.clone();
    recent.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    let mut linked = manifest.files.clone();
    linked.sort_by(|left, right| {
        right
            .backlink_count
            .cmp(&left.backlink_count)
            .then_with(|| right.outgoing_count.cmp(&left.outgoing_count))
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    let unresolved_by_file = unresolved_counts(index);
    let mut lines = vec![
        "---".to_string(),
        "wridian_generated: true".to_string(),
        "wridian_type: knowledge_hot_cache".to_string(),
        format!("updated_at: \"{updated_at}\""),
        "---".to_string(),
        String::new(),
        "# hot".to_string(),
        String::new(),
        format!(
            "- 文件：{}，关系：{}，断链：{}",
            manifest.files.len(),
            manifest.link_count,
            manifest.unresolved_link_count
        ),
        format!(
            "- manifest：`{}`",
            format!("{CACHE_DIR}/{MANIFEST_FILE}").replace('\\', "/")
        ),
        String::new(),
        "## 最近变更".to_string(),
    ];
    for file in recent.iter().take(HOT_TOP_LIMIT) {
        lines.push(format_hot_file_line(file));
    }
    lines.push(String::new());
    lines.push("## 高连接知识".to_string());
    for file in linked
        .iter()
        .filter(|file| file.backlink_count > 0 || file.outgoing_count > 0)
        .take(HOT_TOP_LIMIT)
    {
        lines.push(format_hot_file_line(file));
    }
    lines.push(String::new());
    lines.push("## 待修复断链".to_string());
    if unresolved_by_file.is_empty() {
        lines.push("- 暂无断链。".to_string());
    } else {
        for (source, count) in unresolved_by_file.iter().take(HOT_TOP_LIMIT) {
            lines.push(format!(
                "- [[{}]]：{} 条",
                trim_markdown_extension(source),
                count
            ));
        }
    }
    lines.push(String::new());
    lines.push("## skill 化候选".to_string());
    for file in skill_candidates(manifest).iter().take(HOT_TOP_LIMIT) {
        lines.push(format!(
            "- [[{}]]：标签 {}，反链 {}，可沉淀为方法/流程/检查清单。",
            trim_markdown_extension(&file.relative_path),
            if file.tags.is_empty() {
                "无".to_string()
            } else {
                file.tags.join("、")
            },
            file.backlink_count
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn render_fold(
    manifest: &KnowledgeCacheManifest,
    index: &MetadataLibraryIndex,
    created_at: &str,
) -> String {
    let top = skill_candidates(manifest);
    let unresolved = unresolved_counts(index);
    let mut tag_counts = BTreeMap::<String, usize>::new();
    for file in &manifest.files {
        for tag in &file.tags {
            *tag_counts.entry(tag.clone()).or_default() += 1;
        }
    }
    let mut tags = tag_counts.into_iter().collect::<Vec<_>>();
    tags.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let mut lines = vec![
        "---".to_string(),
        "wridian_generated: true".to_string(),
        "wridian_type: knowledge_fold".to_string(),
        format!("created_at: \"{created_at}\""),
        "---".to_string(),
        String::new(),
        "# Knowledge Fold".to_string(),
        String::new(),
        "## 范围".to_string(),
        format!(
            "- 来源：{} 个知识文件，{} 条关系，{} 条断链。",
            manifest.files.len(),
            manifest.link_count,
            manifest.unresolved_link_count
        ),
        "- 规则：仅做抽取式压缩，不新增事实。".to_string(),
        String::new(),
        "## 主题簇".to_string(),
    ];
    if tags.is_empty() {
        lines.push("- 暂无标签簇。".to_string());
    } else {
        for (tag, count) in tags.iter().take(12) {
            lines.push(format!("- #{}：{} 个文件", tag, count));
        }
    }
    lines.push(String::new());
    lines.push("## 高价值节点".to_string());
    if top.is_empty() {
        lines.push("- 暂无高连接节点。".to_string());
    } else {
        for file in top.iter().take(12) {
            lines.push(format!(
                "- [[{}]]：反链 {}，出链 {}，标签 {}",
                trim_markdown_extension(&file.relative_path),
                file.backlink_count,
                file.outgoing_count,
                if file.tags.is_empty() {
                    "无".to_string()
                } else {
                    file.tags.join("、")
                }
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 断链压缩".to_string());
    if unresolved.is_empty() {
        lines.push("- 暂无断链。".to_string());
    } else {
        for (source, count) in unresolved.iter().take(12) {
            lines.push(format!(
                "- [[{}]]：{} 条待解析链接",
                trim_markdown_extension(source),
                count
            ));
        }
    }
    lines.push(String::new());
    lines.push("## 下一轮 skill 化入口".to_string());
    for file in top.iter().take(8) {
        lines.push(format!(
            "- 从 [[{}]] 提炼可复用 skill：输入、步骤、验收、反例。",
            trim_markdown_extension(&file.relative_path)
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn unresolved_counts(index: &MetadataLibraryIndex) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::<String, usize>::new();
    let generated_paths = generated_relative_paths(index);
    for link in &index.unresolved_links {
        if generated_paths.contains(&link.source_relative_path) {
            continue;
        }
        *counts.entry(link.source_relative_path.clone()).or_default() += 1;
    }
    let mut values = counts.into_iter().collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values
}

fn skill_candidates(manifest: &KnowledgeCacheManifest) -> Vec<KnowledgeCacheFile> {
    let mut files = manifest.files.clone();
    files.sort_by(|left, right| {
        let left_score = left.backlink_count * 3 + left.outgoing_count + left.tags.len();
        let right_score = right.backlink_count * 3 + right.outgoing_count + right.tags.len();
        right_score
            .cmp(&left_score)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    files
        .into_iter()
        .filter(|file| file.backlink_count > 0 || file.outgoing_count > 1 || !file.tags.is_empty())
        .collect()
}

fn format_hot_file_line(file: &KnowledgeCacheFile) -> String {
    format!(
        "- [[{}]]：反链 {} / 出链 {} / 断链 {} / 标签 {}",
        trim_markdown_extension(&file.relative_path),
        file.backlink_count,
        file.outgoing_count,
        file.unresolved_count,
        if file.tags.is_empty() {
            "无".to_string()
        } else {
            file.tags.join("、")
        }
    )
}

fn tokenize_mixed_vec(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    for token in lower.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
        if token.chars().count() > 1 {
            tokens.push(token.to_string());
        }
    }
    let cjk = text
        .chars()
        .filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect::<Vec<_>>();
    for window in cjk.windows(2) {
        tokens.push(window.iter().collect());
    }
    tokens
}

fn term_frequency(tokens: Vec<String>) -> HashMap<String, usize> {
    let mut values = HashMap::new();
    for token in tokens {
        *values.entry(token).or_default() += 1;
    }
    values
}

fn best_snippet(content: &str, terms: &HashSet<String>) -> String {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .max_by_key(|line| {
            let lower = line.to_lowercase();
            terms
                .iter()
                .filter(|term| lower.contains(term.as_str()))
                .count()
        })
        .unwrap_or_default()
        .chars()
        .take(180)
        .collect()
}

fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn trim_markdown_extension(value: &str) -> String {
    value
        .strip_suffix(".markdown")
        .or_else(|| value.strip_suffix(".md"))
        .unwrap_or(value)
        .to_string()
}

fn compact_timestamp(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn health_report_timestamp(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() >= 19
        && chars.get(4) == Some(&'-')
        && chars.get(7) == Some(&'-')
        && chars.get(10) == Some(&'T')
        && chars.get(13) == Some(&':')
        && chars.get(16) == Some(&':')
    {
        return format!(
            "{}{}{}{}{}{}{}{}T{}{}{}{}{}{}",
            chars[0],
            chars[1],
            chars[2],
            chars[3],
            chars[5],
            chars[6],
            chars[8],
            chars[9],
            chars[11],
            chars[12],
            chars[14],
            chars[15],
            chars[17],
            chars[18]
        );
    }
    compact_timestamp(value)
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < 40 {
        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_index::read_metadata_index_for_roots;

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-knowledge-ops-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    fn index(root: &Path) -> MetadataLibraryIndex {
        read_metadata_index_for_roots(vec![("knowledge", root.to_path_buf())])
            .expect("index")
            .libraries
            .into_iter()
            .find(|library| library.library == "knowledge")
            .expect("knowledge")
    }

    #[test]
    fn manifest_records_hashes_links_and_tokens() {
        let root = temp_root("manifest");
        fs::write(root.join("A.md"), "---\ntags: [story]\n---\nA links [[B]]").expect("write A");
        fs::write(root.join("B.md"), "B body").expect("write B");

        let index = index(&root);
        let manifest = build_manifest(&root, &index, None).expect("manifest");

        assert_eq!(manifest.files.len(), 2);
        let a = manifest
            .files
            .iter()
            .find(|file| file.relative_path == "A.md")
            .expect("A");
        assert_eq!(a.outgoing_count, 1);
        assert_eq!(a.tags, vec!["story"]);
        assert!(!a.sha256.is_empty());
        assert!(a.token_count > 0);
    }

    #[test]
    fn bm25_returns_matching_knowledge_cards() {
        let root = temp_root("search");
        fs::write(
            root.join("流亡结构.md"),
            "荒原角色需要隐瞒身份并推进流亡线。",
        )
        .expect("write hit");
        fs::write(root.join("饮食.md"), "厨房菜单。").expect("write miss");
        let index = index(&root);

        let hits = search_knowledge_bm25(&index, "荒原 隐瞒身份", Some(5)).expect("search");

        assert_eq!(hits[0].relative_path, "流亡结构.md");
        assert!(hits[0].score > 0.0);
        assert!(hits[0].reasons.iter().any(|reason| reason.contains("BM25")));
    }

    #[test]
    fn hot_cache_and_fold_are_extractive() {
        let root = temp_root("hot-fold");
        fs::write(
            root.join("方法.md"),
            "---\ntags: [method]\n---\n链接 [[来源]]",
        )
        .expect("write method");
        fs::write(root.join("来源.md"), "source").expect("write source");
        let index = index(&root);
        let manifest = build_manifest(&root, &index, None).expect("manifest");

        let hot = render_hot_cache(&manifest, &index, "2026-06-12T00:00:00Z");
        let fold = render_fold(&manifest, &index, "2026-06-12T00:00:00Z");

        assert!(hot.contains("[[方法]]"));
        assert!(hot.contains("manifest：`.wridian/knowledge-manifest.json`"));
        assert!(!hot.contains("[[.wridian/knowledge-manifest.json]]"));
        assert!(fold.contains("仅做抽取式压缩"));
        assert!(fold.contains("[[方法]]"));
    }

    #[test]
    fn generated_knowledge_files_do_not_pollute_cache_search_or_health() {
        let root = temp_root("generated-noise");
        fs::write(root.join("方法.md"), "---\ntags: [method]\n---\n荒原方法")
            .expect("write method");
        fs::write(
            root.join("hot.md"),
            "---\nwridian_generated: true\nwridian_type: knowledge_hot_cache\n---\n荒原 [[不存在]]",
        )
        .expect("write generated hot");
        let index = index(&root);

        let manifest = build_manifest(&root, &index, None).expect("manifest");
        let hits = search_knowledge_bm25(&index, "不存在", Some(5)).expect("search");
        let audit = audit_knowledge_health(&index, &manifest, "now".to_string());

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].relative_path, "方法.md");
        assert!(hits.is_empty());
        assert_eq!(audit.summary.file_count, 1);
        assert_eq!(audit.summary.generated_file_count, 1);
        assert_eq!(audit.summary.unresolved_link_count, 0);
    }

    #[test]
    fn manifest_reuses_unchanged_hash_and_token_count() {
        let root = temp_root("incremental");
        fs::write(root.join("A.md"), "共同线索 [[B]]").expect("write A");
        fs::write(root.join("B.md"), "B body").expect("write B");
        let first_index = index(&root);
        let first = build_manifest(&root, &first_index, None).expect("first manifest");
        let mut previous = first.clone();
        let cached_a = previous
            .files
            .iter_mut()
            .find(|file| file.relative_path == "A.md")
            .expect("cached A");
        cached_a.sha256 = "cached-hash".to_string();
        cached_a.token_count = 777;
        let second_index = index(&root);

        let second =
            build_manifest(&root, &second_index, Some(&previous)).expect("second manifest");

        let a = second
            .files
            .iter()
            .find(|file| file.relative_path == "A.md")
            .expect("A");
        assert_eq!(a.sha256, "cached-hash");
        assert_eq!(a.token_count, 777);
        assert_eq!(a.outgoing_count, 1);
    }

    #[test]
    fn health_audit_reports_unresolved_links_and_lowers_score() {
        let root = temp_root("health-unresolved");
        fs::write(
            root.join("方法.md"),
            "---\ntype: knowledge_card\ntags: [method]\nsource_refs: [\"[[来源]]\"]\n---\n链接 [[缺失目标]]",
        )
        .expect("write method");
        fs::write(
            root.join("来源.md"),
            "---\ntype: knowledge_source\n---\nsource",
        )
        .expect("write source");
        let index = index(&root);
        let manifest = build_manifest(&root, &index, None).expect("manifest");

        let audit = audit_knowledge_health(&index, &manifest, "now".to_string());

        assert_eq!(audit.summary.unresolved_link_count, 1);
        assert!(audit.score < 100);
        assert!(audit
            .issues
            .iter()
            .any(|issue| issue.title == "存在未解析链接"));
    }

    #[test]
    fn skill_maturity_drops_without_sources_or_frontmatter() {
        let root = temp_root("health-maturity-low");
        fs::write(root.join("片段.md"), "只有正文，没有来源，也没有结构。").expect("write note");
        let index = index(&root);
        let manifest = build_manifest(&root, &index, None).expect("manifest");

        let audit = audit_knowledge_health(&index, &manifest, "now".to_string());

        assert_eq!(audit.summary.frontmatter_file_count, 0);
        assert_eq!(audit.summary.source_coverage_count, 0);
        assert!(audit.skill_maturity_score < 30);
        assert!(audit
            .issues
            .iter()
            .any(|issue| issue.title == "来源覆盖不足"));
    }

    #[test]
    fn formal_skill_files_and_ready_candidates_raise_maturity() {
        let root = temp_root("health-maturity-high");
        fs::write(
            root.join("叙事节奏.md"),
            "---\ntype: knowledge_card\nknowledge_kind: method\ntags: [节奏]\nsource_refs: [\"[[来源]]\"]\nrelated_to: [\"[[来源]]\"]\n---\n足够长的内容用于抽象成步骤。第一步识别转折，第二步压缩冗余，第三步验证钩子。",
        )
        .expect("write card");
        fs::write(
            root.join("节奏Skill.md"),
            "---\nwridian_type: knowledge_skill\ntags: [节奏]\nsource_refs: [\"[[叙事节奏]]\"]\n---\n输入、步骤、验收、反例。",
        )
        .expect("write skill");
        fs::write(
            root.join("来源.md"),
            "---\ntype: knowledge_source\n---\nsource",
        )
        .expect("write source");
        let index = index(&root);
        let manifest = build_manifest(&root, &index, None).expect("manifest");

        let audit = audit_knowledge_health(&index, &manifest, "now".to_string());

        assert_eq!(audit.summary.formal_skill_file_count, 1);
        assert!(audit.skill_maturity_score >= 50);
        assert!(audit
            .skill_candidates
            .iter()
            .any(|candidate| candidate.relative_path == "叙事节奏.md"));
    }

    #[test]
    fn low_risk_fixes_create_missing_structure_without_overwriting_existing_files() {
        let root = temp_root("health-low-risk-fix");
        fs::write(root.join("知识库使用说明.md"), "自定义说明").expect("write usage");

        let planned = collect_low_risk_knowledge_fixes(&root);

        assert!(planned
            .iter()
            .any(|item| item.id == "create-dir:00知识库治理"));
        assert!(planned
            .iter()
            .any(|item| item.id == "create-file:00知识库治理/调用记录台账.md"));
        assert!(!planned
            .iter()
            .any(|item| item.id == "create-file:知识库使用说明.md"));

        let applied = apply_low_risk_knowledge_fixes(&root).expect("apply fixes");

        assert_eq!(applied.len(), planned.len());
        for dir in REQUIRED_KNOWLEDGE_DIRS {
            assert!(root.join(dir).is_dir(), "missing {dir}");
        }
        assert_eq!(
            fs::read_to_string(root.join("知识库使用说明.md")).expect("read usage"),
            "自定义说明"
        );
        assert!(root.join("00知识库治理").join("调用记录台账.md").is_file());
        assert!(collect_low_risk_knowledge_fixes(&root).is_empty());
    }

    #[test]
    fn safe_knowledge_write_rejects_linked_parent_when_available() {
        let root = temp_root("safe-write-root");
        let outside = temp_root("safe-write-outside");
        let linked = root.join("00知识库治理");

        if create_dir_link(&outside, &linked).is_err() {
            return;
        }

        let result = safe_write_knowledge_file(
            &root,
            &linked.join("调用记录台账.md"),
            CALL_LOG_TEMPLATE,
            "调用记录台账",
        );

        assert!(result.is_err());
        assert!(!outside.join("调用记录台账.md").exists());
    }

    #[cfg(windows)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn create_dir_link(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[test]
    fn health_report_timestamp_uses_date_time_filename() {
        assert_eq!(
            health_report_timestamp("2026-06-12T19:30:01+08:00"),
            "20260612T193001"
        );
    }
}
