use std::collections::HashMap;

use super::{CardPage, CardPageColumnRatio, CardPageLayoutBlock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CardPageColumnEffectiveShare(CardPageColumnRatio);

impl CardPageColumnEffectiveShare {
    pub(crate) fn from_fraction(fraction: f64) -> Result<Self, String> {
        if fraction > 1.0 {
            return Err(format!(
                "effective Notion column share must not exceed 1, received {fraction}"
            ));
        }
        CardPageColumnRatio::from_fraction(fraction).map(Self)
    }

    pub(crate) fn fraction(self) -> f64 {
        self.0.fraction()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CardPageColumnWeightTotal(CardPageColumnRatio);

impl CardPageColumnWeightTotal {
    pub(crate) fn fraction(self) -> f64 {
        self.0.fraction()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CardPageColumnLayoutEntry<'a> {
    block_id: &'a str,
    raw_weight: Option<CardPageColumnRatio>,
    weight: CardPageColumnRatio,
    effective_share: CardPageColumnEffectiveShare,
}

impl<'a> CardPageColumnLayoutEntry<'a> {
    pub(crate) fn block_id(self) -> &'a str {
        self.block_id
    }

    pub(crate) fn raw_weight(self) -> Option<CardPageColumnRatio> {
        self.raw_weight
    }

    pub(crate) fn weight(self) -> CardPageColumnRatio {
        self.weight
    }

    pub(crate) fn effective_share(self) -> CardPageColumnEffectiveShare {
        self.effective_share
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardPageColumnLayout<'a> {
    columns: Vec<CardPageColumnLayoutEntry<'a>>,
    weight_total: CardPageColumnWeightTotal,
}

pub(crate) enum CardPageColumnLayoutRead<'a> {
    Empty,
    Columns(CardPageColumnLayout<'a>),
}

pub(crate) struct CardPageColumnLayoutIndex<'a> {
    layouts: HashMap<&'a str, CardPageColumnLayoutRead<'a>>,
}

impl<'a> CardPageColumnLayoutIndex<'a> {
    fn build(page: &'a CardPage) -> Result<Self, String> {
        let mut raw_columns_by_list = HashMap::<&'a str, Vec<RawColumn<'a>>>::new();
        for block in &page.blocks {
            if block.layout_content() == Some(&CardPageLayoutBlock::ColumnList)
                && raw_columns_by_list
                    .insert(block.block_id.as_str(), Vec::new())
                    .is_some()
            {
                return Err(format!(
                    "page {} contains duplicate column-list identity {}",
                    page.block_id, block.block_id
                ));
            }
        }
        for block in &page.blocks {
            let Some(columns) = raw_columns_by_list.get_mut(block.parent_block_id.as_str()) else {
                continue;
            };
            if block.is_opaque_unavailable() {
                continue;
            }
            let Some(CardPageLayoutBlock::Column { ratio }) = block.layout_content() else {
                return Err(format!(
                    "column list {} contains non-column child {}",
                    block.parent_block_id, block.block_id
                ));
            };
            columns.push((block.block_id.as_str(), *ratio));
        }
        let layouts = raw_columns_by_list
            .into_iter()
            .map(|(column_list_id, raw_columns)| {
                let layout = if raw_columns.is_empty() {
                    Ok(CardPageColumnLayoutRead::Empty)
                } else {
                    resolve_column_layout(raw_columns, column_list_id)
                        .map(CardPageColumnLayoutRead::Columns)
                }?;
                Ok::<_, String>((column_list_id, layout))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self { layouts })
    }

    pub(crate) fn layout(
        &self,
        column_list_block_id: &str,
    ) -> Result<&CardPageColumnLayoutRead<'a>, String> {
        self.layouts.get(column_list_block_id).ok_or_else(|| {
            format!("column-list layout index does not contain {column_list_block_id}")
        })
    }
}

impl<'a> CardPageColumnLayout<'a> {
    pub(crate) fn columns(&self) -> &[CardPageColumnLayoutEntry<'a>] {
        &self.columns
    }

    pub(crate) fn weight_total(&self) -> CardPageColumnWeightTotal {
        self.weight_total
    }
}

impl CardPage {
    pub(crate) fn column_layout_index(&self) -> Result<CardPageColumnLayoutIndex<'_>, String> {
        CardPageColumnLayoutIndex::build(self)
    }

    /// Resolves persisted and inferred column weights into effective layout shares.
    /// Missing weights are an unmeasured, layout-only inference; resolving or rendering
    /// this value must not persist them.
    pub(crate) fn resolve_column_layout(
        &self,
        column_list_block_id: &str,
    ) -> Result<CardPageColumnLayout<'_>, String> {
        let column_list = self
            .blocks
            .iter()
            .find(|block| block.block_id == column_list_block_id)
            .ok_or_else(|| {
                format!(
                    "page {} does not contain block {column_list_block_id}",
                    self.block_id
                )
            })?;
        if column_list.layout_content() != Some(&CardPageLayoutBlock::ColumnList) {
            return Err(format!(
                "block {column_list_block_id} is not a Notion column list"
            ));
        }
        let raw_columns = self
            .blocks
            .iter()
            .filter(|block| block.parent_block_id == column_list_block_id)
            .map(|block| {
                let Some(CardPageLayoutBlock::Column { ratio }) = block.layout_content() else {
                    return Err(format!(
                        "column list {column_list_block_id} contains non-column child {}",
                        block.block_id
                    ));
                };
                Ok((block.block_id.as_str(), *ratio))
            })
            .collect::<Result<Vec<_>, _>>()?;
        resolve_column_layout(raw_columns, column_list_block_id)
    }

    pub(crate) fn column_layout_effective_geometry_eq(
        &self,
        other: &Self,
        column_list_block_id: &str,
    ) -> Result<bool, String> {
        let left = self.resolve_column_layout(column_list_block_id)?;
        let right = other.resolve_column_layout(column_list_block_id)?;
        Ok(left
            .columns()
            .iter()
            .map(|column| (column.block_id(), column.effective_share()))
            .eq(right
                .columns()
                .iter()
                .map(|column| (column.block_id(), column.effective_share()))))
    }
}

/// A column's block ID and its persisted ratio, when Notion stores one.
type RawColumn<'a> = (&'a str, Option<CardPageColumnRatio>);

fn resolve_column_layout<'a>(
    raw_columns: Vec<RawColumn<'a>>,
    column_list_block_id: &str,
) -> Result<CardPageColumnLayout<'a>, String> {
    if raw_columns.is_empty() {
        return Err(format!(
            "column list {column_list_block_id} does not contain any columns"
        ));
    }
    let raw_weights = raw_columns
        .iter()
        .map(|(_, ratio)| *ratio)
        .collect::<Vec<_>>();
    let weights = inferred_column_weights(&raw_weights)?;
    let weight_total = finite_weight_sum(&weights)?;
    if weight_total <= 0.0 {
        return Err("Notion column layout has a non-positive total weight".to_string());
    }
    let columns = raw_columns
        .into_iter()
        .zip(weights)
        .map(|((block_id, raw_weight), weight)| {
            let effective_share =
                CardPageColumnEffectiveShare::from_fraction(weight.fraction() / weight_total)?;
            Ok(CardPageColumnLayoutEntry {
                block_id,
                raw_weight,
                weight,
                effective_share,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CardPageColumnLayout {
        columns,
        weight_total: CardPageColumnWeightTotal(CardPageColumnRatio::from_fraction(weight_total)?),
    })
}

fn inferred_column_weights(
    raw_weights: &[Option<CardPageColumnRatio>],
) -> Result<Vec<CardPageColumnRatio>, String> {
    let missing_count = raw_weights.iter().filter(|weight| weight.is_none()).count();
    let known_weights = raw_weights.iter().flatten().copied().collect::<Vec<_>>();
    let known_sum = finite_weight_sum(&known_weights)?;
    let inferred_missing = if missing_count == 0 {
        None
    } else if known_sum < 1.0 {
        Some((1.0 - known_sum) / missing_count as f64)
    } else {
        Some(1.0 / raw_weights.len() as f64)
    };
    raw_weights
        .iter()
        .map(|weight| match weight {
            Some(weight) => Ok(*weight),
            None => {
                let inferred = inferred_missing.ok_or_else(|| {
                    "missing Notion column weight has no layout inference".to_string()
                })?;
                CardPageColumnRatio::from_fraction(inferred)
            }
        })
        .collect()
}

fn finite_weight_sum(weights: &[CardPageColumnRatio]) -> Result<f64, String> {
    let total = weights
        .iter()
        .fold(0.0, |sum, weight| sum + weight.fraction());
    if !total.is_finite() {
        return Err("Notion column layout has a non-finite total weight".to_string());
    }
    Ok(total)
}
