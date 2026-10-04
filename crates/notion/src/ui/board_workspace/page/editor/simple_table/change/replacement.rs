use gpui_components::text_input::TextInputReplacementOrigin;

use crate::model::{
    CardPageSimpleTableCell, CardPageSimpleTableCellAddress, CardPageWritableSimpleTableCell,
};
use crate::ui::surface::{
    PageEditorState, PageSimpleTableCellForcedAnnotations, PageSimpleTableCellGeneration,
    PageSimpleTableCellObservedReplacement, PageSimpleTableCellReplacementOrigin,
};

use super::super::super::rich_text::annotations::{
    apply_annotation_actions_to_spans, apply_text_annotation_replacement_to_spans,
};

/// The inserted text range and the annotations forced onto it.
type ForcedInsertion = (std::ops::Range<usize>, PageSimpleTableCellForcedAnnotations);

impl PageEditorState {
    pub(super) fn changed_simple_table_cell(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        current: &CardPageSimpleTableCell,
        replacement: &PageSimpleTableCellObservedReplacement,
    ) -> Result<CardPageWritableSimpleTableCell, String> {
        let mut annotations = current.annotations().to_vec();
        apply_text_annotation_replacement_to_spans(
            &mut annotations,
            replacement.range.clone(),
            replacement.replacement.len(),
        );
        if let Some((range, forced)) =
            self.simple_table_forced_insertion(address, generation, replacement)
        {
            apply_annotation_actions_to_spans(
                &mut annotations,
                range,
                &forced.removals,
                &forced.additions,
            );
        }
        let text = format!(
            "{}{}{}",
            &current.text()[..replacement.range.start],
            replacement.replacement,
            &current.text()[replacement.range.end..]
        );
        CardPageWritableSimpleTableCell::new(text, annotations)
    }

    fn simple_table_forced_insertion(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        replacement: &PageSimpleTableCellObservedReplacement,
    ) -> Option<ForcedInsertion> {
        if !replacement.range.is_empty()
            || replacement.replacement.is_empty()
            || !is_typing_insertion(replacement.origin)
        {
            return None;
        }
        let inserted =
            replacement.range.start..replacement.range.start + replacement.replacement.len();
        let mut editor = self.tables.editor().borrow_mut();
        let active = editor
            .as_mut()
            .filter(|active| active.address == *address && active.generation == generation)?;
        let forced = active
            .forced_annotations
            .as_mut()
            .filter(|forced| forced.offset_utf8 == inserted.start)?;
        forced.offset_utf8 = inserted.end;
        Some((inserted, forced.clone()))
    }
}

fn is_typing_insertion(origin: PageSimpleTableCellReplacementOrigin) -> bool {
    matches!(
        origin,
        PageSimpleTableCellReplacementOrigin::TextInput(
            TextInputReplacementOrigin::Typing
                | TextInputReplacementOrigin::ImeMarked
                | TextInputReplacementOrigin::ImeCommit
        )
    )
}
