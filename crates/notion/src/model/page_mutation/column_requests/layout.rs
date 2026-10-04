use crate::model::{
    CardPage, CardPageBlock, CardPageBlockContent, CardPageColumnEffectiveShare,
    CardPageColumnLayout, CardPageColumnLayoutEntry, CardPageColumnRatio,
    CardPageColumnWeightTotal, CardPageLayoutBlock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PageColumnWeightPair {
    left: CardPageColumnRatio,
    right: CardPageColumnRatio,
}

impl PageColumnWeightPair {
    pub(super) fn from_layout(left: CardPageColumnRatio, right: CardPageColumnRatio) -> Self {
        Self { left, right }
    }

    pub(crate) fn left(self) -> CardPageColumnRatio {
        self.left
    }

    pub(crate) fn right(self) -> CardPageColumnRatio {
        self.right
    }

    pub(super) fn resized_from_effective_shares(
        self,
        shares: PageColumnEffectiveSharePair,
        total: CardPageColumnWeightTotal,
    ) -> Result<Self, String> {
        let total = total.fraction();
        let requested_left = CardPageColumnRatio::from_fraction(shares.left().fraction() * total)?;
        CardPageColumnRatio::from_fraction(shares.right().fraction() * total)?;
        pair_preserving_total(self.total()?, requested_left.fraction())
    }

    pub(super) fn effective_shares(
        self,
        total: CardPageColumnWeightTotal,
    ) -> Result<PageColumnEffectiveSharePair, String> {
        PageColumnEffectiveSharePair::from_weights(self, total)
    }

    pub(crate) fn total(self) -> Result<f64, String> {
        let total = self.left.fraction() + self.right.fraction();
        if !total.is_finite() {
            return Err("adjacent Notion column weights have a non-finite total".to_string());
        }
        Ok(total)
    }
}

fn pair_preserving_total(
    source_total: f64,
    requested_left: f64,
) -> Result<PageColumnWeightPair, String> {
    // A rounded f64 product does not always have a rounded complement whose sum
    // reproduces the source total. Search the adjacent encodings deterministically.
    for left_fraction in neighboring_fractions(requested_left) {
        let Ok(left) = CardPageColumnRatio::from_fraction(left_fraction) else {
            continue;
        };
        let requested_right = source_total - left_fraction;
        if !requested_right.is_finite() || requested_right <= 0.0 {
            continue;
        }
        for right_fraction in neighboring_fractions(requested_right) {
            let Ok(right) = CardPageColumnRatio::from_fraction(right_fraction) else {
                continue;
            };
            if left.fraction() + right.fraction() == source_total {
                return Ok(PageColumnWeightPair { left, right });
            }
        }
    }
    Err("a column resize cannot preserve its adjacent source weight total".to_string())
}

fn neighboring_fractions(fraction: f64) -> [f64; 3] {
    [
        fraction,
        f64::from_bits(fraction.to_bits() - 1),
        f64::from_bits(fraction.to_bits() + 1),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PageColumnEffectiveSharePair {
    left: CardPageColumnEffectiveShare,
    right: CardPageColumnEffectiveShare,
}

impl PageColumnEffectiveSharePair {
    fn from_layout(
        left: CardPageColumnEffectiveShare,
        right: CardPageColumnEffectiveShare,
    ) -> Self {
        Self { left, right }
    }

    fn from_weights(
        weights: PageColumnWeightPair,
        total: CardPageColumnWeightTotal,
    ) -> Result<Self, String> {
        let total = total.fraction();
        Ok(Self {
            left: CardPageColumnEffectiveShare::from_fraction(weights.left().fraction() / total)?,
            right: CardPageColumnEffectiveShare::from_fraction(weights.right().fraction() / total)?,
        })
    }

    pub(crate) fn left(self) -> CardPageColumnEffectiveShare {
        self.left
    }

    pub(crate) fn right(self) -> CardPageColumnEffectiveShare {
        self.right
    }

    pub(super) fn resized_with_left(
        self,
        left: CardPageColumnEffectiveShare,
    ) -> Result<Self, String> {
        if left == self.left {
            return Err("a column resize must change the divider share".to_string());
        }
        let pair_total = self.left.fraction() + self.right.fraction();
        let right = CardPageColumnEffectiveShare::from_fraction(pair_total - left.fraction())
            .map_err(|_| {
                "a column resize must leave positive effective shares on both sides".to_string()
            })?;
        Ok(Self { left, right })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageColumnPair {
    column_list_block_id: String,
    left_column_block_id: String,
    right_column_block_id: String,
}

#[derive(Clone, Copy)]
pub(super) struct PageColumnBlockIndices {
    left: usize,
    right: usize,
}

impl PageColumnPair {
    pub(crate) fn new(
        page: &CardPage,
        column_list_block_id: impl Into<String>,
        left_column_block_id: impl Into<String>,
        right_column_block_id: impl Into<String>,
    ) -> Result<Self, String> {
        Self::resolve(
            page,
            column_list_block_id,
            left_column_block_id,
            right_column_block_id,
        )
        .map(|(target, _)| target)
    }

    pub(super) fn resolve<'a>(
        page: &'a CardPage,
        column_list_block_id: impl Into<String>,
        left_column_block_id: impl Into<String>,
        right_column_block_id: impl Into<String>,
    ) -> Result<(Self, ResolvedColumnLayout<'a>), String> {
        let target = Self {
            column_list_block_id: column_list_block_id.into(),
            left_column_block_id: left_column_block_id.into(),
            right_column_block_id: right_column_block_id.into(),
        };
        let layout = target.resolve_layout(page)?;
        Ok((target, layout))
    }

    pub(crate) fn column_list_block_id(&self) -> &str {
        &self.column_list_block_id
    }

    pub(crate) fn left_column_block_id(&self) -> &str {
        &self.left_column_block_id
    }

    pub(crate) fn right_column_block_id(&self) -> &str {
        &self.right_column_block_id
    }

    pub(crate) fn source_weights(&self, page: &CardPage) -> Result<PageColumnWeightPair, String> {
        Ok(self.resolve_layout(page)?.source_weights())
    }

    pub(crate) fn materialized_weights(
        &self,
        page: &CardPage,
    ) -> Result<PageColumnWeightPair, String> {
        let layout = self.resolve_layout(page)?;
        let left = layout.columns()[layout.left_index]
            .raw_weight()
            .ok_or_else(|| {
                format!(
                    "column {} does not have a materialized weight",
                    self.left_column_block_id
                )
            })?;
        let right = layout.columns()[layout.left_index + 1]
            .raw_weight()
            .ok_or_else(|| {
                format!(
                    "column {} does not have a materialized weight",
                    self.right_column_block_id
                )
            })?;
        Ok(PageColumnWeightPair::from_layout(left, right))
    }

    pub(crate) fn set_weights(
        &self,
        page: &mut CardPage,
        weights: PageColumnWeightPair,
    ) -> Result<(), String> {
        let (_, indices) = self.resolve_layout_with_indices(page)?;
        self.set_weights_at(page, indices, weights)
    }

    pub(crate) fn materialize_source_weights(&self, page: &mut CardPage) -> Result<(), String> {
        let (weights, indices) = {
            let (layout, indices) = self.resolve_layout_with_indices(page)?;
            (layout.source_weights(), indices)
        };
        self.set_weights_at(page, indices, weights)
    }

    pub(super) fn resolve_layout<'a>(
        &self,
        page: &'a CardPage,
    ) -> Result<ResolvedColumnLayout<'a>, String> {
        validate_target_ids(self)?;
        let layout = page.resolve_column_layout(&self.column_list_block_id)?;
        let left_index = layout
            .columns()
            .iter()
            .position(|column| column.block_id() == self.left_column_block_id)
            .ok_or_else(|| {
                format!(
                    "column list {} does not contain left column {}",
                    self.column_list_block_id, self.left_column_block_id
                )
            })?;
        if layout
            .columns()
            .get(left_index + 1)
            .map(|column| column.block_id())
            != Some(self.right_column_block_id.as_str())
        {
            return Err(format!(
                "columns {} and {} are not consecutive children of column list {}",
                self.left_column_block_id, self.right_column_block_id, self.column_list_block_id
            ));
        }
        Ok(ResolvedColumnLayout { layout, left_index })
    }

    pub(super) fn resolve_layout_with_indices<'a>(
        &self,
        page: &'a CardPage,
    ) -> Result<(ResolvedColumnLayout<'a>, PageColumnBlockIndices), String> {
        let layout = self.resolve_layout(page)?;
        let indices = column_block_indices(page, self)?;
        Ok((layout, indices))
    }

    pub(super) fn set_weights_at(
        &self,
        page: &mut CardPage,
        indices: PageColumnBlockIndices,
        weights: PageColumnWeightPair,
    ) -> Result<(), String> {
        let (left, right) = column_blocks_mut(page, indices)?;
        let left_ratio = column_ratio_slot(left, &self.left_column_block_id)?;
        let right_ratio = column_ratio_slot(right, &self.right_column_block_id)?;
        *left_ratio = Some(weights.left);
        *right_ratio = Some(weights.right);
        Ok(())
    }
}

pub(super) struct ResolvedColumnLayout<'a> {
    layout: CardPageColumnLayout<'a>,
    left_index: usize,
}

impl<'a> ResolvedColumnLayout<'a> {
    pub(super) fn columns(&self) -> &[CardPageColumnLayoutEntry<'a>] {
        self.layout.columns()
    }

    pub(super) fn weight_total(&self) -> CardPageColumnWeightTotal {
        self.layout.weight_total()
    }

    pub(super) fn source_weights(&self) -> PageColumnWeightPair {
        PageColumnWeightPair::from_layout(
            self.columns()[self.left_index].weight(),
            self.columns()[self.left_index + 1].weight(),
        )
    }

    pub(super) fn effective_shares(&self) -> PageColumnEffectiveSharePair {
        PageColumnEffectiveSharePair::from_layout(
            self.columns()[self.left_index].effective_share(),
            self.columns()[self.left_index + 1].effective_share(),
        )
    }
}

fn validate_target_ids(target: &PageColumnPair) -> Result<(), String> {
    if target.column_list_block_id.is_empty()
        || target.left_column_block_id.is_empty()
        || target.right_column_block_id.is_empty()
    {
        return Err("a column resize requires non-empty block IDs".to_string());
    }
    if target.left_column_block_id == target.right_column_block_id {
        return Err("a column resize requires two distinct columns".to_string());
    }
    Ok(())
}

fn column_block_indices(
    page: &CardPage,
    target: &PageColumnPair,
) -> Result<PageColumnBlockIndices, String> {
    let mut left = None;
    let mut right = None;
    for (index, block) in page.blocks.iter().enumerate() {
        if left.is_none() && block.block_id == target.left_column_block_id {
            left = Some(index);
        } else if right.is_none() && block.block_id == target.right_column_block_id {
            right = Some(index);
        }
    }
    Ok(PageColumnBlockIndices {
        left: left.ok_or_else(|| {
            format!(
                "page {} does not contain column {}",
                page.block_id, target.left_column_block_id
            )
        })?,
        right: right.ok_or_else(|| {
            format!(
                "page {} does not contain column {}",
                page.block_id, target.right_column_block_id
            )
        })?,
    })
}

fn column_blocks_mut(
    page: &mut CardPage,
    indices: PageColumnBlockIndices,
) -> Result<(&mut CardPageBlock, &mut CardPageBlock), String> {
    if indices.left < indices.right {
        let (before_right, from_right) = page.blocks.split_at_mut(indices.right);
        return Ok((&mut before_right[indices.left], &mut from_right[0]));
    }
    if indices.right < indices.left {
        let (before_left, from_left) = page.blocks.split_at_mut(indices.left);
        return Ok((&mut from_left[0], &mut before_left[indices.right]));
    }
    Err("a column resize resolved both columns to the same page block".to_string())
}

fn column_ratio_slot<'a>(
    block: &'a mut CardPageBlock,
    block_id: &str,
) -> Result<&'a mut Option<CardPageColumnRatio>, String> {
    if block.block_id != block_id {
        return Err(format!(
            "column index no longer identifies block {block_id}"
        ));
    }
    let CardPageBlockContent::Layout(CardPageLayoutBlock::Column { ratio: target }) =
        &mut block.content
    else {
        return Err(format!("block {block_id} is not a Notion column"));
    };
    Ok(target)
}
