use super::super::{
    CardPage, CardPageColumnEffectiveShare, CardPageColumnRatio, CardPageColumnWeightTotal,
};

mod layout;

use layout::{PageColumnBlockIndices, PageColumnEffectiveSharePair};
pub(crate) use layout::{PageColumnPair, PageColumnWeightPair};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageColumnRatioAuthority {
    column_block_id: String,
    raw_weight: Option<CardPageColumnRatio>,
}

impl PageColumnRatioAuthority {
    pub(crate) fn column_block_id(&self) -> &str {
        &self.column_block_id
    }

    pub(crate) fn raw_weight(&self) -> Option<CardPageColumnRatio> {
        self.raw_weight
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResizePageColumnsSource {
    columns: Vec<PageColumnRatioAuthority>,
    weights: PageColumnWeightPair,
    effective_shares: PageColumnEffectiveSharePair,
    weight_total: CardPageColumnWeightTotal,
}

#[derive(Clone, Debug)]
pub struct ResizePageColumnsRequest {
    page_block_id: String,
    target: PageColumnPair,
    source: ResizePageColumnsSource,
    destination_weights: PageColumnWeightPair,
}

impl ResizePageColumnsRequest {
    pub(crate) fn new(
        page: &CardPage,
        column_list_block_id: impl Into<String>,
        left_column_block_id: impl Into<String>,
        right_column_block_id: impl Into<String>,
        destination_left_share: CardPageColumnEffectiveShare,
    ) -> Result<Self, String> {
        let (target, source) = resolve_source(
            page,
            column_list_block_id,
            left_column_block_id,
            right_column_block_id,
        )?;
        let requested_shares = source
            .effective_shares
            .resized_with_left(destination_left_share)?;
        let destination_weights = source
            .weights
            .resized_from_effective_shares(requested_shares, source.weight_total)?;
        Self::from_parts(page, target, source, destination_weights)
    }

    pub(crate) fn from_materialized_destination(
        page: &CardPage,
        destination_page: &CardPage,
        column_list_block_id: impl Into<String>,
        left_column_block_id: impl Into<String>,
        right_column_block_id: impl Into<String>,
    ) -> Result<Self, String> {
        if page.block_id != destination_page.block_id {
            return Err("column resize destination belongs to a different page".to_string());
        }
        let (target, source) = resolve_source(
            page,
            column_list_block_id,
            left_column_block_id,
            right_column_block_id,
        )?;
        let destination_weights = target.materialized_weights(destination_page)?;
        if destination_weights.total()? != source.weights.total()? {
            return Err(
                "a column resize must preserve the adjacent pair's total weight".to_string(),
            );
        }
        Self::from_parts(page, target, source, destination_weights)
    }

    pub(crate) fn page_block_id(&self) -> &str {
        &self.page_block_id
    }

    pub(crate) fn target(&self) -> &PageColumnPair {
        &self.target
    }

    pub(crate) fn source_columns(&self) -> &[PageColumnRatioAuthority] {
        &self.source.columns
    }

    pub(crate) fn destination_weights(&self) -> PageColumnWeightPair {
        self.destination_weights
    }

    pub(crate) fn validate_source_on(&self, page: &CardPage) -> Result<(), String> {
        self.resolve_source_on(page).map(|_| ())
    }

    fn resolve_source_on(&self, page: &CardPage) -> Result<PageColumnBlockIndices, String> {
        if page.block_id != self.page_block_id {
            return Err(format!(
                "column resize belongs to page {}, received {}",
                self.page_block_id, page.block_id
            ));
        }
        let (layout, indices) = self.target.resolve_layout_with_indices(page)?;
        let unchanged = layout
            .columns()
            .iter()
            .map(|column| (column.block_id(), column.raw_weight()))
            .eq(self
                .source
                .columns
                .iter()
                .map(|column| (column.column_block_id.as_str(), column.raw_weight)));
        if !unchanged {
            return Err(format!(
                "column list {} changed since the resize began",
                self.target.column_list_block_id()
            ));
        }
        Ok(indices)
    }

    pub(crate) fn apply_to_page(&self, page: &mut CardPage) -> Result<(), String> {
        let indices = self.resolve_source_on(page)?;
        self.target
            .set_weights_at(page, indices, self.destination_weights)
    }

    pub(crate) fn same_divider(&self, other: &Self) -> bool {
        self.page_block_id == other.page_block_id && self.target == other.target
    }

    pub(crate) fn same_column_list(&self, other: &Self) -> bool {
        self.page_block_id == other.page_block_id
            && self.target.column_list_block_id() == other.target.column_list_block_id()
    }

    pub(crate) fn coalesced_with(&self, next: &Self) -> Result<Option<Self>, String> {
        if !self.same_divider(next) {
            return Err("column resize writes target different dividers".to_string());
        }
        let mut expected_columns = self.source.columns.clone();
        update_pair_authority(
            &mut expected_columns,
            &self.target,
            self.destination_weights,
        );
        if next.source.columns != expected_columns {
            return Err("column resize writes do not form a continuous source chain".to_string());
        }
        if self.raw_source_weights() == Some(next.destination_weights) {
            return Ok(None);
        }
        if next.destination_weights.total()? != self.source.weights.total()? {
            return Err(
                "coalesced column resize changed the adjacent pair's total weight".to_string(),
            );
        }
        next.destination_weights
            .effective_shares(self.source.weight_total)?;
        let mut coalesced = self.clone();
        coalesced.destination_weights = next.destination_weights;
        Ok(Some(coalesced))
    }

    fn raw_source_weights(&self) -> Option<PageColumnWeightPair> {
        let left = self
            .source
            .columns
            .iter()
            .find(|column| column.column_block_id == self.target.left_column_block_id())?
            .raw_weight?;
        let right = self
            .source
            .columns
            .iter()
            .find(|column| column.column_block_id == self.target.right_column_block_id())?
            .raw_weight?;
        Some(PageColumnWeightPair::from_layout(left, right))
    }

    fn from_parts(
        page: &CardPage,
        target: PageColumnPair,
        source: ResizePageColumnsSource,
        destination_weights: PageColumnWeightPair,
    ) -> Result<Self, String> {
        let destination_shares = destination_weights.effective_shares(source.weight_total)?;
        if destination_weights == source.weights || destination_shares == source.effective_shares {
            return Err("a column resize must change the effective divider share".to_string());
        }
        Ok(Self {
            page_block_id: page.block_id.clone(),
            target,
            source,
            destination_weights,
        })
    }
}

fn resolve_source(
    page: &CardPage,
    column_list_block_id: impl Into<String>,
    left_column_block_id: impl Into<String>,
    right_column_block_id: impl Into<String>,
) -> Result<(PageColumnPair, ResizePageColumnsSource), String> {
    let (target, layout) = PageColumnPair::resolve(
        page,
        column_list_block_id,
        left_column_block_id,
        right_column_block_id,
    )?;
    let source = ResizePageColumnsSource {
        columns: layout
            .columns()
            .iter()
            .map(|column| PageColumnRatioAuthority {
                column_block_id: column.block_id().to_string(),
                raw_weight: column.raw_weight(),
            })
            .collect(),
        weights: layout.source_weights(),
        effective_shares: layout.effective_shares(),
        weight_total: layout.weight_total(),
    };
    Ok((target, source))
}

fn update_pair_authority(
    columns: &mut [PageColumnRatioAuthority],
    target: &PageColumnPair,
    weights: PageColumnWeightPair,
) {
    for column in columns {
        if column.column_block_id == target.left_column_block_id() {
            column.raw_weight = Some(weights.left());
        } else if column.column_block_id == target.right_column_block_id() {
            column.raw_weight = Some(weights.right());
        }
    }
}
