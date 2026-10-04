use std::collections::HashMap;

use crate::model::{
    CardPage, CardPageBlock, CardPageBlockContent, CardPageLayoutBlock, PageColumnPair,
    PageColumnWeightPair,
};

pub(super) fn validate_whole_page_column_history_compatibility(
    current: &CardPage,
    target: &CardPage,
) -> Result<(), String> {
    ColumnHistoryIndex::new(current, target)?.validate()
}

pub(super) fn rebase_whole_page_column_history_target(
    current: &CardPage,
    target: &CardPage,
) -> Result<CardPage, String> {
    ColumnHistoryIndex::new(current, target)?.rebase_target()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnWrapperKind {
    List,
    Column,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ColumnWrapperSignature<'a> {
    parent_block_id: &'a str,
    depth: usize,
    kind: ColumnWrapperKind,
}

#[derive(Clone, Copy)]
struct IndexedHistoryBlock<'a> {
    block: &'a CardPageBlock,
    column_signature: Option<ColumnWrapperSignature<'a>>,
}

struct ColumnHistoryPageIndex<'a> {
    page: &'a CardPage,
    blocks: HashMap<&'a str, IndexedHistoryBlock<'a>>,
}

struct ColumnHistoryIndex<'a> {
    current: ColumnHistoryPageIndex<'a>,
    target: ColumnHistoryPageIndex<'a>,
}

impl<'a> ColumnHistoryPageIndex<'a> {
    fn new(page: &'a CardPage) -> Self {
        let mut blocks = HashMap::with_capacity(page.blocks.len());
        for block in &page.blocks {
            blocks
                .entry(block.block_id.as_str())
                .or_insert_with(|| IndexedHistoryBlock {
                    block,
                    column_signature: column_wrapper_signature(block),
                });
        }
        Self { page, blocks }
    }
}

impl<'a> ColumnHistoryIndex<'a> {
    fn new(current: &'a CardPage, target: &'a CardPage) -> Result<Self, String> {
        if current.block_id != target.block_id {
            return Err("history snapshot belongs to a different page".to_string());
        }
        Ok(Self {
            current: ColumnHistoryPageIndex::new(current),
            target: ColumnHistoryPageIndex::new(target),
        })
    }

    fn validate(&self) -> Result<(), String> {
        validate_matching_column_wrappers(&self.current, &self.target)?;
        validate_matching_column_wrappers(&self.target, &self.current)
    }

    fn rebase_target(&self) -> Result<CardPage, String> {
        self.validate()?;
        let mut rebased = self.target.page.clone();
        rebase_column_ratios(&self.current, &mut rebased)?;
        Ok(rebased)
    }
}

fn validate_matching_column_wrappers(
    expected: &ColumnHistoryPageIndex<'_>,
    actual: &ColumnHistoryPageIndex<'_>,
) -> Result<(), String> {
    for block in &expected.page.blocks {
        let Some(expected_signature) = column_wrapper_signature(block) else {
            continue;
        };
        let matches = actual
            .blocks
            .get(block.block_id.as_str())
            .is_some_and(|actual| actual.column_signature == Some(expected_signature));
        if !matches {
            return Err(format!(
                "history changed the identity, parent, depth, or type of column wrapper {}",
                block.block_id
            ));
        }
    }
    Ok(())
}

fn column_wrapper_signature(block: &CardPageBlock) -> Option<ColumnWrapperSignature<'_>> {
    Some(ColumnWrapperSignature {
        parent_block_id: &block.parent_block_id,
        depth: block.depth,
        kind: column_wrapper_kind(&block.content)?,
    })
}

fn column_wrapper_kind(content: &CardPageBlockContent) -> Option<ColumnWrapperKind> {
    match content {
        CardPageBlockContent::Layout(CardPageLayoutBlock::ColumnList) => {
            Some(ColumnWrapperKind::List)
        }
        CardPageBlockContent::Layout(CardPageLayoutBlock::Column { .. }) => {
            Some(ColumnWrapperKind::Column)
        }
        _ => None,
    }
}

pub(super) enum PreparedColumnRatioHistory {
    KeepCurrent,
    Apply(PageColumnWeightPair),
}

pub(super) fn prepare_column_ratio_history(
    current: &CardPage,
    target: &CardPage,
    pair: &PageColumnPair,
) -> Result<PreparedColumnRatioHistory, String> {
    if current.block_id != target.block_id {
        return Err("history snapshot belongs to a different page".to_string());
    }
    if current.column_layout_effective_geometry_eq(target, pair.column_list_block_id())? {
        return Ok(PreparedColumnRatioHistory::KeepCurrent);
    }
    let target_weights = materialized_weights(target, pair, "history")?;
    let current_weights = pair.source_weights(current)?;
    validate_same_pair_total(current_weights, target_weights)?;
    Ok(PreparedColumnRatioHistory::Apply(target_weights))
}

pub(super) fn rebase_column_ratio_history_target(
    current: &CardPage,
    pair: &PageColumnPair,
    prepared: PreparedColumnRatioHistory,
) -> Result<CardPage, String> {
    let PreparedColumnRatioHistory::Apply(target_weights) = prepared else {
        return Ok(current.clone());
    };
    let mut rebased = current.clone();
    pair.set_weights(&mut rebased, target_weights)?;
    Ok(rebased)
}

fn rebase_column_ratios(
    current: &ColumnHistoryPageIndex<'_>,
    target: &mut CardPage,
) -> Result<(), String> {
    for target_block in &mut target.blocks {
        let CardPageBlockContent::Layout(CardPageLayoutBlock::Column {
            ratio: target_ratio,
        }) = &mut target_block.content
        else {
            continue;
        };
        let current_block = current
            .blocks
            .get(target_block.block_id.as_str())
            .ok_or_else(|| {
                format!(
                    "history column {} disappeared during rebase",
                    target_block.block_id
                )
            })?
            .block;
        let CardPageBlockContent::Layout(CardPageLayoutBlock::Column {
            ratio: current_ratio,
        }) = &current_block.content
        else {
            return Err(format!(
                "history column {} changed type during rebase",
                target_block.block_id
            ));
        };
        target_ratio.clone_from(current_ratio);
    }
    Ok(())
}

fn materialized_weights(
    page: &CardPage,
    pair: &PageColumnPair,
    side: &str,
) -> Result<PageColumnWeightPair, String> {
    pair.materialized_weights(page).map_err(|error| {
        format!("{side} column ratio history does not have materialized weights: {error}")
    })
}

fn validate_same_pair_total(
    current: PageColumnWeightPair,
    target: PageColumnWeightPair,
) -> Result<(), String> {
    if current.total()? != target.total()? {
        return Err("column ratio history changed the adjacent pair's total weight".to_string());
    }
    Ok(())
}
