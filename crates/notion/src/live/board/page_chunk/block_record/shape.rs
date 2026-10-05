pub(super) fn block_requires_known_content(block_type: &str) -> bool {
    matches!(
        block_type,
        "page"
            | "transcription"
            | "text"
            | "header"
            | "sub_header"
            | "sub_sub_header"
            | "header_4"
            | "bulleted_list"
            | "numbered_list"
            | "to_do"
            | "toggle"
            | "callout"
            | "quote"
            | "column_list"
            | "column"
            | "table"
            | "template"
            | "synced_block"
            | "transclusion_container"
    )
}

pub(super) fn block_requires_known_properties(block_type: &str) -> bool {
    matches!(
        block_type,
        "page"
            | "transcription"
            | "text"
            | "header"
            | "sub_header"
            | "sub_sub_header"
            | "header_4"
            | "bulleted_list"
            | "numbered_list"
            | "to_do"
            | "toggle"
            | "callout"
            | "quote"
            | "code"
            | "table_row"
            | "collection_view"
            | "collection_view_page"
    )
}
