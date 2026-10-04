use std::{ops::Range, sync::Arc};

use gpui::SharedString;

use crate::model::{CardPageSimpleTableCellAddress, PageTextColor};

use super::LoadedCardPageData;

#[derive(Clone, Debug)]
pub(crate) struct PageDocumentUnitLayoutRevision(Arc<PageDocumentUnitLayoutRevisionIdentity>);

#[derive(Debug)]
struct PageDocumentUnitLayoutRevisionIdentity;

impl PageDocumentUnitLayoutRevision {
    pub(super) fn fresh() -> Self {
        Self(Arc::new(PageDocumentUnitLayoutRevisionIdentity))
    }
}

impl PartialEq for PageDocumentUnitLayoutRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PageDocumentUnitLayoutRevision {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageDocumentUnitKey {
    Block {
        block_id: Arc<str>,
    },
    SimpleTableRow {
        table_block_id: Arc<str>,
        row_block_id: Arc<str>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadedCardPageSimpleTableCellAccess {
    Writable,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoadedCardPageSimpleTableCellLocation {
    pub(crate) document_unit_index: usize,
    pub(crate) row_ordinal: usize,
    pub(crate) column_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoadedCardPageSimpleTableRowPosition {
    Only,
    First,
    Middle,
    Last,
}

impl LoadedCardPageSimpleTableRowPosition {
    pub(crate) const fn is_first(self) -> bool {
        matches!(self, Self::Only | Self::First)
    }

    pub(crate) const fn is_last(self) -> bool {
        matches!(self, Self::Only | Self::Last)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedCardPageSimpleTableAnnotationRun {
    pub(crate) range: Range<usize>,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) code: bool,
    pub(crate) link: bool,
    pub(crate) text_color: Option<PageTextColor>,
    pub(crate) background_color: Option<PageTextColor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedCardPageSimpleTableCell {
    pub(crate) address: CardPageSimpleTableCellAddress,
    pub(crate) access: LoadedCardPageSimpleTableCellAccess,
    pub(crate) text: SharedString,
    pub(crate) annotation_runs: Arc<[LoadedCardPageSimpleTableAnnotationRun]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoadedCardPageDocumentUnit {
    Block {
        key: PageDocumentUnitKey,
        owner_visible_row_index: usize,
    },
    SimpleTableRow {
        key: PageDocumentUnitKey,
        owner_visible_row_index: usize,
        row_block_index: usize,
        ordinal: usize,
        position: LoadedCardPageSimpleTableRowPosition,
        cells: Arc<[LoadedCardPageSimpleTableCell]>,
    },
}

impl LoadedCardPageDocumentUnit {
    pub(crate) const fn key(&self) -> &PageDocumentUnitKey {
        match self {
            Self::Block { key, .. } | Self::SimpleTableRow { key, .. } => key,
        }
    }

    pub(crate) const fn owner_visible_row_index(&self) -> usize {
        match self {
            Self::Block {
                owner_visible_row_index,
                ..
            }
            | Self::SimpleTableRow {
                owner_visible_row_index,
                ..
            } => *owner_visible_row_index,
        }
    }

    pub(crate) const fn is_simple_table_row(&self) -> bool {
        matches!(self, Self::SimpleTableRow { .. })
    }
}

impl LoadedCardPageData {
    pub(crate) fn simple_table_cell_location(
        &self,
        address: &CardPageSimpleTableCellAddress,
    ) -> Option<LoadedCardPageSimpleTableCellLocation> {
        self.simple_table_cell_locations.get(address).copied()
    }

    pub(crate) fn projected_simple_table_cell(
        &self,
        address: &CardPageSimpleTableCellAddress,
    ) -> Option<&LoadedCardPageSimpleTableCell> {
        let location = self.simple_table_cell_location(address)?;
        let LoadedCardPageDocumentUnit::SimpleTableRow { cells, .. } =
            self.document_units.get(location.document_unit_index)?
        else {
            return None;
        };
        cells.get(location.column_index)
    }
}
