use std::rc::Rc;

use gpui::Context;
use gpui_components::text_input::{
    TextInput, TextInputPreMutationAction, TextInputReplacementOrigin,
};

use super::{PagePendingRichTextComposition, PagePendingRichTextTyping};
use crate::ui::surface::PageEditorState;
use crate::ui::view_actions::ViewActionSink;

pub(crate) enum PageRichTextInputAction {
    LinkChanged(String),
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn capture_page_rich_text_pre_mutation(
        &mut self,
        block_id: &str,
        action: &TextInputPreMutationAction,
    ) -> bool {
        if let Some(pending) = pending_typing(block_id, action) {
            if self.capture_page_rich_text_typing(pending) {
                return true;
            }
        }
        self.capture_page_rich_text_composition(block_id, action)
    }
}

fn pending_typing(
    block_id: &str,
    action: &TextInputPreMutationAction,
) -> Option<PagePendingRichTextTyping> {
    let TextInputPreMutationAction::Replace {
        snapshot,
        range,
        text,
        origin: TextInputReplacementOrigin::Typing,
    } = action
    else {
        return None;
    };
    if snapshot.is_composing
        || !snapshot.selection.is_empty()
        || !range.is_empty()
        || range.start != snapshot.cursor
        || text.is_empty()
        || text.contains('\n')
    {
        return None;
    }
    Some(PagePendingRichTextTyping {
        block_id: block_id.to_string(),
        offset_utf8: range.start,
        text: text.clone(),
        removals: Vec::new(),
        additions: Vec::new(),
    })
}

impl PageEditorState {
    fn capture_page_rich_text_typing(&mut self, mut pending: PagePendingRichTextTyping) -> bool {
        if self.page_text_selection.is_some() {
            return false;
        }
        if let Some(forced) = self.page_forced_text_annotations.as_ref() {
            if forced.block_id == pending.block_id && forced.offset_utf8 == pending.offset_utf8 {
                pending.removals.clone_from(&forced.removals);
                pending.additions.clone_from(&forced.additions);
            } else {
                self.page_forced_text_annotations = None;
            }
        }
        self.page_pending_rich_text_typing = Some(pending);
        true
    }

    fn capture_page_rich_text_composition(
        &mut self,
        block_id: &str,
        action: &TextInputPreMutationAction,
    ) -> bool {
        let TextInputPreMutationAction::Replace {
            snapshot,
            range,
            text,
            origin,
        } = action
        else {
            return false;
        };
        if !matches!(
            origin,
            TextInputReplacementOrigin::ImeMarked | TextInputReplacementOrigin::ImeCommit
        ) {
            return false;
        }
        if let Some(composition) = self
            .page_pending_rich_text_composition
            .as_mut()
            .filter(|composition| composition.block_id == block_id)
        {
            composition.text.clone_from(text);
            return true;
        }
        if *origin != TextInputReplacementOrigin::ImeMarked
            || !range.is_empty()
            || !snapshot.selection.is_empty()
        {
            return false;
        }
        let Some(forced) = self
            .page_forced_text_annotations
            .as_ref()
            .filter(|forced| forced.block_id == block_id && forced.offset_utf8 == range.start)
        else {
            return false;
        };
        self.page_pending_rich_text_composition = Some(PagePendingRichTextComposition {
            block_id: block_id.to_string(),
            offset_utf8: range.start,
            baseline_text: snapshot.text.clone(),
            text: text.clone(),
            removals: forced.removals.clone(),
            additions: forced.additions.clone(),
        });
        true
    }
}

pub(crate) fn page_rich_text_link_on_change(
    actions: ViewActionSink<PageRichTextInputAction>,
) -> gpui_components::text_input::TextInputChange {
    Rc::new(move |value, window, cx: &mut Context<TextInput>| {
        actions.emit(PageRichTextInputAction::LinkChanged(value), window, cx);
    })
}
