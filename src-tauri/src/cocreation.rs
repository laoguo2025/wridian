use crate::memory::{read_relevant_memory_snippets, write_memory_leaves, MemoryLeafDraft};
use crate::model_accounts::{read_custom_api_settings, StoredCustomApiSettings};
use crate::projects::{active_project_model, read_active_project_context};
use crate::runtime::{ensure_workspace, runtime_root, wridian_data_dir};
use crate::workspace::{read_active_work_root, resolved_knowledge_root};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
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
    #[serde(default)]
    context_items: Vec<DialogueContextItem>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DialogueContextItem {
    kind: String,
    label: String,
    value: String,
    source_path: Option<String>,
    relative_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories_used: Vec<String>,
    memories_written: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoCreateEdit {
    target: String,
    replacement: String,
    rationale: Option<String>,
}

#[tauri::command]
pub(crate) async fn wridian_cocreate(mut input: CoCreateInput) -> Result<CoCreateResponse, String> {
    let user_input = input.user_input.trim();
    if user_input.is_empty() {
        return Err("对话输入不能为空。".to_string());
    }

    let data_dir = wridian_data_dir()?;
    ensure_workspace(&data_dir)?;
    input.context_items = expand_context_items(&data_dir, &input.context_items)?;
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

    let memories_written = write_memory_leaves(&data_dir, &model_output.memories)?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    Ok(CoCreateResponse {
        reply: model_output.reply,
        edits: model_output.edits,
        memories_used,
        memories_written,
    })
}

#[derive(Debug, Deserialize)]
struct ModelCoCreateResponse {
    reply: Option<String>,
    #[serde(default)]
    edits: Vec<CoCreateEdit>,
    #[serde(default)]
    memories: Vec<MemoryLeafDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCoCreateResponse {
    reply: String,
    edits: Vec<CoCreateEdit>,
    memories: Vec<MemoryLeafDraft>,
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
        .map_err(|error| format!("对话客户端创建失败：{error}"))?;
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
        .map_err(|error| format!("对话请求失败：{error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("对话响应读取失败：{error}"))?;
    if !status.is_success() {
        return Err(format!(
            "对话请求失败：HTTP {} {}",
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

fn read_active_context(data_dir: &Path) -> String {
    let path = runtime_root(data_dir).join("active-context.json");
    fs::read_to_string(path)
        .map(|content| compact_text(&content, 1200))
        .unwrap_or_default()
}

fn expand_context_items(
    data_dir: &Path,
    items: &[DialogueContextItem],
) -> Result<Vec<DialogueContextItem>, String> {
    items
        .iter()
        .map(|item| expand_context_item(data_dir, item))
        .collect()
}

fn expand_context_item(
    data_dir: &Path,
    item: &DialogueContextItem,
) -> Result<DialogueContextItem, String> {
    let Some(raw_path) = referenced_context_path(item) else {
        return Ok(item.clone());
    };
    let (path, relative_path) = resolve_allowed_context_file(data_dir, &raw_path)?;
    let content =
        fs::read_to_string(&path).map_err(|error| format!("上下文文件读取失败：{error}"))?;
    let mut expanded = item.clone();
    expanded.source_path = Some(path.to_string_lossy().into_owned());
    if expanded
        .relative_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        expanded.relative_path = Some(relative_path);
    }
    expanded.value = content;
    Ok(expanded)
}

fn referenced_context_path(item: &DialogueContextItem) -> Option<PathBuf> {
    let source_path = item.source_path.as_deref().unwrap_or("").trim();
    if !source_path.is_empty() {
        return Some(PathBuf::from(source_path));
    }
    let value = item.value.trim();
    if let Some(path) = value.strip_prefix("path:") {
        let trimmed = path.trim();
        return (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
    }
    if let Some(path) = value.strip_prefix("路径：") {
        let trimmed = path.lines().next().unwrap_or("").trim();
        return (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
    }
    None
}

fn resolve_allowed_context_file(
    data_dir: &Path,
    raw_path: &Path,
) -> Result<(PathBuf, String), String> {
    let canonical = raw_path
        .canonicalize()
        .map_err(|error| format!("上下文文件不存在：{error}"))?;
    if !canonical.is_file() {
        return Err("上下文引用必须指向文件。".to_string());
    }
    for root in selected_context_roots(data_dir)? {
        if canonical.starts_with(&root) {
            let relative = canonical
                .strip_prefix(&root)
                .unwrap_or(&canonical)
                .to_string_lossy()
                .replace('\\', "/");
            return Ok((canonical, relative));
        }
    }
    Err("上下文文件不在已选择的作品库或知识库中。".to_string())
}

fn selected_context_roots(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(root) = read_active_work_root(data_dir)? {
        let path = PathBuf::from(root);
        if path.is_dir() {
            roots.push(
                path.canonicalize()
                    .map_err(|error| format!("作品库目录解析失败：{error}"))?,
            );
        }
    }
    let knowledge = resolved_knowledge_root(data_dir)?;
    if knowledge.is_dir() {
        roots.push(
            knowledge
                .canonicalize()
                .map_err(|error| format!("知识库目录解析失败：{error}"))?,
        );
    }
    Ok(roots)
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
    let explicit_context_block = render_context_items(&input.context_items);

    let source_label = prompt_source_label(&input.source_path, &input.title);
    format!(
        "稿件类型：{}\n当前文件：{}\n来源路径：{}\n\nProject Mode：\n{}\n\n当前现场：\n{}\n\n记忆树上下文：\n{}\n\n显式上下文：\n{}\n\n用户选中的片段：\n{}\n\n稿件内容：\n{}\n\n用户这次想要：\n{}",
        draft_kind,
        input.title,
        source_label,
        active_project_block,
        active_context_block,
        memories_block,
        explicit_context_block,
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

fn prompt_source_label(source_path: &str, title: &str) -> String {
    Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| (!title.trim().is_empty()).then(|| title.trim().to_string()))
        .unwrap_or_else(|| "当前稿件".to_string())
}

fn render_context_items(items: &[DialogueContextItem]) -> String {
    if items.is_empty() {
        return "无显式上下文。".to_string();
    }
    items
        .iter()
        .filter_map(|item| {
            let value = compact_text(&item.value, 1800);
            if value.trim().is_empty() {
                return None;
            }
            let source = item
                .relative_path
                .as_deref()
                .or(item.source_path.as_deref())
                .unwrap_or("")
                .trim();
            let header = if source.is_empty() {
                format!("【{}｜{}】", item.kind.trim(), item.label.trim())
            } else {
                format!(
                    "【{}｜{}｜{}】",
                    item.kind.trim(),
                    item.label.trim(),
                    source
                )
            };
            Some(format!("{header}\n{value}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_chat_completion_content(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("对话响应格式损坏：{error}"))?;
    value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "对话响应缺少 choices[0].message.content。".to_string())
}

fn compact_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_chars).collect()
}

fn parse_cocreation_model_output(output: &str) -> Result<ParsedCoCreateResponse, String> {
    let parsed: ModelCoCreateResponse = serde_json::from_str(output.trim())
        .map_err(|error| format!("对话结果不是有效 JSON：{error}"))?;
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
    let memories = parsed
        .memories
        .into_iter()
        .filter_map(normalize_model_memory_leaf)
        .collect();
    Ok(ParsedCoCreateResponse {
        reply,
        edits,
        memories,
    })
}

fn normalize_model_memory_leaf(mut leaf: MemoryLeafDraft) -> Option<MemoryLeafDraft> {
    leaf.branch = leaf.branch.trim().to_lowercase();
    leaf.title = leaf.title.trim().to_string();
    leaf.summary = leaf.summary.trim().to_string();
    leaf.reason = leaf
        .reason
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    leaf.source_path = leaf
        .source_path
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    if leaf.branch.is_empty() || leaf.title.is_empty() || leaf.summary.is_empty() {
        None
    } else {
        Some(leaf)
    }
}

fn cocreation_system_prompt() -> &'static str {
    r#"你是 Wridian 的写作对话助手。
你的任务是围绕当前稿件给出可执行的写作建议、局部改写方案或结构判断。
你会同时服务小说和短剧/剧本创作：小说关注章节、人物动机、叙述节奏、伏笔和设定一致性；短剧/剧本关注对白、场景冲突、钩子、角色口吻和分集节奏。
当稿件类型是短剧/剧本时，优先关注场次、对白可表演性、结尾钩子、分集节奏和低成本拍摄约束。
不要写成通用聊天回复；不要自动声称已经修改正文。
你需要判断本轮是否产生值得长期保留的创作记忆。如果没有，memories 输出空数组；如果有，只提取稳定、可复用、对后续写作有约束或参考价值的事实，不记录一次性闲聊。
必须输出 JSON 对象：
{"reply":"给用户看的正常回复","edits":[{"target":"需要被替换的原文片段，必须从稿件内容或用户选中片段中逐字复制","replacement":"替换后的新文本","rationale":"简短理由"}],"memories":[{"branch":"novel|drama|knowledge|skill|user|relationship|journey|awareness|sense","title":"短标题","summary":"要沉淀的长期记忆正文","reason":"为什么值得沉淀","sourcePath":"当前来源路径或空"}]}
如果只是聊天、讨论、解释，edits 输出空数组。
如果用户要求修改、润色、批量替换角色名、调整对白或重写片段，必须尽量给 edits。
target 必须是原文中存在的精确片段；不要用行号、摘要或正则；不能确定精确原文时只给 reply，不给 edits。"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wridian-cocreation-test-{}-{}",
            name,
            crate::runtime::unique_test_suffix()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp data dir");
        path
    }

    #[test]
    fn build_prompt_keeps_draft_memories_and_user_request_separate() {
        let input = CoCreateInput {
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "强化她进门前的动机".to_string(),
            selected_text: None,
            context_items: Vec::new(),
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
    fn build_prompt_separates_selection_from_explicit_context_items() {
        let input = CoCreateInput {
            source_path: "demo://03.md".to_string(),
            title: "03.md".to_string(),
            content: "她推开门，没有立刻喊人。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "结合人物卡改写".to_string(),
            selected_text: Some("她推开门".to_string()),
            context_items: vec![DialogueContextItem {
                kind: "memory".to_string(),
                label: "人物卡".to_string(),
                value: "她怕黑，但不承认。".to_string(),
                source_path: Some("D:/vault/knowledge/人物卡.md".to_string()),
                relative_path: Some("人物卡.md".to_string()),
            }],
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "");

        assert!(prompt.contains("用户选中的片段：\n她推开门"));
        assert!(prompt.contains("显式上下文："));
        assert!(prompt.contains("【memory｜人物卡｜人物卡.md】\n她怕黑，但不承认。"));
    }

    #[test]
    fn build_prompt_does_not_send_absolute_source_path() {
        let input = CoCreateInput {
            source_path: "D:/private/vault/works/第一章.md".to_string(),
            title: "第一章.md".to_string(),
            content: "她推开门。".to_string(),
            draft_kind: Some("prose".to_string()),
            user_input: "润色".to_string(),
            selected_text: None,
            context_items: Vec::new(),
        };

        let prompt = build_cocreation_prompt(&input, &[], "", "");

        assert!(prompt.contains("来源路径：第一章.md"));
        assert!(!prompt.contains("D:/private"));
    }

    #[test]
    fn prompt_source_label_falls_back_to_title() {
        assert_eq!(prompt_source_label("", "未保存稿件"), "未保存稿件");
        assert_eq!(prompt_source_label("", ""), "当前稿件");
    }

    #[test]
    fn expands_path_context_items_from_selected_library() {
        let data_dir = temp_data_dir("path-context");
        let work_root = data_dir.join("works");
        let knowledge_root = data_dir.join("knowledge");
        fs::create_dir_all(&work_root).expect("create works");
        fs::create_dir_all(&knowledge_root).expect("create knowledge");
        let card_path = knowledge_root.join("人物.md");
        fs::write(&card_path, "她怕黑，但不承认。").expect("write card");
        fs::create_dir_all(crate::runtime::runtime_root(&data_dir)).expect("create runtime");
        fs::write(
            crate::runtime::workspace_config_path(&data_dir),
            serde_json::json!({
                "schemaVersion": 1,
                "activeWorkRoot": work_root.to_string_lossy(),
                "knowledgeRoot": knowledge_root.to_string_lossy()
            })
            .to_string(),
        )
        .expect("write workspace config");

        let expanded = expand_context_items(
            &data_dir,
            &[DialogueContextItem {
                kind: "memory".to_string(),
                label: "人物".to_string(),
                value: format!("path:{}", card_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("expand context");

        assert_eq!(expanded[0].value, "她怕黑，但不承认。");
        assert_eq!(expanded[0].relative_path.as_deref(), Some("人物.md"));
    }

    #[test]
    fn accepts_default_knowledge_context_without_user_selection() {
        let data_dir = temp_data_dir("default-knowledge-context");
        crate::runtime::ensure_workspace(&data_dir).expect("ensure workspace");
        let default_knowledge = crate::runtime::default_knowledge_root(&data_dir);
        let card_path = default_knowledge.join("03故事模型").join("默认人物.md");
        fs::write(&card_path, "默认知识").expect("write default card");

        let expanded = expand_context_items(
            &data_dir,
            &[DialogueContextItem {
                kind: "memory".to_string(),
                label: "默认人物".to_string(),
                value: format!("path:{}", card_path.to_string_lossy()),
                source_path: None,
                relative_path: None,
            }],
        )
        .expect("default knowledge context should be accepted");

        assert_eq!(expanded[0].value, "默认知识");
        assert_eq!(
            expanded[0].relative_path.as_deref(),
            Some("03故事模型/默认人物.md")
        );
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

    #[test]
    fn parse_cocreation_model_output_reads_memory_leaves() {
        let output = r#"{
            "reply": "这个人物禁区我会记住。",
            "edits": [],
            "memories": [
                {
                    "branch": "novel",
                    "title": "人物禁区",
                    "summary": "女主不能主动说出真相。",
                    "reason": "这是后续章节约束。",
                    "sourcePath": "chapter.md"
                }
            ]
        }"#;

        let parsed = parse_cocreation_model_output(output).expect("valid output");

        assert_eq!(parsed.memories.len(), 1);
        assert_eq!(parsed.memories[0].branch, "novel");
        assert_eq!(parsed.memories[0].title, "人物禁区");
    }
}
