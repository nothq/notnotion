use gpui_components::text_input::TextInputSnapshot;

use crate::model::{CardPage, CardPageSimpleTableCellAddress, CardPageWritableSimpleTableCell};
use crate::ui::surface::{PageSimpleTableCellGeneration, PageSimpleTableCellObservedReplacement};

use super::composition::{SimpleTableCompositionLifecycle, SimpleTableCompositionTarget};

#[derive(Clone, Copy)]
pub(super) struct SimpleTableCellChangeTarget<'a> {
    pub(super) page_id: &'a str,
    pub(super) address: &'a CardPageSimpleTableCellAddress,
    pub(super) generation: PageSimpleTableCellGeneration,
    pub(super) snapshot: &'a TextInputSnapshot,
}

impl SimpleTableCellChangeTarget<'_> {
    pub(super) fn composition<'a>(
        &'a self,
        replacement: Option<&'a PageSimpleTableCellObservedReplacement>,
    ) -> SimpleTableCompositionTarget<'a> {
        SimpleTableCompositionTarget {
            address: self.address,
            generation: self.generation,
            snapshot: self.snapshot,
            replacement,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SimpleTableCellChangeAttempt {
    Ordinary,
    CompositionCommit,
}

pub(super) struct PreparedSimpleTableCellChange {
    pub(super) visual_page: CardPage,
    pub(super) replacement: Option<PageSimpleTableCellObservedReplacement>,
    pub(super) target: CardPageWritableSimpleTableCell,
    pub(super) lifecycle: SimpleTableCompositionLifecycle,
}
