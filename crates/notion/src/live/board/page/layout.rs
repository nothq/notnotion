use super::Value;
use crate::model::{CardPageBlockApiType, CardPageLayoutBlock};

pub(super) fn page_layout_block(
    block_id: &str,
    block: &Value,
    api_type: &CardPageBlockApiType,
    has_traversable_content: bool,
) -> Result<Option<CardPageLayoutBlock>, String> {
    match api_type.as_str() {
        "column_list" => Ok(Some(CardPageLayoutBlock::ColumnList)),
        "column" => Ok(Some(CardPageLayoutBlock::Column {
            ratio: super::super::column_ratio::parse_live_column_ratio(block, block_id)?,
        })),
        _ if has_traversable_content => Ok(Some(CardPageLayoutBlock::Passthrough {
            api_type: api_type.clone(),
        })),
        _ => Ok(None),
    }
}
