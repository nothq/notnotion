use super::super::explicit_page_shell_icon;
use super::{CardPageBlock, PageBlockParseWork, ParsedPageBlocks};
use crate::model::CardPageBlockKind;

pub(super) fn parsed_editable_page_block(
    work: &PageBlockParseWork<'_, '_>,
    parsed: &mut ParsedPageBlocks,
) -> Result<Option<CardPageBlock>, String> {
    let PageBlockParseWork {
        context,
        parent_block_id,
        child_id: block_id,
        block,
        api_type,
        depth,
        ..
    } = *work;
    let Some(editable) =
        super::values::page_editable_block(context, block_id, block, api_type, parsed)?
    else {
        return Ok(None);
    };
    let mut page_block = CardPageBlock::from_editable(block_id, parent_block_id, depth, editable);
    if page_block.editable_content().is_some_and(|editable| {
        matches!(
            editable.kind,
            CardPageBlockKind::PageLink | CardPageBlockKind::Callout
        )
    }) {
        let editable = page_block
            .editable_content()
            .expect("matched editable page block must retain editable content");
        let fallback = if editable.kind == CardPageBlockKind::Callout {
            "callout"
        } else {
            "page"
        };
        page_block.icon = explicit_page_shell_icon(block, fallback);
    }
    Ok(Some(page_block))
}
