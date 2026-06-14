use crate::atomic_write::atomic_write_text;
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CHAT_SCOPE_KEY: &str = "__default__";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatTranscriptInput {
    session_id: String,
    parent_session_id: Option<String>,
    forked_from_message_id: Option<String>,
    project_id: Option<String>,
    title: String,
    source_path: String,
    active_context: Option<Value>,
    messages: Vec<ChatTranscriptMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadChatContinuityInput {
    project_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChatTranscriptMessage {
    id: String,
    role: String,
    text: String,
    created_at: Option<u128>,
    selected_text: Option<String>,
    context_pills: Option<Vec<ChatContextPill>>,
    context_load_status: Option<Vec<ChatContextLoadStatus>>,
    file_operations: Option<Vec<ChatFileOperation>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChatFileOperation {
    action: String,
    library: String,
    message: String,
    ok: bool,
    path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChatContextPill {
    id: Option<String>,
    kind: Option<String>,
    label: String,
    value: String,
    source_path: Option<String>,
    relative_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChatContextLoadStatus {
    key: String,
    label: String,
    loaded: bool,
    item_count: usize,
    included_chars: usize,
    budget_chars: usize,
    truncated: bool,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveChatTranscriptResponse {
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadChatContinuityResponse {
    session_id: String,
    parent_session_id: Option<String>,
    forked_from_message_id: Option<String>,
    title: String,
    source_path: String,
    active_context: Option<Value>,
    messages: Vec<ChatTranscriptMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSessionIndex {
    #[serde(default = "chat_session_index_schema_version")]
    schema_version: u8,
    #[serde(default)]
    active_session_id: String,
    #[serde(default)]
    active_sessions_by_project: BTreeMap<String, String>,
    #[serde(default)]
    updated_at: u128,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSessionState {
    schema_version: u8,
    session_id: String,
    parent_session_id: Option<String>,
    forked_from_message_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    current_node_id: Option<String>,
    title: String,
    source_path: String,
    active_context: Option<Value>,
    compact_summary: String,
    updated_at: u128,
    nodes: Vec<ChatSessionNode>,
    messages: Vec<ChatTranscriptMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSessionNode {
    id: String,
    parent_id: Option<String>,
    role: String,
    preview: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatSessionHistoryEvent<'a> {
    schema_version: u8,
    event: &'a str,
    session_id: &'a str,
    parent_session_id: Option<&'a str>,
    forked_from_message_id: Option<&'a str>,
    project_id: Option<&'a str>,
    current_node_id: Option<&'a str>,
    title: &'a str,
    source_path: &'a str,
    active_context: Option<&'a Value>,
    compact_summary: &'a str,
    message_count: usize,
    updated_at: u128,
}

fn chat_session_index_schema_version() -> u8 {
    2
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
    atomic_write_text(&path, &render_chat_transcript(&input))
        .map_err(|error| format!("聊天记录写入失败：{error}"))?;
    write_chat_session_state(&chat_dir, &input)?;
    write_active_context_files(&data_dir, &chat_dir, input.active_context.as_ref())?;

    Ok(SaveChatTranscriptResponse {
        path: path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub(crate) fn wridian_load_chat_continuity(
    input: Option<LoadChatContinuityInput>,
) -> Result<LoadChatContinuityResponse, String> {
    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let chat_dir = runtime_root(&data_dir).join("chat");
    load_chat_continuity_from_chat_dir(
        &chat_dir,
        input
            .as_ref()
            .and_then(|input| clean_optional(input.project_id.as_deref()))
            .as_deref(),
    )
}

fn load_chat_continuity_from_chat_dir(
    chat_dir: &Path,
    project_id: Option<&str>,
) -> Result<LoadChatContinuityResponse, String> {
    let index_path = chat_dir.join("session-index.json");
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Ok(empty_chat_continuity());
    };
    let index: ChatSessionIndex = serde_json::from_str(&index_content)
        .map_err(|error| format!("对话索引读取失败：{error}"))?;
    let scope_key = chat_scope_key(project_id);
    let session_id = index
        .active_sessions_by_project
        .get(&scope_key)
        .filter(|session_id| !session_id.trim().is_empty())
        .cloned()
        .or_else(|| {
            if scope_key == DEFAULT_CHAT_SCOPE_KEY
                && index.schema_version < chat_session_index_schema_version()
                && !index.active_session_id.trim().is_empty()
            {
                Some(index.active_session_id)
            } else {
                None
            }
        });
    let Some(session_id) = session_id else {
        return Ok(empty_chat_continuity());
    };
    let session_path = chat_dir
        .join("sessions")
        .join(format!("{}.json", sanitize_file_name(&session_id)));
    let Ok(session_content) = fs::read_to_string(session_path) else {
        return Ok(empty_chat_continuity());
    };
    let state: ChatSessionState = serde_json::from_str(&session_content)
        .map_err(|error| format!("对话续接读取失败：{error}"))?;
    Ok(LoadChatContinuityResponse {
        session_id: state.session_id,
        parent_session_id: state.parent_session_id,
        forked_from_message_id: state.forked_from_message_id,
        title: state.title,
        source_path: state.source_path,
        active_context: state.active_context,
        messages: state.messages,
    })
}

fn empty_chat_continuity() -> LoadChatContinuityResponse {
    LoadChatContinuityResponse {
        session_id: String::new(),
        parent_session_id: None,
        forked_from_message_id: None,
        title: String::new(),
        source_path: String::new(),
        active_context: None,
        messages: Vec::new(),
    }
}

fn write_chat_session_state(
    chat_dir: &Path,
    input: &SaveChatTranscriptInput,
) -> Result<(), String> {
    let updated_at = timestamp_millis();
    let current_node_id = input.messages.last().map(|message| message.id.clone());
    let compact_summary = compact_summary_from_active_context(input.active_context.as_ref());
    let state = ChatSessionState {
        schema_version: 1,
        session_id: input.session_id.clone(),
        parent_session_id: clean_optional(input.parent_session_id.as_deref()),
        forked_from_message_id: clean_optional(input.forked_from_message_id.as_deref()),
        project_id: clean_optional(input.project_id.as_deref()),
        current_node_id,
        title: input.title.clone(),
        source_path: input.source_path.clone(),
        active_context: input.active_context.clone(),
        compact_summary,
        updated_at,
        nodes: build_session_nodes(&input.messages),
        messages: input.messages.clone(),
    };

    let sessions_dir = chat_dir.join("sessions");
    fs::create_dir_all(&sessions_dir).map_err(|error| format!("对话树目录创建失败：{error}"))?;
    let session_path = sessions_dir.join(format!("{}.json", sanitize_file_name(&input.session_id)));
    atomic_write_text(
        &session_path,
        &serde_json::to_string_pretty(&state)
            .map_err(|error| format!("对话树序列化失败：{error}"))?,
    )
    .map_err(|error| format!("对话树写入失败：{error}"))?;

    let index_path = chat_dir.join("session-index.json");
    let mut index = fs::read_to_string(&index_path)
        .ok()
        .and_then(|content| serde_json::from_str::<ChatSessionIndex>(&content).ok())
        .unwrap_or_else(|| ChatSessionIndex {
            schema_version: chat_session_index_schema_version(),
            active_session_id: String::new(),
            active_sessions_by_project: BTreeMap::new(),
            updated_at: 0,
        });
    let scope_key = chat_scope_key(input.project_id.as_deref());
    if scope_key == DEFAULT_CHAT_SCOPE_KEY {
        index.active_session_id = input.session_id.clone();
    }
    index
        .active_sessions_by_project
        .insert(scope_key, input.session_id.clone());
    index.schema_version = chat_session_index_schema_version();
    index.updated_at = updated_at;
    atomic_write_text(
        &index_path,
        &serde_json::to_string_pretty(&index)
            .map_err(|error| format!("对话索引序列化失败：{error}"))?,
    )
    .map_err(|error| format!("对话索引写入失败：{error}"))?;

    append_chat_history_event(chat_dir, input, &state)?;
    Ok(())
}

fn append_chat_history_event(
    chat_dir: &Path,
    input: &SaveChatTranscriptInput,
    state: &ChatSessionState,
) -> Result<(), String> {
    let history_dir = chat_dir.join("session-history");
    fs::create_dir_all(&history_dir).map_err(|error| format!("对话历史目录创建失败：{error}"))?;
    let history_path = history_dir.join(format!("{}.jsonl", sanitize_file_name(&input.session_id)));
    let event = ChatSessionHistoryEvent {
        schema_version: 1,
        event: "snapshot",
        session_id: &input.session_id,
        parent_session_id: state.parent_session_id.as_deref(),
        forked_from_message_id: state.forked_from_message_id.as_deref(),
        project_id: state.project_id.as_deref(),
        current_node_id: state.current_node_id.as_deref(),
        title: &input.title,
        source_path: &input.source_path,
        active_context: input.active_context.as_ref(),
        compact_summary: &state.compact_summary,
        message_count: input.messages.len(),
        updated_at: state.updated_at,
    };
    let line =
        serde_json::to_string(&event).map_err(|error| format!("对话历史序列化失败：{error}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)
        .map_err(|error| format!("对话历史写入失败：{error}"))?;
    writeln!(file, "{line}").map_err(|error| format!("对话历史写入失败：{error}"))?;
    Ok(())
}

fn write_active_context_files(
    data_dir: &Path,
    chat_dir: &Path,
    active_context: Option<&Value>,
) -> Result<(), String> {
    let Some(active_context) = active_context else {
        return Ok(());
    };
    atomic_write_text(
        &runtime_root(data_dir).join("active-context.json"),
        &serde_json::to_string_pretty(active_context)
            .map_err(|error| format!("当前现场序列化失败：{error}"))?,
    )
    .map_err(|error| format!("当前现场写入失败：{error}"))?;
    atomic_write_text(
        &chat_dir.join("compact-summary.md"),
        &compact_summary_from_active_context(Some(active_context)),
    )
    .map_err(|error| format!("创作交接卡写入失败：{error}"))?;
    Ok(())
}

fn build_session_nodes(messages: &[ChatTranscriptMessage]) -> Vec<ChatSessionNode> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| ChatSessionNode {
            id: message.id.clone(),
            parent_id: index
                .checked_sub(1)
                .and_then(|previous| messages.get(previous))
                .map(|previous| previous.id.clone()),
            role: message.role.clone(),
            preview: compact_plain_text(&message.text, 120),
        })
        .collect()
}

fn compact_summary_from_active_context(active_context: Option<&Value>) -> String {
    let Some(context) = active_context else {
        return "# 创作交接卡\n\n暂无当前现场。\n".to_string();
    };
    context
        .get("compactSummary")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            let last_intent = context
                .get("lastUserIntent")
                .and_then(Value::as_str)
                .unwrap_or("暂无");
            let last_judgment = context
                .get("lastJudgment")
                .and_then(Value::as_str)
                .unwrap_or("暂无");
            format!("# 创作交接卡\n\n- 上次用户意图：{last_intent}\n- 上次判断：{last_judgment}\n")
        })
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn chat_scope_key(project_id: Option<&str>) -> String {
    clean_optional(project_id).unwrap_or_else(|| DEFAULT_CHAT_SCOPE_KEY.to_string())
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn render_chat_transcript(input: &SaveChatTranscriptInput) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str("type: wridian-chat\n");
    content.push_str(&format!("session: {}\n", escape_yaml(&input.session_id)));
    if let Some(project_id) = clean_optional(input.project_id.as_deref()) {
        content.push_str(&format!("project: {}\n", escape_yaml(&project_id)));
    }
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
        if let Some(created_at) = message.created_at {
            content.push_str(&format!("time: {created_at}\n\n"));
        }
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
        if let Some(operations) = &message.file_operations {
            if !operations.is_empty() {
                content.push_str("### 文件操作\n\n");
                for operation in operations {
                    let status = if operation.ok {
                        "已执行"
                    } else {
                        "未执行"
                    };
                    content.push_str(&format!(
                        "- {} {} / {}：{}\n",
                        status,
                        operation.library.trim(),
                        operation.path.trim(),
                        operation.message.trim()
                    ));
                }
                content.push('\n');
            }
        }
    }

    content
}

fn compact_plain_text(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_chars).collect()
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
            parent_session_id: None,
            forked_from_message_id: None,
            project_id: None,
            title: "# hacked".to_string(),
            source_path: "source.md".to_string(),
            active_context: None,
            messages: vec![ChatTranscriptMessage {
                id: "user-1".to_string(),
                role: "user".to_string(),
                text: "---\ntype: injected\n```".to_string(),
                created_at: Some(1),
                selected_text: Some("## not heading".to_string()),
                context_pills: Some(vec![ChatContextPill {
                    id: None,
                    kind: None,
                    label: "# pill".to_string(),
                    value: "- fake list".to_string(),
                    source_path: None,
                    relative_path: None,
                }]),
                context_load_status: None,
                file_operations: Some(vec![ChatFileOperation {
                    action: "writeFile".to_string(),
                    library: "works".to_string(),
                    message: "已写入 测试/第2集.md".to_string(),
                    ok: true,
                    path: "测试/第2集.md".to_string(),
                }]),
            }],
        });

        assert!(transcript.contains("# \\# hacked"));
        assert!(transcript.contains("#### \\# pill"));
        assert!(transcript.contains("````text\n---\ntype: injected\n```\n````"));
        assert!(transcript.contains("```text\n- fake list\n```"));
        assert!(transcript.contains("### 文件操作"));
        assert!(transcript.contains("已执行 works / 测试/第2集.md"));
    }

    #[test]
    fn chat_continuity_keeps_project_sessions_separate() {
        let chat_dir = test_chat_dir("scoped");
        fs::create_dir_all(&chat_dir).expect("create chat dir");

        write_chat_session_state(
            &chat_dir,
            &test_input("default-session", None, "default message"),
        )
        .expect("write default session");
        write_chat_session_state(
            &chat_dir,
            &test_input("project-a-session", Some("project-a"), "project message"),
        )
        .expect("write project session");

        let default_continuity =
            load_chat_continuity_from_chat_dir(&chat_dir, None).expect("load default");
        let project_continuity =
            load_chat_continuity_from_chat_dir(&chat_dir, Some("project-a")).expect("load project");
        let missing_continuity =
            load_chat_continuity_from_chat_dir(&chat_dir, Some("project-b")).expect("load missing");

        assert_eq!(default_continuity.session_id, "default-session");
        assert_eq!(default_continuity.messages[0].text, "default message");
        assert_eq!(project_continuity.session_id, "project-a-session");
        assert_eq!(project_continuity.messages[0].text, "project message");
        assert!(missing_continuity.session_id.is_empty());

        let _ = fs::remove_dir_all(chat_dir.parent().expect("test root parent"));
    }

    #[test]
    fn project_only_chat_does_not_become_default_chat() {
        let chat_dir = test_chat_dir("project-only");
        fs::create_dir_all(&chat_dir).expect("create chat dir");

        write_chat_session_state(
            &chat_dir,
            &test_input("project-a-session", Some("project-a"), "project message"),
        )
        .expect("write project session");

        let default_continuity =
            load_chat_continuity_from_chat_dir(&chat_dir, None).expect("load default");
        let project_continuity =
            load_chat_continuity_from_chat_dir(&chat_dir, Some("project-a")).expect("load project");

        assert!(default_continuity.session_id.is_empty());
        assert_eq!(project_continuity.session_id, "project-a-session");

        let _ = fs::remove_dir_all(chat_dir.parent().expect("test root parent"));
    }

    #[test]
    fn chat_continuity_loads_legacy_global_index() {
        let chat_dir = test_chat_dir("legacy");
        let sessions_dir = chat_dir.join("sessions");
        fs::create_dir_all(&sessions_dir).expect("create sessions dir");
        let session = ChatSessionState {
            schema_version: 1,
            session_id: "legacy-session".to_string(),
            parent_session_id: None,
            forked_from_message_id: None,
            project_id: None,
            current_node_id: Some("message-1".to_string()),
            title: "Legacy".to_string(),
            source_path: "legacy.md".to_string(),
            active_context: None,
            compact_summary: String::new(),
            updated_at: 1,
            nodes: Vec::new(),
            messages: vec![test_message("legacy message")],
        };
        fs::write(
            chat_dir.join("session-index.json"),
            r#"{"schemaVersion":1,"activeSessionId":"legacy-session","updatedAt":1}"#,
        )
        .expect("write legacy index");
        fs::write(
            sessions_dir.join("legacy-session.json"),
            serde_json::to_string_pretty(&session).expect("serialize session"),
        )
        .expect("write legacy session");

        let continuity = load_chat_continuity_from_chat_dir(&chat_dir, None).expect("load legacy");

        assert_eq!(continuity.session_id, "legacy-session");
        assert_eq!(continuity.messages[0].text, "legacy message");

        let _ = fs::remove_dir_all(chat_dir.parent().expect("test root parent"));
    }

    fn test_chat_dir(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "wridian-chat-persistence-{name}-{}",
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&root);
        root.join("chat")
    }

    fn test_input(
        session_id: &str,
        project_id: Option<&str>,
        text: &str,
    ) -> SaveChatTranscriptInput {
        SaveChatTranscriptInput {
            session_id: session_id.to_string(),
            parent_session_id: None,
            forked_from_message_id: None,
            project_id: project_id.map(ToOwned::to_owned),
            title: "Test".to_string(),
            source_path: "test.md".to_string(),
            active_context: None,
            messages: vec![test_message(text)],
        }
    }

    fn test_message(text: &str) -> ChatTranscriptMessage {
        ChatTranscriptMessage {
            id: "message-1".to_string(),
            role: "user".to_string(),
            text: text.to_string(),
            created_at: Some(1),
            selected_text: None,
            context_pills: None,
            context_load_status: None,
            file_operations: None,
        }
    }
}
