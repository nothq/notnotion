use gpui::{App, Window};
use gpui_components::text_input::TextInputSnapshot;

use crate::model::{
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell,
    PageMutation, PageTextAnnotation, PageTextAnnotationKind, ReplacePageSimpleTableCellRequest,
};
use crate::ui::surface::{
    PageEditorState, PageSimpleTableCellForcedAnnotations, PageSimpleTableCellGeneration,
    PageSimpleTableRuntime,
};
use crate::ui::KeyDownEvent;

use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::super::rich_text::annotations::{
    annotations_at_caret_in_spans, apply_annotation_actions_to_spans, range_has_annotation_in_spans,
};

pub(crate) enum PageSimpleTableShortcut {
    Consumed,
    Format(Box<PageSimpleTableFormatAction>),
}

pub(crate) struct PageSimpleTableFormatAction {
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    snapshot: TextInputSnapshot,
    annotation: PageTextAnnotation,
}

impl PageSimpleTableShortcut {
    pub(crate) fn capture(
        tables: &PageSimpleTableRuntime,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Self> {
        let annotation = simple_table_shortcut_annotation(event)?;
        let (address, generation, input) = {
            let active = tables.editor().borrow();
            let active = active.as_ref()?;
            (
                active.address.clone(),
                active.generation,
                active.input.clone()?,
            )
        };
        if !input.read(cx).focus_handle_clone().is_focused(window) {
            return None;
        }
        let snapshot = input.read(cx).snapshot();
        if snapshot.is_composing {
            return Some(Self::Consumed);
        }
        Some(Self::Format(Box::new(PageSimpleTableFormatAction {
            address,
            generation,
            snapshot,
            annotation,
        })))
    }

    pub(crate) fn apply(self, edit: &mut PageEditSession<'_>) {
        match self {
            Self::Consumed => {}
            Self::Format(action) => edit.apply_page_simple_table_format(*action),
        }
    }
}

impl PageEditSession<'_> {
    pub(super) fn apply_page_simple_table_format(&mut self, action: PageSimpleTableFormatAction) {
        let result = if action.snapshot.selection.is_empty() {
            self.toggle_forced_simple_table_annotation(
                &action.address,
                action.generation,
                action.snapshot.cursor,
                action.annotation,
            )
        } else {
            self.apply_selected_simple_table_annotation(action)
        };
        if let Err(error) = result {
            self.effects.push(PageEditEffect::Error(error));
        }
        self.effects.push(PageEditEffect::Notify);
    }

    fn toggle_forced_simple_table_annotation(
        &mut self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        offset: usize,
        annotation: PageTextAnnotation,
    ) -> Result<(), String> {
        let page_id = active_page_id(self.editor, address, generation)?;
        let page = self
            .documents
            .page_with_id(&page_id)
            .ok_or_else(|| format!("loaded page {page_id} disappeared during formatting"))?;
        let index = CardPageSimpleTableCellIndex::new(&page)?;
        let cell = index.cell(&page, address)?;
        CardPageWritableSimpleTableCell::try_from(cell.clone())?;
        let authority = self
            .documents
            .page_authority_with_id(&page_id)
            .ok_or_else(|| format!("live authority for page {page_id} disappeared"))?;
        let authority_index = CardPageSimpleTableCellIndex::new(&authority)?;
        CardPageWritableSimpleTableCell::try_from(
            authority_index.cell(&authority, address)?.clone(),
        )?;
        let inherited = annotations_at_caret_in_spans(cell.annotations(), offset);
        update_forced_annotations(
            self.editor,
            address,
            generation,
            CaretAnnotationToggle {
                offset,
                annotation,
                inherited: &inherited,
            },
        )
    }

    fn apply_selected_simple_table_annotation(
        &mut self,
        action: PageSimpleTableFormatAction,
    ) -> Result<(), String> {
        let PageSimpleTableFormatAction {
            address,
            generation,
            snapshot,
            annotation,
        } = action;
        let address = &address;
        let range = snapshot.selection;
        let cursor = snapshot.cursor;
        let page_id = active_page_id(self.editor, address, generation)?;
        let visual = self
            .documents
            .page_with_id(&page_id)
            .ok_or_else(|| format!("loaded page {page_id} disappeared during formatting"))?;
        let index = CardPageSimpleTableCellIndex::new(&visual)?;
        let cell = index.cell(&visual, address)?.clone();
        CardPageWritableSimpleTableCell::try_from(cell.clone())?;
        let enabled =
            range_has_annotation_in_spans(cell.annotations(), range.clone(), annotation.kind());
        let mut annotations = cell.annotations().to_vec();
        let removals = enabled
            .then_some(annotation.kind())
            .into_iter()
            .collect::<Vec<_>>();
        let additions = (!enabled)
            .then_some(annotation)
            .into_iter()
            .collect::<Vec<_>>();
        apply_annotation_actions_to_spans(&mut annotations, range, &removals, &additions);
        let target = CardPageWritableSimpleTableCell::new(cell.text().to_owned(), annotations)?;
        let authority = self
            .documents
            .page_authority_with_id(&page_id)
            .ok_or_else(|| format!("live authority for page {page_id} disappeared"))?;
        let request = ReplacePageSimpleTableCellRequest::new(
            &authority,
            address.clone(),
            target.as_cell().clone(),
        )?;
        self.editor
            .record_page_simple_table_cell_structural_edit(&visual, address, cursor);
        let mut optimistic = visual;
        index.replace(&mut optimistic, address, target)?;
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(optimistic));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::ReplaceSimpleTableCell(request),
            },
        ));
        Ok(())
    }
}

fn active_page_id(
    editor: &PageEditorState,
    address: &CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
) -> Result<String, String> {
    editor
        .tables
        .editor()
        .borrow()
        .as_ref()
        .filter(|active| active.address == *address && active.generation == generation)
        .map(|active| active.page_id.clone())
        .ok_or_else(|| "active table-cell target changed".to_string())
}

/// An annotation toggled at a collapsed caret, over the annotations it inherits there.
struct CaretAnnotationToggle<'a> {
    offset: usize,
    annotation: PageTextAnnotation,
    inherited: &'a [PageTextAnnotation],
}

fn update_forced_annotations(
    editor: &PageEditorState,
    address: &CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    toggle: CaretAnnotationToggle<'_>,
) -> Result<(), String> {
    let CaretAnnotationToggle {
        offset,
        annotation,
        inherited,
    } = toggle;
    let mut active = editor.tables.editor().borrow_mut();
    let active = active
        .as_mut()
        .filter(|active| active.address == *address && active.generation == generation)
        .ok_or_else(|| "active table-cell formatting target changed".to_string())?;
    let forced = active
        .forced_annotations
        .get_or_insert(PageSimpleTableCellForcedAnnotations {
            offset_utf8: offset,
            removals: Vec::new(),
            additions: Vec::new(),
        });
    let enabled = forced_annotation_enabled(inherited, forced, annotation.kind());
    set_forced_annotation(forced, annotation, !enabled);
    Ok(())
}

fn forced_annotation_enabled(
    inherited: &[PageTextAnnotation],
    forced: &PageSimpleTableCellForcedAnnotations,
    kind: PageTextAnnotationKind,
) -> bool {
    if forced.removals.contains(&kind) {
        return false;
    }
    forced.additions.iter().any(|item| item.kind() == kind)
        || inherited.iter().any(|item| item.kind() == kind)
}

fn set_forced_annotation(
    forced: &mut PageSimpleTableCellForcedAnnotations,
    annotation: PageTextAnnotation,
    enabled: bool,
) {
    let kind = annotation.kind();
    forced.removals.retain(|candidate| *candidate != kind);
    forced
        .additions
        .retain(|candidate| candidate.kind() != kind);
    if enabled {
        forced.additions.push(annotation);
    } else {
        forced.removals.push(kind);
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn update_page_simple_table_cell_selection(
        &mut self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: TextInputSnapshot,
    ) -> bool {
        let mut active = self.tables.editor().borrow_mut();
        let Some(active) = active
            .as_mut()
            .filter(|active| active.address == *address && active.generation == generation)
        else {
            return false;
        };
        if !snapshot.selection.is_empty() || snapshot.is_composing {
            active.forced_annotations = None;
        } else if active
            .forced_annotations
            .as_ref()
            .is_none_or(|forced| forced.offset_utf8 != snapshot.cursor)
        {
            active.forced_annotations = Some(PageSimpleTableCellForcedAnnotations {
                offset_utf8: snapshot.cursor,
                removals: Vec::new(),
                additions: Vec::new(),
            });
        }
        true
    }
}

fn simple_table_shortcut_annotation(event: &KeyDownEvent) -> Option<PageTextAnnotation> {
    let modifiers = event.keystroke.modifiers;
    if !modifiers.platform || modifiers.control || modifiers.alt || modifiers.function {
        return None;
    }
    match (event.keystroke.key.as_str(), modifiers.shift) {
        ("b", false) => Some(PageTextAnnotation::Bold),
        ("i", false) => Some(PageTextAnnotation::Italic),
        ("u", false) => Some(PageTextAnnotation::Underline),
        ("e", false) => Some(PageTextAnnotation::Code),
        ("x" | "s", true) => Some(PageTextAnnotation::Strike),
        _ => None,
    }
}
