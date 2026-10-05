use std::{
    cell::{Cell, RefCell},
    ops::Range,
};

use gpui::{Entity, Pixels, Point};
use gpui_components::text_input::{
    TextInput, TextInputPreMutationAction, TextInputReplacementOrigin, TextInputSnapshot,
    TextInputVisualLine,
};

use crate::model::{
    CardPageSimpleTableCellAddress, CardPageWritableSimpleTableCell, PageTextAnnotation,
    PageTextAnnotationKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PageSimpleTableCellGeneration(u64);

#[derive(Clone, Debug)]
pub(crate) enum PageSimpleTableCellFocusMode {
    Pointer(Point<Pixels>),
    Start,
    End,
    Offset(usize),
    Vertical {
        window_x: Pixels,
        line: TextInputVisualLine,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct PageSimpleTableCellPendingFocus {
    pub(crate) mode: PageSimpleTableCellFocusMode,
    pub(crate) caret_applied: bool,
    pub(crate) horizontal_revealed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PageSimpleTableCellForcedAnnotations {
    pub(crate) offset_utf8: usize,
    pub(crate) removals: Vec<PageTextAnnotationKind>,
    pub(crate) additions: Vec<PageTextAnnotation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageSimpleTableCellReplacementOrigin {
    Cut,
    Paste,
    TextInput(TextInputReplacementOrigin),
}

#[derive(Clone, Debug)]
pub(crate) struct PageSimpleTableCellReplacementPlan {
    previous_text: String,
    previous_cursor: usize,
    range: Range<usize>,
    replacement: String,
    origin: PageSimpleTableCellReplacementOrigin,
}

pub(crate) struct PageSimpleTableCellObservedReplacement {
    pub(crate) previous_cursor: usize,
    pub(crate) range: Range<usize>,
    pub(crate) replacement: String,
    pub(crate) origin: PageSimpleTableCellReplacementOrigin,
}

#[derive(Clone)]
pub(crate) struct PageSimpleTableCellCompositionBaseline {
    pub(crate) cell: CardPageWritableSimpleTableCell,
    pub(crate) cursor: usize,
}

impl PageSimpleTableCellReplacementPlan {
    pub(crate) fn from_action(action: TextInputPreMutationAction) -> Result<Option<Self>, String> {
        let (snapshot, range, replacement, origin) = match action {
            TextInputPreMutationAction::Copy { .. } => return Ok(None),
            TextInputPreMutationAction::Cut { snapshot } => {
                let range = snapshot.selection.clone();
                (
                    snapshot,
                    range,
                    String::new(),
                    PageSimpleTableCellReplacementOrigin::Cut,
                )
            }
            TextInputPreMutationAction::Paste {
                snapshot,
                range,
                text,
                ..
            } => (
                snapshot,
                range,
                text,
                PageSimpleTableCellReplacementOrigin::Paste,
            ),
            TextInputPreMutationAction::Replace {
                snapshot,
                range,
                text,
                origin,
            } => (
                snapshot,
                range,
                text,
                PageSimpleTableCellReplacementOrigin::TextInput(origin),
            ),
        };
        validate_replacement_range(&snapshot.text, &range)?;
        Ok(Some(Self {
            previous_text: snapshot.text,
            previous_cursor: snapshot.cursor,
            range,
            replacement,
            origin,
        }))
    }

    pub(crate) fn observe(
        self,
        snapshot: &TextInputSnapshot,
    ) -> Result<Option<PageSimpleTableCellObservedReplacement>, String> {
        let expected = format!(
            "{}{}{}",
            &self.previous_text[..self.range.start],
            self.replacement,
            &self.previous_text[self.range.end..]
        );
        if snapshot.text != expected {
            return Err("table-cell input changed outside its typed replacement plan".to_string());
        }
        if expected == self.previous_text {
            return Ok(None);
        }
        Ok(Some(PageSimpleTableCellObservedReplacement {
            previous_cursor: self.previous_cursor,
            range: self.range,
            replacement: self.replacement,
            origin: self.origin,
        }))
    }

    pub(crate) fn previous_text(&self) -> &str {
        &self.previous_text
    }
}

fn validate_replacement_range(text: &str, range: &Range<usize>) -> Result<(), String> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Err("table-cell replacement range is not valid UTF-8 text".to_string());
    }
    Ok(())
}

pub(crate) struct PageSimpleTableCellEditor {
    pub(crate) page_id: String,
    pub(crate) address: CardPageSimpleTableCellAddress,
    pub(crate) generation: PageSimpleTableCellGeneration,
    pub(crate) input: Option<Entity<TextInput>>,
    pub(crate) pending_focus: Option<PageSimpleTableCellPendingFocus>,
    pub(crate) composition_active: bool,
    pub(crate) composition_history_recorded: bool,
    pub(crate) composition_history_baseline: Option<PageSimpleTableCellCompositionBaseline>,
    pub(crate) composition_dirty: bool,
    pub(crate) forced_annotations: Option<PageSimpleTableCellForcedAnnotations>,
    pub(crate) pending_replacement: Option<PageSimpleTableCellReplacementPlan>,
    pub(crate) blur_registered: bool,
    pub(crate) optimistic_composition_cell: Option<CardPageWritableSimpleTableCell>,
    pub(crate) retire_after_composition: bool,
}

#[derive(Default)]
pub(crate) struct PageSimpleTableCellEditorState {
    active: RefCell<Option<PageSimpleTableCellEditor>>,
    next_generation: Cell<u64>,
}

impl PageSimpleTableCellEditorState {
    pub(crate) fn pending_focus_matches(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) -> bool {
        self.active.borrow().as_ref().is_some_and(|active| {
            active.address == *address
                && active.generation == generation
                && active.pending_focus.is_some()
        })
    }

    pub(crate) fn activate(
        &self,
        page_id: String,
        address: CardPageSimpleTableCellAddress,
        mode: PageSimpleTableCellFocusMode,
    ) -> Option<PageSimpleTableCellGeneration> {
        if let Some(active) = self.active.borrow_mut().as_mut() {
            if active.page_id == page_id && active.address == address {
                active.pending_focus = Some(PageSimpleTableCellPendingFocus {
                    mode,
                    caret_applied: false,
                    horizontal_revealed: false,
                });
                return Some(active.generation);
            }
            if active.composition_active || active.composition_dirty {
                return None;
            }
        }
        let generation = PageSimpleTableCellGeneration(self.next_generation.get());
        self.next_generation
            .set(self.next_generation.get().wrapping_add(1));
        let input = self
            .active
            .borrow_mut()
            .take()
            .filter(|active| active.page_id == page_id && active.address == address)
            .and_then(|active| active.input);
        self.active.replace(Some(PageSimpleTableCellEditor {
            page_id,
            address,
            generation,
            input,
            pending_focus: Some(PageSimpleTableCellPendingFocus {
                mode,
                caret_applied: false,
                horizontal_revealed: false,
            }),
            composition_active: false,
            composition_history_recorded: false,
            composition_history_baseline: None,
            composition_dirty: false,
            forced_annotations: None,
            pending_replacement: None,
            blur_registered: false,
            optimistic_composition_cell: None,
            retire_after_composition: false,
        }));
        Some(generation)
    }

    pub(crate) fn clear(&self) -> bool {
        self.active.borrow_mut().take().is_some()
    }

    pub(crate) fn clear_unless_composing(&self) -> bool {
        if self
            .active
            .borrow()
            .as_ref()
            .is_some_and(|active| active.composition_active || active.composition_dirty)
        {
            return false;
        }
        self.active.borrow_mut().take();
        true
    }

    pub(crate) fn borrow(&self) -> std::cell::Ref<'_, Option<PageSimpleTableCellEditor>> {
        self.active.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> std::cell::RefMut<'_, Option<PageSimpleTableCellEditor>> {
        self.active.borrow_mut()
    }

    pub(crate) fn matches(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) -> bool {
        self.active
            .borrow()
            .as_ref()
            .is_some_and(|active| active.address == *address && active.generation == generation)
    }

    pub(crate) fn retains_local_composition(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) -> bool {
        self.active.borrow().as_ref().is_some_and(|active| {
            active.address == *address
                && active.generation == generation
                && (active.composition_active || active.composition_dirty)
        })
    }

    pub(crate) fn has_pending_composition_for_page(&self, page_id: &str) -> bool {
        self.active.borrow().as_ref().is_some_and(|active| {
            active.page_id == page_id && (active.composition_active || active.composition_dirty)
        })
    }
}
