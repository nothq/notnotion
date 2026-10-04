use crate::ui::{LoadedCardPageData, PageTextSelection};

/// A selected text range within one block, and whether the selection is reversed.
pub(in crate::ui::board_workspace::page::editor) type DirectedTextRange =
    (std::ops::Range<usize>, bool);

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page::editor) struct VisiblePageTextSelection<'a> {
    data: &'a LoadedCardPageData,
    selection: &'a PageTextSelection,
    anchor_index: usize,
    focus_index: usize,
}

impl<'a> VisiblePageTextSelection<'a> {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        data: &'a LoadedCardPageData,
        selection: &'a PageTextSelection,
    ) -> Option<Self> {
        let anchor_index = *data
            .editable_block_indices
            .get(&selection.anchor_block_id)?;
        let focus_index = *data.editable_block_indices.get(&selection.focus_block_id)?;
        if !data.visible_block_mask.get(anchor_index).copied()?
            || !data.visible_block_mask.get(focus_index).copied()?
        {
            return None;
        }
        Some(Self {
            data,
            selection,
            anchor_index,
            focus_index,
        })
    }

    pub(in crate::ui::board_workspace::page::editor) fn anchor_index(self) -> usize {
        self.anchor_index
    }

    pub(in crate::ui::board_workspace::page::editor) fn focus_index(self) -> usize {
        self.focus_index
    }

    pub(in crate::ui::board_workspace::page::editor) fn ordered_block_span(
        self,
    ) -> std::ops::RangeInclusive<usize> {
        self.anchor_index.min(self.focus_index)..=self.anchor_index.max(self.focus_index)
    }

    pub(in crate::ui::board_workspace::page::editor) fn contains_visible_block_index(
        self,
        candidate_index: usize,
    ) -> bool {
        self.ordered_block_span().contains(&candidate_index)
            && self
                .data
                .visible_block_mask
                .get(candidate_index)
                .copied()
                .unwrap_or(false)
    }

    pub(in crate::ui::board_workspace::page::editor) fn block_indices(
        self,
    ) -> impl DoubleEndedIterator<Item = usize> + 'a {
        let first_index = self.anchor_index.min(self.focus_index);
        let last_index = self.anchor_index.max(self.focus_index);
        let first_row_index = self
            .data
            .visible_rows
            .binary_search_by_key(&first_index, |row| row.block_index)
            .expect("visible text-selection endpoint must resolve to a visible row");
        let last_row_index = self
            .data
            .visible_rows
            .binary_search_by_key(&last_index, |row| row.block_index)
            .expect("visible text-selection endpoint must resolve to a visible row");
        self.data.visible_rows[first_row_index..=last_row_index]
            .iter()
            .map(|row| row.block_index)
    }

    pub(in crate::ui::board_workspace::page::editor) fn range_for(
        self,
        candidate_index: usize,
    ) -> Option<DirectedTextRange> {
        if !self.contains_visible_block_index(candidate_index) {
            return None;
        }
        let editable = self
            .data
            .page
            .blocks
            .get(candidate_index)?
            .editable_content()?;
        if self.anchor_index == self.focus_index {
            let start = self
                .selection
                .anchor_offset
                .min(self.selection.focus_offset);
            let end = self
                .selection
                .anchor_offset
                .max(self.selection.focus_offset);
            return Some((
                start..end,
                self.selection.focus_offset < self.selection.anchor_offset,
            ));
        }
        let (start_index, end_index, start_offset, end_offset) =
            if self.anchor_index < self.focus_index {
                (
                    self.anchor_index,
                    self.focus_index,
                    self.selection.anchor_offset,
                    self.selection.focus_offset,
                )
            } else {
                (
                    self.focus_index,
                    self.anchor_index,
                    self.selection.focus_offset,
                    self.selection.anchor_offset,
                )
            };
        let range = if candidate_index == start_index {
            start_offset..editable.text.len()
        } else if candidate_index == end_index {
            0..end_offset
        } else {
            0..editable.text.len()
        };
        Some((
            range,
            candidate_index == self.focus_index && self.focus_index < self.anchor_index,
        ))
    }
}
