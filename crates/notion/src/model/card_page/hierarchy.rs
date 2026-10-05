use std::collections::HashMap;

use super::{CardPage, CardPageBlock, CardPageLayoutBlock};

impl CardPage {
    pub(crate) fn validate_block_hierarchy(&self) -> Result<(), String> {
        let mut preceding_blocks: HashMap<&str, &CardPageBlock> =
            HashMap::with_capacity(self.blocks.len());
        let mut table_row_counts: HashMap<&str, usize> = HashMap::new();
        for block in &self.blocks {
            if block.block_id == self.block_id {
                return Err(format!(
                    "page {} contains its root as a content block",
                    self.block_id
                ));
            }
            if preceding_blocks.contains_key(block.block_id.as_str()) {
                return Err(format!(
                    "page {} contains duplicate block {}",
                    self.block_id, block.block_id
                ));
            }

            let parent = if block.parent_block_id == self.block_id {
                if block.depth != 0 {
                    return Err(format!(
                        "root child {} has depth {}, expected 0",
                        block.block_id, block.depth
                    ));
                }
                None
            } else {
                let parent = *preceding_blocks
                    .get(block.parent_block_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "block {} references missing or following parent {}",
                            block.block_id, block.parent_block_id
                        )
                    })?;
                let expected_depth = parent
                    .depth
                    .checked_add(1)
                    .ok_or_else(|| format!("page block {} depth exceeds usize", block.block_id))?;
                if block.depth != expected_depth {
                    return Err(format!(
                        "block {} has depth {}, expected {} below parent {}",
                        block.block_id, block.depth, expected_depth, block.parent_block_id
                    ));
                }
                Some(parent)
            };

            validate_block_parent(block, parent, &mut table_row_counts)?;
            preceding_blocks.insert(block.block_id.as_str(), block);
        }
        validate_table_row_counts(&self.blocks, &table_row_counts)
    }
}

fn validate_table_row_counts(
    blocks: &[CardPageBlock],
    table_row_counts: &HashMap<&str, usize>,
) -> Result<(), String> {
    for table_block in blocks
        .iter()
        .filter(|block| block.simple_table_content().is_some())
    {
        let table = table_block
            .simple_table_content()
            .expect("filtered Notion table block must contain a table");
        let row_count = table_row_counts
            .get(table_block.block_id.as_str())
            .copied()
            .unwrap_or_default();
        if row_count != table.row_block_ids().len() {
            return Err(format!(
                "Notion table {} contains {row_count} rows, expected {}",
                table_block.block_id,
                table.row_block_ids().len()
            ));
        }
    }
    Ok(())
}

fn validate_block_parent<'a>(
    block: &'a CardPageBlock,
    parent: Option<&'a CardPageBlock>,
    table_row_counts: &mut HashMap<&'a str, usize>,
) -> Result<(), String> {
    if validate_table_child(block, parent, table_row_counts)? {
        return Ok(());
    }
    if block.is_opaque_unavailable() {
        return Ok(());
    }
    match (block.layout_content(), parent) {
        (Some(CardPageLayoutBlock::Column { .. }), Some(parent))
            if matches!(
                parent.layout_content(),
                Some(CardPageLayoutBlock::ColumnList)
            ) => {}
        (Some(CardPageLayoutBlock::Column { .. }), _) => {
            return Err(format!(
                "column block {} must be a direct child of a column list",
                block.block_id
            ));
        }
        (_, Some(parent))
            if matches!(
                parent.layout_content(),
                Some(CardPageLayoutBlock::ColumnList)
            ) =>
        {
            return Err(format!(
                "column list {} contains non-column child {}",
                parent.block_id, block.block_id
            ));
        }
        _ => {}
    }
    if let Some(parent) = parent {
        let parent_is_column_list = matches!(
            parent.layout_content(),
            Some(CardPageLayoutBlock::ColumnList)
        );
        if !parent_is_column_list && !parent.accepts_content_children() {
            return Err(format!(
                "page block {} cannot own child {}",
                parent.block_id, block.block_id
            ));
        }
    }
    Ok(())
}

fn validate_table_child<'a>(
    block: &'a CardPageBlock,
    parent: Option<&'a CardPageBlock>,
    table_row_counts: &mut HashMap<&'a str, usize>,
) -> Result<bool, String> {
    if let Some(row) = block.simple_table_row_content() {
        let Some(parent) = parent else {
            return Err(format!(
                "Notion table row {} must be a direct child of a table",
                block.block_id
            ));
        };
        let Some(table) = parent.simple_table_content() else {
            return Err(format!(
                "Notion table row {} must be a direct child of a table",
                block.block_id
            ));
        };
        record_table_child(block, parent, table, table_row_counts)?;
        if row.cells().len() != table.columns().len() {
            return Err(format!(
                "Notion table row {} contains {} cells, expected {}",
                block.block_id,
                row.cells().len(),
                table.columns().len()
            ));
        }
        return Ok(true);
    }
    if let Some(parent) = parent {
        if let Some(table) = parent.simple_table_content() {
            if block.is_opaque_unavailable() {
                record_table_child(block, parent, table, table_row_counts)?;
                return Ok(true);
            }
            return Err(format!(
                "Notion table {} contains non-row child {}",
                parent.block_id, block.block_id
            ));
        }
        if parent.simple_table_row_content().is_some() {
            return Err(format!(
                "Notion table row {} cannot own child {}",
                parent.block_id, block.block_id
            ));
        }
    }
    Ok(false)
}

fn record_table_child<'a>(
    block: &'a CardPageBlock,
    parent: &'a CardPageBlock,
    table: &super::CardPageSimpleTableBlock,
    table_row_counts: &mut HashMap<&'a str, usize>,
) -> Result<(), String> {
    let row_index = table_row_counts
        .entry(parent.block_id.as_str())
        .or_default();
    if table.row_block_ids().get(*row_index).map(String::as_str) != Some(block.block_id.as_str()) {
        return Err(format!(
            "Notion table {} contains unexpected child {} at position {}",
            parent.block_id, block.block_id, row_index
        ));
    }
    *row_index += 1;
    Ok(())
}
