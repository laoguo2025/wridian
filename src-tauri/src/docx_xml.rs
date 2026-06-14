pub(crate) fn minimal_docx_document_xml(content: &str) -> String {
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
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                encode_xml_text(paragraph)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    )
}

pub(crate) fn encode_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 将纯文本回写到既有 DOCX 时尽量保留原文档的段落级格式。
///
/// 从 `original_xml`（原 `word/document.xml`）抽取每个 `<w:p>` 的 `<w:pPr>`
/// （段落样式：标题级别、对齐、缩进、编号）与首个 `<w:r>` 的 `<w:rPr>`
/// （字符样式：加粗、斜体、字号）作为“样式模板”，按 `new_text` 的行重新映射：
/// 第 N 行复用第 N 个模板；行数多于模板时尾部一律套用最后一个模板
/// （契合“标题 + 正文”写作结构：用户常在正文段后追加段落）。
///
/// 仅保留段落级与首 run 字符级格式；行内混合格式（如段中加粗）会被压平为首 run 样式。
/// 解析失败（原文档无可识别段落）时返回 `Err`，调用方应回退到 `minimal_docx_document_xml`，
/// 以保证不破坏当前行为。
pub(crate) fn round_trip_document_xml(
    original_xml: &str,
    new_text: &str,
) -> Result<String, String> {
    let templates = extract_paragraph_templates(original_xml);
    if templates.is_empty() {
        return Err("未在原文档中找到可识别的段落结构。".to_string());
    }

    let lines: Vec<&str> = if new_text.is_empty() {
        vec![""]
    } else {
        new_text.split('\n').collect()
    };
    let last_index = templates.len().saturating_sub(1);

    let body = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let template = &templates[index.min(last_index)];
            render_style_aware_paragraph(line, template)
        })
        .collect::<Vec<_>>()
        .join("");

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    ))
}

struct ParagraphTemplate {
    /// 完整的 `<w:pPr>...</w:pPr>` 片段（含标签），段落不存在时为 `None`。
    ppr: Option<String>,
    /// 首个 `<w:r>` 内完整的 `<w:rPr>...</w:rPr>` 片段（含标签），不存在时为 `None`。
    rpr: Option<String>,
}

/// 按出现顺序抽取每个 `<w:p>` 的样式模板。
///
/// 复用 `docx_document_xml_to_text` 的扫描约定：匹配 `<w:p`（非 `<w:p>`），
/// 以兼容带属性的段落标签（如 `<w:p w:rsidR="...">`）。
fn extract_paragraph_templates(xml: &str) -> Vec<ParagraphTemplate> {
    let mut templates = Vec::new();
    let mut search = 0;
    while let Some(relative) = xml[search..].find("<w:p") {
        let paragraph_start = search + relative;
        let Some(open_end) = xml[paragraph_start..].find('>') else {
            break;
        };
        let content_start = paragraph_start + open_end + 1;
        let Some(close_offset) = xml[content_start..].find("</w:p>") else {
            break;
        };
        let content_end = content_start + close_offset;
        let content = &xml[content_start..content_end];
        let ppr = extract_tag_block(content, "<w:pPr", "</w:pPr>");
        let rpr = extract_first_run_rpr(content);
        templates.push(ParagraphTemplate { ppr, rpr });
        search = content_end + "</w:p>".len();
    }
    templates
}

/// 抽取首个 `open_prefix...>` 到 `close_tag` 的完整片段（含首尾标签）。
///
/// 例如 `extract_tag_block(content, "<w:pPr", "</w:pPr>")` 返回
/// `<w:pPr><w:pStyle w:val="2"/></w:pPr>`。找不到时返回 `None`。
fn extract_tag_block(content: &str, open_prefix: &str, close_tag: &str) -> Option<String> {
    let start = content.find(open_prefix)?;
    let open_end = content[start..].find('>')?;
    let inner_start = start + open_end + 1;
    let close_offset = content[inner_start..].find(close_tag)?;
    let block_end = inner_start + close_offset + close_tag.len();
    Some(content[start..block_end].to_string())
}

/// 抽取段落内首个 `<w:r>` 的 `<w:rPr>` 片段。
///
/// 需要跳过形如 `<w:rPr>`（前缀同为 `<w:r`）的误匹配：只有当 `<w:r` 后紧跟
/// `>` 或属性空白时才认定为 run。`<w:rPr>` 后跟 `P`，故被跳过。
fn extract_first_run_rpr(content: &str) -> Option<String> {
    let mut search = 0;
    loop {
        let relative = content[search..].find("<w:r")?;
        let run_start = search + relative;
        let after_prefix = &content[run_start + 4..];
        let next_char = after_prefix.chars().next()?;
        if next_char == '>' || next_char == ' ' {
            let open_end = after_prefix.find('>')?;
            let inner_start = run_start + 4 + open_end + 1;
            let close_offset = content[inner_start..].find("</w:r>")?;
            let run = &content[inner_start..inner_start + close_offset];
            return extract_tag_block(run, "<w:rPr", "</w:rPr>");
        }
        search = run_start + 4;
    }
}

fn render_style_aware_paragraph(text: &str, template: &ParagraphTemplate) -> String {
    let ppr = template.ppr.as_deref().unwrap_or("");
    let rpr = template.rpr.as_deref().unwrap_or("");
    format!(
        "<w:p>{ppr}<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        encode_xml_text(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with_paragraphs(paragraphs: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{paragraphs}<w:sectPr/></w:body></w:document>"#
        )
    }

    #[test]
    fn round_trip_preserves_heading_and_body_styles() {
        let original = document_with_paragraphs(
            "<w:p><w:pPr><w:pStyle w:val=\"2\"/></w:pPr><w:r><w:t xml:space=\"preserve\">旧标题</w:t></w:r></w:p>\
             <w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">旧正文</w:t></w:r></w:p>",
        );
        let result = round_trip_document_xml(&original, "新标题\n新正文").expect("round-trip");

        assert!(
            result.contains("<w:pPr><w:pStyle w:val=\"2\"/></w:pPr>"),
            "标题段落样式应保留：{result}"
        );
        assert!(
            result.contains("<w:rPr><w:b/></w:rPr>"),
            "正文首 run 加粗样式应保留：{result}"
        );
        assert!(
            result.contains("新标题") && result.contains("新正文"),
            "新文本应写入：{result}"
        );
        assert!(
            !result.contains("旧标题") && !result.contains("旧正文"),
            "旧文本不应残留：{result}"
        );
    }

    #[test]
    fn round_trip_reuses_last_template_for_extra_lines() {
        // 标题段落 + 正文段落；新文本 4 行，后 3 行应套用正文（最后）模板。
        let original = document_with_paragraphs(
            "<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t xml:space=\"preserve\">H</w:t></w:r></w:p>\
             <w:p><w:r><w:t xml:space=\"preserve\">B</w:t></w:r></w:p>",
        );
        let result = round_trip_document_xml(&original, "标题\n正文一\n正文二\n正文三").expect("round-trip");

        let heading_count = result.matches("<w:pStyle w:val=\"Heading1\"/>").count();
        assert_eq!(heading_count, 1, "只有首行应是标题样式：{result}");
        let paragraph_count = result.matches("<w:p>").count();
        assert_eq!(paragraph_count, 4, "应输出 4 个段落：{result}");
    }

    #[test]
    fn round_trip_handles_cjk_with_padding_spaces() {
        let original = document_with_paragraphs(
            "<w:p><w:r><w:t xml:space=\"preserve\">旧</w:t></w:r></w:p>",
        );
        let result = round_trip_document_xml(&original, "  带空格的中文  ").expect("round-trip");

        assert!(
            result.contains("xml:space=\"preserve\""),
            "应带 xml:space=preserve 防空格丢失：{result}"
        );
        assert!(
            result.contains("  带空格的中文  "),
            "CJK + 前后空格应原样保留：{result}"
        );
    }

    #[test]
    fn round_trip_empty_text_yields_single_empty_paragraph() {
        let original = document_with_paragraphs(
            "<w:p><w:pPr><w:pStyle w:val=\"2\"/></w:pPr><w:r><w:t xml:space=\"preserve\">旧</w:t></w:r></w:p>",
        );
        let result = round_trip_document_xml(&original, "").expect("round-trip");

        let paragraph_count = result.matches("<w:p>").count();
        assert_eq!(paragraph_count, 1, "空文本应输出 1 个段落：{result}");
        assert!(
            result.contains("<w:pStyle w:val=\"2\"/>"),
            "空段落应继承首个模板样式：{result}"
        );
    }

    #[test]
    fn round_trip_returns_err_when_no_paragraphs() {
        let broken = "<?xml version=\"1.0\"?><w:document><w:body><w:sectPr/></w:body></w:document>";
        let result = round_trip_document_xml(broken, "任意文本");

        assert!(result.is_err(), "无可识别段落应返回 Err 触发回退");
    }

    #[test]
    fn round_trip_skips_rpr_inside_ppr_and_uses_first_run() {
        // <w:pPr> 内嵌 <w:rPr>（段落标记样式）不应被误当作首 run 的字符样式。
        let original = document_with_paragraphs(
            "<w:p><w:pPr><w:pStyle w:val=\"2\"/><w:rPr><w:i/></w:rPr></w:pPr>\
             <w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">旧</w:t></w:r></w:p>",
        );
        let result = round_trip_document_xml(&original, "新").expect("round-trip");

        assert!(
            result.contains("<w:rPr><w:b/></w:rPr>"),
            "应取首个 <w:r> 的 <w:rPr>（加粗），而非 <w:pPr> 内的斜体：{result}"
        );
    }

    #[test]
    fn minimal_document_emits_preserve_space() {
        let result = minimal_docx_document_xml("  前后空格  ");
        assert!(
            result.contains("xml:space=\"preserve\""),
            "minimal 新建也应带 xml:space=preserve：{result}"
        );
    }
}
