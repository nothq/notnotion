use super::{
    loaded_record_value, page_layout_block, CardPageBlock, CardPageBlockApiType, HashMap,
    PageBlockParseContext, Value,
};

mod alias;
mod editable;
mod opaque;
mod resource;
mod shape;
mod table;
mod values;

use alias::page_alias_block;
use editable::parsed_editable_page_block;
use opaque::parsed_opaque_unavailable_block;
use resource::page_resource_block;
use shape::{block_allows_content_traversal, validate_code_block_content};
use table::{page_simple_table_block, page_simple_table_row_block, SimpleTableRowInput};

use values::{page_block_color, page_block_last_edited, page_structural_block};

#[derive(Default)]
struct ParsedPageBlocks {
    blocks: Vec<CardPageBlock>,
    page_title_cache: HashMap<String, String>,
}

struct PageBlockParseWork<'input, 'context> {
    context: &'input PageBlockParseContext<'context>,
    parent_block_id: &'input str,
    child_id: &'input str,
    block: &'input Value,
    api_type: &'input CardPageBlockApiType,
    depth: usize,
    recurse_into_content: bool,
}

pub(super) fn page_blocks(
    root_id: &str,
    root: &Value,
    context: &PageBlockParseContext<'_>,
) -> Result<Vec<CardPageBlock>, String> {
    let mut parsed = ParsedPageBlocks::default();
    append_page_blocks(context, root_id, root, 0, &mut parsed)?;
    Ok(parsed.blocks)
}

fn append_page_blocks(
    context: &PageBlockParseContext<'_>,
    parent_block_id: &str,
    parent: &Value,
    depth: usize,
    parsed: &mut ParsedPageBlocks,
) -> Result<(), String> {
    let Some(content) = parent.get("content").and_then(Value::as_array) else {
        return Ok(());
    };
    for child_id in content.iter().filter_map(Value::as_str) {
        if let Some(block) =
            parsed_opaque_unavailable_block(context, parent_block_id, child_id, depth)?
        {
            parsed.blocks.push(block);
            continue;
        }
        let block = context
            .blocks
            .get(child_id)
            .and_then(loaded_record_value)
            .ok_or_else(|| format!("missing hydrated Notion page block {child_id}"))?;
        let api_type = page_block_api_type(block, child_id)?;
        let recurse_into_content = block_allows_content_traversal(&api_type);
        let work = PageBlockParseWork {
            context,
            parent_block_id,
            child_id,
            block,
            api_type: &api_type,
            depth,
            recurse_into_content,
        };
        if api_type.as_str() == "table" {
            append_simple_table(&work, parsed)?;
            continue;
        }
        validate_code_block_content(block, child_id, &api_type)?;
        if let Some(mut page_block) = parsed_page_block(work, parsed)? {
            page_block.color = page_block_color(block, child_id)?;
            page_block.last_edited = page_block_last_edited(block, context.users);
            parsed.blocks.push(page_block);
        }
        if recurse_into_content {
            append_page_blocks(context, child_id, block, depth + 1, parsed)?;
        }
    }
    Ok(())
}

fn parsed_page_block(
    work: PageBlockParseWork<'_, '_>,
    parsed: &mut ParsedPageBlocks,
) -> Result<Option<CardPageBlock>, String> {
    let PageBlockParseWork {
        context,
        parent_block_id,
        child_id,
        block,
        api_type,
        depth,
        ..
    } = work;
    validate_page_block_is_not_orphan_table_row(child_id, api_type)?;
    if let Some(page_block) = parsed_editable_page_block(&work, parsed)? {
        return Ok(Some(page_block));
    }
    if api_type.as_str() == "alias" {
        let alias = page_alias_block(block, child_id, context)?;
        return Ok(Some(CardPageBlock::alias(
            child_id,
            parent_block_id,
            depth,
            alias,
        )));
    }
    if let Some(resource) = page_resource_block(child_id, block, api_type)? {
        return Ok(Some(CardPageBlock::resource(
            child_id,
            parent_block_id,
            depth,
            resource,
        )));
    }
    if let Some(structural) = page_structural_block(block, api_type, context.collections)? {
        return Ok(Some(CardPageBlock::structural(
            child_id,
            parent_block_id,
            depth,
            structural,
        )));
    }
    page_layout_or_unsupported_block(&work).map(Some)
}

fn validate_page_block_is_not_orphan_table_row(
    child_id: &str,
    api_type: &CardPageBlockApiType,
) -> Result<(), String> {
    if api_type.as_str() == "table_row" {
        return Err(format!(
            "orphan Notion table row {child_id} is not owned by a parsed table"
        ));
    }
    Ok(())
}

fn page_layout_or_unsupported_block(
    work: &PageBlockParseWork<'_, '_>,
) -> Result<CardPageBlock, String> {
    let PageBlockParseWork {
        parent_block_id,
        child_id,
        block,
        api_type,
        depth,
        recurse_into_content,
        ..
    } = *work;
    let has_traversable_content = recurse_into_content
        && block
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|content| !content.is_empty());
    Ok(
        page_layout_block(child_id, block, api_type, has_traversable_content)?
            .map(|layout| CardPageBlock::layout(child_id, parent_block_id, depth, layout))
            .unwrap_or_else(|| {
                CardPageBlock::unsupported_leaf(
                    child_id,
                    parent_block_id,
                    depth,
                    crate::model::CardPageUnsupportedLeafBlock::new(api_type.clone()),
                )
            }),
    )
}

fn append_simple_table(
    work: &PageBlockParseWork<'_, '_>,
    parsed: &mut ParsedPageBlocks,
) -> Result<(), String> {
    let PageBlockParseWork {
        context,
        parent_block_id,
        child_id: block_id,
        block,
        depth,
        ..
    } = *work;
    let table = page_simple_table_block(block_id, block)?;
    let mut page_block =
        CardPageBlock::simple_table(block_id, parent_block_id, depth, table.clone());
    page_block.color = page_block_color(block, block_id)?;
    page_block.last_edited = page_block_last_edited(block, context.users);
    parsed.blocks.push(page_block);
    for row_id in table.row_block_ids() {
        if let Some(block) = parsed_opaque_unavailable_block(context, block_id, row_id, depth + 1)?
        {
            parsed.blocks.push(block);
            continue;
        }
        let row = context
            .blocks
            .get(row_id)
            .and_then(loaded_record_value)
            .ok_or_else(|| format!("missing hydrated Notion table row {row_id}"))?;
        let api_type = page_block_api_type(row, row_id)?;
        if api_type.as_str() != "table_row" {
            return Err(format!(
                "Notion table {block_id} contains non-row child {row_id} of type {}",
                api_type.as_str()
            ));
        }
        let row_content = page_simple_table_row_block(
            context,
            SimpleTableRowInput {
                table_id: block_id,
                table: &table,
                row_id,
                row,
            },
            parsed,
        )?;
        let mut row_block =
            CardPageBlock::simple_table_row(row_id, block_id, depth + 1, row_content);
        row_block.color = page_block_color(row, row_id)?;
        row_block.last_edited = page_block_last_edited(row, context.users);
        parsed.blocks.push(row_block);
    }
    Ok(())
}

fn page_block_api_type(block: &Value, block_id: &str) -> Result<CardPageBlockApiType, String> {
    let api_type = block
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion block {block_id} has no string block type"))?;
    CardPageBlockApiType::new(api_type.to_string())
        .map_err(|error| format!("Notion block {block_id}: {error}"))
}
