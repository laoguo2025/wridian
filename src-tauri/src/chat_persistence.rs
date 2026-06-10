use crate::file_lock::with_file_write_lock;
use crate::runtime::{ensure_workspace, iso_timestamp, runtime_root, wridian_data_dir};
use crate::workspace::resolved_knowledge_root;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatKnowledgeCardInput {
    session_id: String,
    source_path: String,
    title: String,
    card_title: Option<String>,
    user_message: Option<String>,
    assistant_message: String,
    context_pills: Option<Vec<ChatContextPill>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatTranscriptResponse {
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatKnowledgeCardResponse {
    path: String,
    title: String,
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
    with_file_write_lock(&data_dir, &path, || {
        fs::write(&path, render_chat_transcript(&input))
            .map_err(|error| format!("聊天记录写入失败：{error}"))
    })?;

    Ok(SaveChatTranscriptResponse {
        path: path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_save_chat_knowledge_card(
    input: SaveChatKnowledgeCardInput,
) -> Result<SaveChatKnowledgeCardResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let knowledge_root = resolved_knowledge_root(&data_dir)?;
    let card_dir = knowledge_root.join("00知识库治理").join("对话沉淀");
    fs::create_dir_all(&card_dir).map_err(|error| format!("知识卡目录创建失败：{error}"))?;

    let title = normalize_card_title(
        input
            .card_title
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                first_non_empty_line(&input.assistant_message).unwrap_or("Wridian 对话沉淀")
            }),
    );
    let slug = format!("{}-{}", chrono_like_date(), sanitize_file_name(&title));
    let path = unique_markdown_path(&card_dir, &slug);
    let content = render_chat_knowledge_card(&input, &title);
    with_file_write_lock(&data_dir, &path, || {
        fs::write(&path, content).map_err(|error| format!("知识卡写入失败：{error}"))
    })?;

    Ok(SaveChatKnowledgeCardResponse {
        path: path.to_string_lossy().into_owned(),
        title,
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

fn render_chat_knowledge_card(input: &SaveChatKnowledgeCardInput, title: &str) -> String {
    let created_at = iso_timestamp();
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str("type: knowledge_card\n");
    content.push_str("wridian_type: conversation_distillation\n");
    content.push_str("status: draft\n");
    content.push_str("review_status: 待核查\n");
    content.push_str("source: wridian-chat\n");
    content.push_str(&format!(
        "chat_session: {}\n",
        escape_yaml(&input.session_id)
    ));
    content.push_str(&format!(
        "writing_context: {}\n",
        escape_yaml(&input.source_path)
    ));
    content.push_str(&format!("created_at: {}\n", escape_yaml(&created_at)));
    content.push_str(&format!("updated_at: {}\n", escape_yaml(&created_at)));
    content.push_str("---\n\n");
    content.push_str(&format!("# {}\n\n", escape_markdown_heading(title)));
    content.push_str(
        "> 待核查草稿。建议后续用 zhishiku-skill 提炼、蒸馏或体检后再转为正式知识卡。\n\n",
    );
    content.push_str("## 可沉淀内容\n\n");
    content.push_str(input.assistant_message.trim());
    content.push_str("\n\n## 来自对话\n\n");
    content.push_str(&format!(
        "- 对话：{}\n- 写作现场：{}\n- 当前标题：{}\n\n",
        input.session_id.trim(),
        input.source_path.trim(),
        input.title.trim()
    ));
    if let Some(user_message) = input
        .user_message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        content.push_str("### 用户提问\n\n");
        content.push_str(&fenced_block(user_message));
        content.push_str("\n\n");
    }
    if let Some(pills) = &input.context_pills {
        if !pills.is_empty() {
            content.push_str("### 写作上下文\n\n");
            for pill in pills {
                content.push_str(&format!(
                    "#### {}\n\n{}\n\n",
                    escape_markdown_heading(pill.label.trim()),
                    fenced_block(&pill.value)
                ));
            }
        }
    }
    content.push_str("### Wridian 回复原文\n\n");
    content.push_str(&fenced_block(&input.assistant_message));
    content.push('\n');
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

fn normalize_card_title(value: &str) -> String {
    let title = value
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('#')
        .trim()
        .chars()
        .take(48)
        .collect::<String>();
    if title.is_empty() {
        "Wridian 对话沉淀".to_string()
    } else {
        title
    }
}

fn first_non_empty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

fn unique_markdown_path(folder: &Path, slug: &str) -> PathBuf {
    let mut path = folder.join(format!("{slug}.md"));
    if !path.exists() {
        return path;
    }
    for index in 2..100 {
        path = folder.join(format!("{slug}-{index}.md"));
        if !path.exists() {
            return path;
        }
    }
    folder.join(format!("{slug}-{}.md", chrono_like_timestamp()))
}

fn chrono_like_date() -> String {
    iso_timestamp().chars().take(10).collect()
}

fn chrono_like_timestamp() -> String {
    iso_timestamp()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
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

    #[test]
    fn render_chat_knowledge_card_marks_draft_for_review() {
        let card = render_chat_knowledge_card(
            &SaveChatKnowledgeCardInput {
                session_id: "session-1".to_string(),
                source_path: "chapter.md".to_string(),
                title: "第一章".to_string(),
                card_title: Some("反转技巧".to_string()),
                user_message: Some("怎么强化反转？".to_string()),
                assistant_message: "先建立误导，再回收伏笔。".to_string(),
                context_pills: Some(vec![ChatContextPill {
                    label: "选区".to_string(),
                    value: "她没有说实话。".to_string(),
                }]),
            },
            "反转技巧",
        );

        assert!(card.contains("type: knowledge_card"));
        assert!(card.contains("status: draft"));
        assert!(card.contains("review_status: 待核查"));
        assert!(card.contains("source: wridian-chat"));
        assert!(card.contains("## 可沉淀内容"));
        assert!(card.contains("### 用户提问"));
    }
}
