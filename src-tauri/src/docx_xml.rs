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

pub(crate) fn encode_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
