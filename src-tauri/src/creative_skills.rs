use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{path::BaseDirectory, AppHandle, Manager};

const SKILLS_RESOURCE_ROOT: &str = "resources/skills";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreativeSkillSources {
    knowledge_ops: CreativeSkillSource,
    work_decompose: CreativeSkillSource,
    knowledge_card: CreativeSkillSource,
    author_distill: CreativeSkillSource,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreativeSkillSource {
    available: bool,
    source: &'static str,
    label: &'static str,
    path: Option<String>,
}

#[tauri::command]
pub(crate) fn wridian_get_creative_skill_sources(app: AppHandle) -> CreativeSkillSources {
    let resource_root = app
        .path()
        .resolve(SKILLS_RESOURCE_ROOT, BaseDirectory::Resource)
        .ok();
    creative_skill_sources_from_resource_root(resource_root.as_deref())
}

fn creative_skill_sources_from_resource_root(resource_root: Option<&Path>) -> CreativeSkillSources {
    CreativeSkillSources {
        knowledge_ops: builtin_skill_source(
            resource_root,
            "知识库运维",
            Path::new("zhishiku-skill").join("SKILL.md"),
        ),
        work_decompose: builtin_skill_source(
            resource_root,
            "作品拆解",
            Path::new("zhishiku-skill")
                .join("references")
                .join("embedded-skills")
                .join("chaijie-skill.md"),
        ),
        knowledge_card: builtin_skill_source(
            resource_root,
            "知识卡提炼",
            Path::new("zhishiku-skill")
                .join("references")
                .join("embedded-skills")
                .join("tilian-skill.md"),
        ),
        author_distill: builtin_skill_source(
            resource_root,
            "大神蒸馏",
            Path::new("zhishiku-skill")
                .join("references")
                .join("embedded-skills")
                .join("zhengliu-skill.md"),
        ),
    }
}

fn builtin_skill_source(
    resource_root: Option<&Path>,
    label: &'static str,
    relative_path: PathBuf,
) -> CreativeSkillSource {
    let path = resource_root.map(|root| root.join(relative_path));
    CreativeSkillSource {
        available: path.as_ref().is_some_and(|path| path.is_file()),
        source: "builtin-resource",
        label,
        path: path.map(|path| path.to_string_lossy().into_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_skill_sources_are_builtin_resources_and_distributable() {
        let resource_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("skills");
        let sources = creative_skill_sources_from_resource_root(Some(&resource_root));

        assert!(sources.knowledge_ops.available);
        assert!(sources.work_decompose.available);
        assert!(sources.knowledge_card.available);
        assert!(sources.author_distill.available);
        assert_eq!(sources.knowledge_ops.source, "builtin-resource");
        assert!(
            sources
                .knowledge_ops
                .path
                .as_deref()
                .unwrap_or_default()
                .ends_with("resources\\skills\\zhishiku-skill\\SKILL.md")
                || sources
                    .knowledge_ops
                    .path
                    .as_deref()
                    .unwrap_or_default()
                    .ends_with("resources/skills/zhishiku-skill/SKILL.md")
        );
    }

    #[test]
    fn creative_skill_sources_report_missing_resources() {
        let sources = creative_skill_sources_from_resource_root(Some(Path::new("missing-skills")));

        assert!(!sources.knowledge_ops.available);
        assert!(!sources.work_decompose.available);
        assert!(!sources.knowledge_card.available);
        assert!(!sources.author_distill.available);
    }
}
