use super::{CardPageBlockApiType, Value};

pub(super) fn validate_code_block_content(
    block: &Value,
    block_id: &str,
    api_type: &CardPageBlockApiType,
) -> Result<(), String> {
    if api_type.as_str() != "code" {
        return Ok(());
    }
    match block.get("content") {
        None => Ok(()),
        Some(Value::Array(children)) if children.is_empty() => Ok(()),
        Some(Value::Array(_)) => Err(format!(
            "Notion code block {block_id} unexpectedly contains child content"
        )),
        Some(_) => Err(format!(
            "Notion code block {block_id} contains non-array child content"
        )),
    }
}

pub(super) fn block_allows_content_traversal(api_type: &CardPageBlockApiType) -> bool {
    !matches!(
        api_type.as_str(),
        "page"
            | "link_to_page"
            | "alias"
            | "code"
            | "divider"
            | "collection_view"
            | "collection_view_page"
            | "image"
    )
}
