use serde_json::Value;

use crate::model::CardPageColumnRatio;

pub(super) fn parse_live_column_ratio(
    block: &Value,
    block_id: &str,
) -> Result<Option<CardPageColumnRatio>, String> {
    let Some(format) = block.get("format").filter(|format| !format.is_null()) else {
        return Ok(None);
    };
    let format = format
        .as_object()
        .ok_or_else(|| format!("Notion column {block_id} contains a non-object format"))?;
    let Some(raw_ratio) = format.get("column_ratio") else {
        return Ok(None);
    };
    let fraction = raw_ratio
        .as_f64()
        .ok_or_else(|| format!("Notion column {block_id} contains a non-numeric column ratio"))?;
    CardPageColumnRatio::from_fraction(fraction)
        .map(Some)
        .map_err(|error| format!("Notion column {block_id} contains an invalid ratio: {error}"))
}
