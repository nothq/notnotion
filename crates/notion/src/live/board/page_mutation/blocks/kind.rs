use crate::model::NotionPageBlockKind;

pub(super) fn creatable_block_type(kind: &NotionPageBlockKind) -> Result<&str, String> {
    match kind {
        NotionPageBlockKind::Text
        | NotionPageBlockKind::Header
        | NotionPageBlockKind::SubHeader
        | NotionPageBlockKind::SubSubHeader
        | NotionPageBlockKind::Header4
        | NotionPageBlockKind::BulletedList
        | NotionPageBlockKind::NumberedList
        | NotionPageBlockKind::ToDo
        | NotionPageBlockKind::Toggle
        | NotionPageBlockKind::Page
        | NotionPageBlockKind::Callout
        | NotionPageBlockKind::Quote
        | NotionPageBlockKind::Divider => Ok(kind.api_type()),
        NotionPageBlockKind::Table
        | NotionPageBlockKind::LinkToPage
        | NotionPageBlockKind::Alias
        | NotionPageBlockKind::Code
        | NotionPageBlockKind::Image => Err(format!(
            "creating {} requires additional unverified Notion block data",
            kind.api_type()
        )),
        NotionPageBlockKind::Other(block_type) => Err(format!(
            "creating unsupported Notion block type {block_type} is not allowed"
        )),
    }
}

pub(super) fn convertible_block_type(kind: &NotionPageBlockKind) -> Result<&str, String> {
    match kind {
        NotionPageBlockKind::Text
        | NotionPageBlockKind::Header
        | NotionPageBlockKind::SubHeader
        | NotionPageBlockKind::SubSubHeader
        | NotionPageBlockKind::Header4
        | NotionPageBlockKind::BulletedList
        | NotionPageBlockKind::NumberedList
        | NotionPageBlockKind::ToDo
        | NotionPageBlockKind::Toggle
        | NotionPageBlockKind::Callout
        | NotionPageBlockKind::Quote => Ok(kind.api_type()),
        NotionPageBlockKind::Page
        | NotionPageBlockKind::Table
        | NotionPageBlockKind::Divider
        | NotionPageBlockKind::LinkToPage
        | NotionPageBlockKind::Alias
        | NotionPageBlockKind::Code
        | NotionPageBlockKind::Image => Err(format!(
            "converting to {} requires an unverified Notion block transform",
            kind.api_type()
        )),
        NotionPageBlockKind::Other(block_type) => Err(format!(
            "converting to unsupported Notion block type {block_type} is not allowed"
        )),
    }
}

pub(super) fn require_text_navigation_kind(
    kind: &NotionPageBlockKind,
    operation: &str,
) -> Result<(), String> {
    convertible_block_type(kind).map(|_| ()).map_err(|_| {
        format!(
            "{operation} {} is not a verified text-block mutation",
            kind.api_type()
        )
    })
}
