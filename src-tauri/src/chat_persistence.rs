use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatTranscriptInput {
    session_id: String,
    title: String,
    source_path: String,
    messages: Vec<ChatTranscriptMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatTranscriptMessage {
    role: String,
    text: String,
    selected_text: Option<String>,
    context_pills: Option<Vec<ChatContextPill>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatContextPill {
    label: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatTranscriptResponse {
    path: String,
}

#[tauri::command]
pub(crate) fn wridian_save_chat_transcript(
    input: SaveChatTranscriptInput,
) -> Result<SaveChatTranscriptResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let chat_dir = runtime_root(&data_dir).join("chat");
    fs::create_dir_all(&chat_dir).map_err(|error| format!("聊天记录目录创建失败：{error}"))?;

    let file_name = format!("{}.md", sanitize_file_name(&input.session_id));
    let path = chat_dir.join(file_name);
    fs::write(&path, render_chat_transcript(&input))
        .map_err(|error| format!("聊天记录写入失败：{error}"))?;

    Ok(SaveChatTranscriptResponse {
        path: path.to_string_lossy().into_owned(),
    })
}

fn render_chat_transcript(input: &SaveChatTranscriptInput) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str("type: wridian-chat\n");
    content.push_str(&format!("session: {}\n", escape_yaml(&input.session_id)));
    content.push_str(&format!("title: {}\n", escape_yaml(&input.title)));
    content.push_str(&format!("source: {}\n", escape_yaml(&input.source_path)));
    content.push_str("---\n\n");
    content.push_str(&format!(
        "# {}\n\n",
        escape_markdown_heading(if input.title.trim().is_empty() {
            "Wridian 对话"
        } else {
            input.title.trim()
        })
    ));

    for message in &input.messages {
        let heading = if message.role == "assistant" {
            "Wridian"
        } else {
            "用户"
        };
        content.push_str(&format!("## {heading}\n\n"));
        if let Some(pills) = &message.context_pills {
            if !pills.is_empty() {
                content.push_str("### 上下文\n\n");
                for pill in pills {
                    content.push_str(&format!(
                        "#### {}\n\n{}\n\n",
                        escape_markdown_heading(pill.label.trim()),
                        fenced_block(&pill.value)
                    ));
                }
            }
        } else if let Some(selected_text) = &message.selected_text {
            if !selected_text.trim().is_empty() {
                content.push_str("### 上下文\n\n");
                content.push_str(&fenced_block(selected_text));
                content.push_str("\n\n");
            }
        }
        content.push_str(&fenced_block(&message.text));
        content.push_str("\n\n");
    }

    content
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches(['.', ' ']).trim();
    if trimmed.is_empty() {
        "chat".to_string()
    } else {
        trimmed.to_string()
    }
}

fn escape_yaml(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn escape_markdown_heading(value: &str) -> String {
    let escaped = value
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]");
    if escaped.is_empty() {
        "未命名".to_string()
    } else {
        escaped
    }
}

fn fenced_block(value: &str) -> String {
    let mut fence = "```";
    while value.contains(fence) {
        fence = match fence.len() {
            3 => "````",
            4 => "`````",
            _ => "``````",
        };
    }
    format!("{fence}text\n{}\n{fence}", value.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_chat_transcript_fences_untrusted_content() {
        let transcript = render_chat_transcript(&SaveChatTranscriptInput {
            session_id: "session-1".to_string(),
            title: "# hacked".to_string(),
            source_path: "source.md".to_string(),
            messages: vec![ChatTranscriptMessage {
                role: "user".to_string(),
                text: "---\ntype: injected\n```".to_string(),
                selected_text: Some("## not heading".to_string()),
                context_pills: Some(vec![ChatContextPill {
                    label: "# pill".to_string(),
                    value: "- fake list".to_string(),
                }]),
            }],
        });

        assert!(transcript.contains("# \\# hacked"));
        assert!(transcript.contains("#### \\# pill"));
        assert!(transcript.contains("````text\n---\ntype: injected\n```\n````"));
        assert!(transcript.contains("```text\n- fake list\n```"));
    }
}
