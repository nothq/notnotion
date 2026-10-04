use gpui::App;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::annotations::{
    annotations_at_caret, apply_selection_annotation, selection_annotation_kinds,
    selection_has_annotation,
};
use super::{PageForcedTextAnnotations, PageRichTextDialog};
use crate::model::{
    EditPageBlockTextRequest, PageTextAnnotation, PageTextAnnotationKind, PageTextColor,
    PageTextEditTarget,
};
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPage, KeyDownEvent, LoadedCardPageData, PageTextSelection};

type PageRichTextSelectionTargets = (CardPage, Vec<PageTextEditTarget>);

impl PageEditSession<'_> {
    pub(crate) fn handle_page_rich_text_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut App,
    ) -> bool {
        if event.keystroke.key == "escape" && self.editor.page_rich_text_dialog.take().is_some() {
            self.editor.page_rich_text_link_input.borrow_mut().take();
            self.effects.push(PageEditEffect::Notify);
            return true;
        }
        if self.editor.page_rich_text_dialog.is_some() {
            return false;
        }
        let modifiers = event.keystroke.modifiers;
        if !modifiers.platform || modifiers.control || modifiers.alt || modifiers.function {
            return false;
        }
        let annotation = match (event.keystroke.key.as_str(), modifiers.shift) {
            ("b", false) => PageTextAnnotation::Bold,
            ("i", false) => PageTextAnnotation::Italic,
            ("u", false) => PageTextAnnotation::Underline,
            ("e", false) => PageTextAnnotation::Code,
            ("x" | "s", true) => PageTextAnnotation::Strike,
            ("k", false) => {
                self.open_page_rich_text_link_dialog(cx);
                return true;
            }
            _ => return false,
        };
        self.toggle_page_text_annotation(annotation, cx);
        true
    }

    pub(super) fn toggle_page_text_annotation(
        &mut self,
        annotation: PageTextAnnotation,
        cx: &mut App,
    ) {
        if let Some((page, targets)) = self.page_rich_text_selection_targets() {
            let enabled = selection_has_annotation(&page, &targets, annotation.kind());
            if enabled {
                self.apply_page_text_selection_action(targets, Some(annotation.kind()), None, cx);
            } else {
                self.apply_page_text_selection_action(targets, None, Some(annotation), cx);
            }
            return;
        }
        self.toggle_forced_page_text_annotation(annotation, cx);
    }

    pub(super) fn clear_selected_page_text_formatting(&mut self, cx: &mut App) {
        let Some((mut page, targets)) = self.page_rich_text_selection_targets() else {
            self.editor.page_forced_text_annotations = None;
            self.effects.push(PageEditEffect::Notify);
            return;
        };
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let mut removals = selection_annotation_kinds(&page, &targets);
        for kind in [
            PageTextAnnotationKind::TextColor,
            PageTextAnnotationKind::BackgroundColor,
        ] {
            if !removals.contains(&kind) {
                removals.push(kind);
            }
        }
        let request = EditPageBlockTextRequest::new(
            page.block_id.clone(),
            targets.clone(),
            removals.clone(),
            vec![PageTextAnnotation::TextColor(PageTextColor::Default)],
        )
        .expect("selection clear-format action must be valid");
        for kind in removals {
            apply_selection_annotation(&mut page, &targets, Some(kind), None);
        }
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueRichTextEdit(request),
        ));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::SynchronizeTextInputSelections {
                excluded_block_id: None,
            },
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(super) fn set_page_rich_text_color(
        &mut self,
        color: PageTextColor,
        background: bool,
        cx: &mut App,
    ) {
        let (kind, annotation) = if background {
            (
                PageTextAnnotationKind::BackgroundColor,
                PageTextAnnotation::BackgroundColor(color),
            )
        } else {
            (
                PageTextAnnotationKind::TextColor,
                PageTextAnnotation::TextColor(color),
            )
        };
        if let Some((_, targets)) = self.page_rich_text_selection_targets() {
            if color == PageTextColor::Default {
                self.apply_page_text_selection_action(targets, Some(kind), None, cx);
            } else {
                self.apply_page_text_selection_action(targets, None, Some(annotation), cx);
            }
        } else {
            self.set_forced_page_text_annotation(annotation, color != PageTextColor::Default, cx);
        }
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::DismissRichTextColorDialog,
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(super) fn open_page_rich_text_link_dialog(&mut self, cx: &mut App) {
        if self.page_rich_text_selection_targets().is_none()
            && self.page_rich_text_caret(cx).is_none()
        {
            return;
        }
        self.effects.push(PageEditEffect::OpenRichTextLinkInput);
    }

    pub(super) fn apply_page_rich_text_link(&mut self, cx: &mut App) {
        let url = self.editor.page_rich_text_link_value.trim().to_string();
        if url.is_empty() {
            self.effects.push(PageEditEffect::Error(
                "A text link requires a URL.".to_string(),
            ));
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let annotation = PageTextAnnotation::Link(url);
        if let Some((_, targets)) = self.page_rich_text_selection_targets() {
            self.apply_page_text_selection_action(targets, None, Some(annotation), cx);
        } else {
            self.set_forced_page_text_annotation(annotation, true, cx);
        }
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::CloseRichTextDialog,
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(super) fn remove_page_rich_text_link(&mut self, cx: &mut App) {
        if let Some((_, targets)) = self.page_rich_text_selection_targets() {
            self.apply_page_text_selection_action(
                targets,
                Some(PageTextAnnotationKind::Link),
                None,
                cx,
            );
        } else {
            self.set_forced_page_text_annotation(
                PageTextAnnotation::Link(String::new()),
                false,
                cx,
            );
        }
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::CloseRichTextDialog,
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    fn apply_page_text_selection_action(
        &mut self,
        targets: Vec<PageTextEditTarget>,
        removal: Option<PageTextAnnotationKind>,
        addition: Option<PageTextAnnotation>,
        cx: &mut App,
    ) {
        let page_block_id = self
            .editor
            .page_text_selection
            .as_ref()
            .and_then(|selection| {
                self.documents
                    .page_containing_block(&selection.anchor_block_id)
            })
            .map(|page| page.block_id)
            .expect("selected page text must belong to a loaded page");
        let request = EditPageBlockTextRequest::new(
            page_block_id.clone(),
            targets.clone(),
            removal.into_iter().collect(),
            addition.clone().into_iter().collect(),
        )
        .expect("selection formatting action must be valid");
        let mut page = self
            .documents
            .page_with_id(&page_block_id)
            .expect("selected page text must belong to a loaded page");
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        apply_selection_annotation(&mut page, &targets, removal, addition);
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueRichTextEdit(request),
        ));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::SynchronizeTextInputSelections {
                excluded_block_id: None,
            },
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor::rich_text) fn page_rich_text_selection_targets(
        &self,
    ) -> Option<PageRichTextSelectionTargets> {
        let selection = self.editor.page_text_selection.as_ref()?;
        if selection.pointer_active {
            return None;
        }
        let data = self
            .documents
            .page_data_containing_editable_block(&selection.anchor_block_id)?;
        let targets = selection_targets(&data, selection);
        (!targets.is_empty()).then(|| (data.page.clone(), targets))
    }

    fn page_rich_text_caret(&self, cx: &App) -> Option<(CardPage, String, usize)> {
        let block_id = self.editor.active_page_block.as_ref()?;
        let input = self
            .editor
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(block_id)?
            .clone();
        let range = input.read(cx).selection_range();
        if !range.is_empty() {
            return None;
        }
        Some((
            self.documents.page_containing_block(block_id)?,
            block_id.clone(),
            range.start,
        ))
    }

    fn toggle_forced_page_text_annotation(&mut self, annotation: PageTextAnnotation, cx: &mut App) {
        let Some((page, block_id, offset)) = self.page_rich_text_caret(cx) else {
            return;
        };
        let editable = page
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .and_then(|block| block.editable_content())
            .expect("active page text block must be editable");
        let kind = annotation.kind();
        let inherited = annotations_at_caret(editable, offset)
            .iter()
            .any(|candidate| candidate.kind() == kind);
        let forced = self.editor.forced_page_text_annotations(&block_id, offset);
        let enabled = forced
            .additions
            .iter()
            .any(|candidate| candidate.kind() == kind)
            || inherited && !forced.removals.contains(&kind);
        forced.set(annotation, !enabled);
        self.effects.push(PageEditEffect::Notify);
    }

    fn set_forced_page_text_annotation(
        &mut self,
        annotation: PageTextAnnotation,
        enabled: bool,
        cx: &mut App,
    ) {
        let Some((_, block_id, offset)) = self.page_rich_text_caret(cx) else {
            return;
        };
        self.editor
            .forced_page_text_annotations(&block_id, offset)
            .set(annotation, enabled);
        self.effects.push(PageEditEffect::Notify);
    }
}

impl PageEditorState {
    pub(super) fn open_page_rich_text_color_dialog(&mut self) {
        self.page_rich_text_dialog = Some(PageRichTextDialog::Color);
    }

    pub(in crate::ui::board_workspace::page::editor) fn close_page_rich_text_dialog(&mut self) {
        self.page_rich_text_dialog = None;
        self.page_rich_text_link_input.borrow_mut().take();
    }

    fn forced_page_text_annotations(
        &mut self,
        block_id: &str,
        offset: usize,
    ) -> &mut PageForcedTextAnnotations {
        let matches = self
            .page_forced_text_annotations
            .as_ref()
            .is_some_and(|forced| forced.block_id == block_id && forced.offset_utf8 == offset);
        if !matches {
            self.page_forced_text_annotations =
                Some(PageForcedTextAnnotations::new(block_id.to_string(), offset));
        }
        self.page_forced_text_annotations
            .as_mut()
            .expect("forced page text annotations must be initialized")
    }
}

fn selection_targets(
    data: &LoadedCardPageData,
    selection: &PageTextSelection,
) -> Vec<PageTextEditTarget> {
    super::super::selection::page_text_selection_ranges(data, selection)
        .into_iter()
        .filter(|(_, range, _)| !range.is_empty())
        .map(|(block_id, range, _)| PageTextEditTarget::Selection {
            block_id,
            start_utf8: range.start,
            end_utf8: range.end,
        })
        .collect()
}
