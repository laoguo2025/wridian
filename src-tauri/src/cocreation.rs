use crate::memory::read_relevant_memory_snippets;
use crate::model_accounts::{read_custom_api_settings, StoredCustomApiSettings};
use crate::projects::{active_project_model, read_active_project_context};
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateInput {
    source_path: String,
    title: String,
    content: String,
    draft_kind: Option<String>,
    user_input: String,
    selected_text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories_used: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateEdit {
    target: String,
    replacement: String,
    rationale: Option<String>,
}

#[tauri::command]
pub(crate) async fn wridian_cocreate(input: CoCreateInput) -> Result<CoCreateResponse, String> {
    let user_input = input.user_input.trim();
    if user_input.is_empty() {
        return Err("共创输入不能为空。".to_string());
    }

    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    let settings = read_custom_api_settings(&data_dir)?
        .ok_or_else(|| "请先在模型设置里保存第三方 API。".to_string())?;
    let memories_used =
        read_relevant_memory_snippets(&data_dir, &input.source_path, &input.title, 8)?;
    let active_context = read_active_context(&data_dir);
    let active_project_context = read_active_project_context(&data_dir)?;
    let project_model = active_project_model(&data_dir)?;
    let model_output = cocreate_with_model(
        &settings,
        project_model.as_deref(),
        &input,
        &memories_used,
        &active_context,
        &active_project_context,
    )
    .await?;

    Ok(CoCreateResponse {
        reply: model_output.reply,
        edits: model_output.edits,
        memories_used,
    })
}

#[derive(Debug, Deserialize)]
struct ModelCoCreateResponse {
    reply: Option<String>,
    #[serde(default)]
    edits: Vec<CoCreateEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
}

async fn cocreate_with_model(
    settings: &StoredCustomApiSettings,
    project_model: Option<&str>,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> Result<ParsedCoCreateResponse, String> {
    let url = format!("{}/chat/completions", settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("共创客户端创建失败：{error}"))?;
    let response = client
        .post(url)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": project_model.unwrap_or(&settings.model),
            "messages": [
                {
                    "role": "system",
                    "content": cocreation_system_prompt()
                },
                {
                    "role": "user",
                    "content": build_cocreation_prompt(input, memories, active_context, active_project_context)
                }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.7
        }))
        .send()
        .await
        .map_err(|error| format!("共创请求失败：{error}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "共创请求失败：HTTP {} {}",
            status.as_u16(),
            body.chars().take(240).collect::<String>()
        ));
    }
    let content = read_chat_completion_content(&body)?;
    let parsed = parse_cocreation_model_output(&content)?;
    if parsed.reply.trim().is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(parsed)
    }
}

fn read_active_context(data_dir: &std::path::Path) -> String {
    let path = runtime_root(data_dir).join("active-context.json");
    fs::read_to_string(path)
        .map(|content| compact_text(&content, 1200))
        .unwrap_or_default()
}

fn build_cocreation_prompt(
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
    active_project_context: &str,
) -> String {
    let memories_block = if memories.is_empty() {
        "暂无记忆树上下文。".to_string()
    } else {
        memories
            .iter()
            .map(|memory| format!("- {memory}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let active_context_block = if active_context.is_empty() {
        "暂无当前现场。".to_string()
    } else {
        active_context.to_string()
    };
    let active_project_block = if active_project_context.trim().is_empty() {
        "未启用 Project Mode。".to_string()
    } else {
        active_project_context.to_string()
    };
    let draft_kind = match input.draft_kind.as_deref() {
        Some("screenplay") => "短剧/剧本稿件",
        _ => "小说/散文稿件",
    };

    format!(
        "稿件类型：{}\n当前文件：{}\n来源路径：{}\n\nProject Mode：\n{}\n\n当前现场：\n{}\n\n记忆树上下文：\n{}\n\n用户选中的片段：\n{}\n\n稿件内容：\n{}\n\n用户这次想要：\n{}",
        draft_kind,
        input.title,
        input.source_path,
        active_project_block,
        active_context_block,
        memories_block,
        input
            .selected_text
            .as_deref()
            .map(|text| compact_text(text, 3000))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "未选择片段。".to_string()),
        compact_text(&input.content, 7000),
        input.user_input.trim()
    )
}

fn read_chat_completion_content(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("共创响应格式损坏：{error}"))?;
    value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "共创响应缺少 choices[0].message.content。".to_string())
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_chars).collect()
}

fn parse_cocreation_model_output(output: &str) -> Result<ParsedCoCreateResponse, String> {
    let parsed: ModelCoCreateResponse = serde_json::from_str(output.trim())
        .map_err(|error| format!("共创结果不是有效 JSON：{error}"))?;
    let reply = parsed.reply.unwrap_or_default().trim().to_string();
    let edits = parsed
        .edits
        .into_iter()
        .filter_map(|edit| {
            let target = edit.target.trim().to_string();
            let replacement = edit.replacement.trim().to_string();
            if target.is_empty() || replacement.is_empty() || target == replacement {
                return None;
            }
            Some(CoCreateEdit {
                target,
                replacement,
                rationale: edit
                    .rationale
                    .map(|text| text.trim().to_string())
                    .filter(|text| !text.is_empty()),
            })
        })
        .collect();
    Ok(ParsedCoCreateResponse { reply, edits })
}

fn cocreation_system_prompt() -> &'static str {
    r#"你是 Wridian 的写作共创助手。
你的任务是围绕当前稿件给出可执行的写作建议、局部改写方案或结构判断。
你会同时服务小说和短剧/剧本创作：小说关注章节、人物动机、叙述节奏、伏笔和设定一致性；短剧/剧本关注对白、场景冲突、钩子、角色口吻和分集节奏。
当稿件类型是短剧/剧本时，优先关注场次、对白可表演性、结尾钩子、分集节奏和低成本拍摄约束。
不要写成通用聊天回复；不要自动声称已经修改正文；不要把普通共创内容写入长期记忆。
必须输出 JSON 对象：
{"reply":"给用户看的正常回复","edits":[{"target":"需要被替换的原文片段，必须从稿件内容或用户选中片段中逐字复制","replacement":"替换后的新文本","rationale":"简短理由"}]}
如果只是聊天、讨论、解释，edits 输出空数组。
如果用户要求修改、润色、批量替换角色名、调整对白或重写片段，必须尽量给 edits。
target 必须是原文中存在的精确片段；不要用行号、摘要或正则；不能确定精确原文时只给 reply，不给 edits。"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_keeps_draft_memories_and_user_request_separate() {
        let input = CoCreateInput {
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "强化她进门前的动机".to_string(),
            selected_text: None,
        };
        let prompt = build_cocreation_prompt(
            &input,
            &["【剧情线】雨夜场景不能提前暴露凶手。".to_string()],
            "{\"currentChapter\":\"第三章\"}",
            "Project Mode：短剧项目",
        );

        assert!(prompt.contains("稿件内容"));
        assert!(prompt.contains("记忆树上下文"));
        assert!(prompt.contains("强化她进门前的动机"));
    }

    #[test]
    fn read_chat_completion_content_reads_first_choice_message() {
        let body = r#"{
            "choices": [
                { "message": { "content": "可以先补一段动作。" } }
            ]
        }"#;

        let content = read_chat_completion_content(body).expect("content exists");

        assert_eq!(content, "可以先补一段动作。");
    }

    #[test]
    fn parse_cocreation_model_output_reads_reply_and_edits() {
        let output = r#"{
            "reply": "我会把动机提前到进门动作里。",
            "edits": [
                {
                    "target": "她没有立刻喊人。",
                    "replacement": "她没有立刻喊人，而是先摸了摸口袋里那把旧钥匙。",
                    "rationale": "让动机从动作里出现。"
                }
            ]
        }"#;

        let parsed = parse_cocreation_model_output(output).expect("valid output");

        assert_eq!(parsed.reply, "我会把动机提前到进门动作里。");
        assert_eq!(parsed.edits.len(), 1);
        assert_eq!(parsed.edits[0].target, "她没有立刻喊人。");
    }
}
