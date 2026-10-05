use gpui::App;

use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::super::{CardPageBlockColor, CardPageQuoteSize};
use super::context::page_block_supports_color;
use crate::model::{
    PageMutation, SetPageBlockColorRequest, SetPageQuoteSizeRequest, SetPageToDoStateRequest,
};

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn toggle_page_to_do_state(
        &mut self,
        block_id: &str,
        cx: &App,
    ) {
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)
        else {
            return;
        };
        let Some(current) = page.blocks[index]
            .editable_content()
            .and_then(|editable| editable.to_do_state())
        else {
            return;
        };
        let next = current.toggled();
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let updated = page.blocks[index]
            .editable_content_mut()
            .expect("validated to-do block must remain editable")
            .set_to_do_state(next);
        debug_assert!(updated, "validated to-do block must retain to-do state");
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::SetToDoState(SetPageToDoStateRequest {
                    block_id: block_id.to_string(),
                    state: next,
                }),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn set_page_block_color(
        &mut self,
        block_id: &str,
        color: CardPageBlockColor,
        cx: &App,
    ) {
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let root_indices = self
            .editor
            .page_block_context_menu_root_indices(&page, block_id)
            .into_iter()
            .filter(|index| page_block_supports_color(&page.blocks[*index]))
            .collect::<Vec<_>>();
        if root_indices.is_empty() {
            return;
        }
        self.editor.last_used_page_block_color = color;
        self.editor.page_block_context_menu = None;
        if root_indices
            .iter()
            .all(|index| page.blocks[*index].color == color)
        {
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let block_ids = root_indices
            .iter()
            .map(|index| page.blocks[*index].block_id.clone())
            .collect::<Vec<_>>();
        for index in root_indices {
            page.blocks[index].color = color;
        }
        let page_id = page.block_id.clone();
        let request = SetPageBlockColorRequest::new(block_ids, color)
            .expect("selected color targets must be non-empty and unique");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::SetBlockColor(request),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn set_page_quote_size(
        &mut self,
        block_id: &str,
        size: CardPageQuoteSize,
        cx: &App,
    ) {
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let root_indices = self
            .editor
            .page_block_context_menu_root_indices(&page, block_id);
        if root_indices.is_empty()
            || root_indices.iter().any(|index| {
                page.blocks[*index]
                    .editable_content()
                    .and_then(|editable| editable.quote_size())
                    .is_none()
            })
        {
            return;
        }
        self.editor.page_block_context_menu = None;
        if root_indices.iter().all(|index| {
            page.blocks[*index]
                .editable_content()
                .and_then(|editable| editable.quote_size())
                == Some(size)
        }) {
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let block_ids = root_indices
            .iter()
            .map(|index| page.blocks[*index].block_id.clone())
            .collect::<Vec<_>>();
        for index in root_indices {
            let updated = page.blocks[index]
                .editable_content_mut()
                .expect("validated quote block must remain editable")
                .set_quote_size(size);
            debug_assert!(updated, "validated quote block must retain quote size");
        }
        let page_id = page.block_id.clone();
        let request = SetPageQuoteSizeRequest::new(block_ids, size)
            .expect("selected quote-size targets must be non-empty and unique");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::SetQuoteSize(request),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }
}
