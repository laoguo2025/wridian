use crate::memory::read_relevant_memory_snippets;
use crate::model_accounts::{read_custom_api_settings, StoredCustomApiSettings};
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
    user_input: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateResponse {
    reply: String,
    memories_used: Vec<String>,
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
    let reply = cocreate_with_model(&settings, &input, &memories_used, &active_context).await?;

    Ok(CoCreateResponse {
        reply,
        memories_used,
    })
}

async fn cocreate_with_model(
    settings: &StoredCustomApiSettings,
    input: &CoCreateInput,
    memories: &[String],
    active_context: &str,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", settings.base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("共创客户端创建失败：{error}"))?;
    let response = client
        .post(url)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": settings.model,
            "messages": [
                {
                    "role": "system",
                    "content": cocreation_system_prompt()
                },
                {
                    "role": "user",
                    "content": build_cocreation_prompt(input, memories, active_context)
                }
            ],
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
    let reply = read_chat_completion_content(&body)?;
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        Err("模型返回了空回复。".to_string())
    } else {
        Ok(trimmed.to_string())
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
) -> String {
    let memories_block = if memories.is_empty() {
        "暂无已确认记忆。".to_string()
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

    format!(
        "当前文件：{}\n来源路径：{}\n\n当前现场：\n{}\n\n已确认相关记忆：\n{}\n\n稿件内容：\n{}\n\n用户这次想要：\n{}",
        input.title,
        input.source_path,
        active_context_block,
        memories_block,
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

fn cocreation_system_prompt() -> &'static str {
    r#"你是 Wridian 的写作共创助手。
你的任务是围绕当前稿件给出可执行的写作建议、局部改写方案或结构判断。
你会同时服务小说和短剧/剧本创作：小说关注章节、人物动机、叙述节奏、伏笔和设定一致性；短剧/剧本关注对白、场景冲突、钩子、角色口吻和分集节奏。
不要写成通用聊天回复；不要自动声称已经修改正文；不要把普通共创内容写入长期记忆。
如果用户要求改写，先给出可直接插入或替换的文本，再用简短说明解释处理理由。"#
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
            user_input: "强化她进门前的动机".to_string(),
        };
        let prompt = build_cocreation_prompt(
            &input,
            &["【剧情线】雨夜场景不能提前暴露凶手。".to_string()],
            "{\"currentChapter\":\"第三章\"}",
        );

        assert!(prompt.contains("稿件内容"));
        assert!(prompt.contains("已确认相关记忆"));
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
}
