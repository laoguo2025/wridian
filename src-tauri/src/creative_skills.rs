use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreativeSkillSources {
    knowledge_ops: CreativeSkillSource,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreativeSkillSource {
    available: bool,
    path: Option<String>,
}

#[tauri::command]
pub(crate) fn wridian_get_creative_skill_sources() -> CreativeSkillSources {
    let knowledge_ops_path = find_zhishiku_skill();
    CreativeSkillSources {
        knowledge_ops: CreativeSkillSource {
            available: knowledge_ops_path.is_some(),
            path: knowledge_ops_path.map(|path| path.to_string_lossy().into_owned()),
        },
    }
}

fn find_zhishiku_skill() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(desktop) = dirs::desktop_dir() {
        candidates.push(desktop.join("zhishiku-skill").join("SKILL.md"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(
            home.join(".codex")
                .join("skills")
                .join("zhishiku-skill")
                .join("SKILL.md"),
        );
        candidates.push(
            home.join(".agents")
                .join("skills")
                .join("zhishiku-skill")
                .join("SKILL.md"),
        );
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_skill_sources_returns_stable_knowledge_ops_shape() {
        let sources = wridian_get_creative_skill_sources();

        if sources.knowledge_ops.available {
            assert!(sources
                .knowledge_ops
                .path
                .as_deref()
                .unwrap_or("")
                .ends_with("SKILL.md"));
        } else {
            assert!(sources.knowledge_ops.path.is_none());
        }
    }
}
