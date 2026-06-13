use crate::runtime::{ensure_workspace, runtime_root, workspace_config_path, wridian_data_dir};
use crate::workspace::read_workspace_file_trees;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct E2eStatus {
    enabled: bool,
    data_dir: String,
    runtime_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct E2eFixtureInput {
    reset: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct E2eMockCocreationInput {
    output: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct E2eFixtureResponse {
    data_dir: String,
    works_root: String,
    knowledge_root: String,
    first_draft_path: String,
    file_count: usize,
}

#[tauri::command]
pub(crate) fn wridian_e2e_status() -> Result<E2eStatus, String> {
    let data_dir = wridian_data_dir()?;
    Ok(E2eStatus {
        enabled: e2e_enabled(),
        runtime_path: runtime_root(&data_dir).to_string_lossy().into_owned(),
        data_dir: data_dir.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_e2e_prepare_fixture(
    input: E2eFixtureInput,
) -> Result<E2eFixtureResponse, String> {
    require_e2e_enabled()?;
    let data_dir = wridian_data_dir()?;
    let fixture_root = data_dir.join("e2e-fixture");
    if input.reset.unwrap_or(true) && fixture_root.exists() {
        fs::remove_dir_all(&fixture_root).map_err(|error| format!("E2E 夹具清理失败：{error}"))?;
    }

    ensure_workspace(&data_dir)?;
    let works_root = fixture_root.join("works");
    let knowledge_root = fixture_root.join("knowledge");
    fs::create_dir_all(&works_root).map_err(|error| format!("E2E 作品库创建失败：{error}"))?;
    fs::create_dir_all(&knowledge_root).map_err(|error| format!("E2E 知识库创建失败：{error}"))?;

    let project_dir = works_root.join("测试");
    fs::create_dir_all(&project_dir).map_err(|error| format!("E2E 项目目录创建失败：{error}"))?;
    let first_draft = project_dir.join("第1集.docx");
    write_minimal_docx(
        &first_draft,
        "# 第1集\n\n主角在雨夜进入旧车站，发现墙上写着一句警告：不要相信第二班车。\n",
    )?;
    write_file(
        &knowledge_root.join("角色卡.md"),
        "# 角色卡\n\n- 主角：谨慎但好奇。\n- 对手：掌控车站广播的人。\n",
    )?;
    write_workspace_config(&data_dir, &works_root, &knowledge_root)?;

    let file_count = read_workspace_file_trees(&data_dir)?.len();
    Ok(E2eFixtureResponse {
        data_dir: data_dir.to_string_lossy().into_owned(),
        works_root: works_root.to_string_lossy().into_owned(),
        knowledge_root: knowledge_root.to_string_lossy().into_owned(),
        first_draft_path: first_draft.to_string_lossy().into_owned(),
        file_count,
    })
}

#[tauri::command]
pub(crate) fn wridian_e2e_set_next_cocreation(input: E2eMockCocreationInput) -> Result<(), String> {
    require_e2e_enabled()?;
    let output = input.output.trim();
    if output.is_empty() {
        return Err("E2E mock 回复不能为空。".to_string());
    }
    let data_dir = wridian_data_dir()?;
    fs::create_dir_all(runtime_root(&data_dir))
        .map_err(|error| format!("E2E 运行目录创建失败：{error}"))?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(e2e_next_cocreation_path(&data_dir))
        .and_then(|mut file| {
            writeln!(
                file,
                "{}",
                serde_json::to_string(output).unwrap_or_default()
            )
        })
        .map_err(|error| format!("E2E mock 回复写入失败：{error}"))
}

pub(crate) fn take_next_cocreation_output(data_dir: &Path) -> Result<Option<String>, String> {
    if !e2e_enabled() {
        return Ok(None);
    }
    let path = e2e_next_cocreation_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).map_err(|error| format!("E2E mock 回复读取失败：{error}"))?;
    let mut lines = content.lines();
    let Some(first_line) = lines.find(|line| !line.trim().is_empty()) else {
        let _ = fs::remove_file(&path);
        return Ok(None);
    };
    let output: String =
        serde_json::from_str(first_line).unwrap_or_else(|_| first_line.to_string());
    let remaining = lines.collect::<Vec<_>>().join("\n");
    if remaining.trim().is_empty() {
        let _ = fs::remove_file(&path);
    } else {
        fs::write(&path, remaining).map_err(|error| format!("E2E mock 队列更新失败：{error}"))?;
    }
    Ok(Some(output))
}

pub(crate) fn has_queued_cocreation_output(data_dir: &Path) -> bool {
    e2e_enabled() && e2e_next_cocreation_path(data_dir).exists()
}

fn e2e_enabled() -> bool {
    std::env::var("WRIDIAN_E2E")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn require_e2e_enabled() -> Result<(), String> {
    if e2e_enabled() {
        Ok(())
    } else {
        Err("E2E 控制入口未启用。请设置 WRIDIAN_E2E=1 后启动 Wridian。".to_string())
    }
}

fn e2e_next_cocreation_path(data_dir: &Path) -> PathBuf {
    runtime_root(data_dir).join("e2e-next-cocreation.json")
}

fn write_workspace_config(
    data_dir: &Path,
    works_root: &Path,
    knowledge_root: &Path,
) -> Result<(), String> {
    fs::create_dir_all(runtime_root(data_dir))
        .map_err(|error| format!("E2E 运行目录创建失败：{error}"))?;
    let content = serde_json::to_string_pretty(&json!({
        "schemaVersion": 1,
        "activeWorkRoot": works_root.to_string_lossy(),
        "knowledgeRoot": knowledge_root.to_string_lossy(),
    }))
    .map_err(|error| error.to_string())?;
    fs::write(workspace_config_path(data_dir), content)
        .map_err(|error| format!("E2E 工作区配置写入失败：{error}"))
}

fn write_file(path: &PathBuf, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("E2E 目录创建失败：{error}"))?;
    }
    fs::write(path, content).map_err(|error| format!("E2E 文件写入失败：{error}"))
}

fn write_minimal_docx(path: &PathBuf, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("E2E 目录创建失败：{error}"))?;
    }
    let document_xml = minimal_docx_document_xml(content);
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("[Content_Types].xml", options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#)
            .map_err(|error| error.to_string())?;
        writer
            .start_file("word/document.xml", options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(document_xml.as_bytes())
            .map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    fs::write(path, output.into_inner()).map_err(|error| format!("E2E DOCX 写入失败：{error}"))
}

fn minimal_docx_document_xml(content: &str) -> String {
    let paragraphs = if content.is_empty() {
        vec![String::new()]
    } else {
        content
            .split('\n')
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };
    let body = paragraphs
        .iter()
        .map(|paragraph| {
            format!(
                "<w:p><w:r><w:t>{}</w:t></w:r></w:p>",
                encode_xml_text(paragraph)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    )
}

fn encode_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e2e_status_is_safe_when_disabled() {
        std::env::remove_var("WRIDIAN_E2E");
        let status = wridian_e2e_status().expect("status");
        assert!(!status.enabled);
    }

    #[test]
    fn e2e_fixture_requires_env_gate() {
        std::env::remove_var("WRIDIAN_E2E");
        assert!(wridian_e2e_prepare_fixture(E2eFixtureInput { reset: Some(true) }).is_err());
    }
}
