use gpui::App;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::{CardPage, CardPageBlockKind};
use crate::model::PageMutation;

pub(in crate::ui::board_workspace::page::editor) struct PreparedPageBlockConversion {
    page: CardPage,
    roots: Vec<PreparedPageBlockConversionRoot>,
}

struct PreparedPageBlockConversionRoot {
    index: usize,
    block_id: String,
}

impl PreparedPageBlockConversion {
    pub(in crate::ui::board_workspace::page::editor) fn for_ids(
        page: CardPage,
        block_ids: &[String],
    ) -> Option<PreparedPageBlockConversion> {
        if block_ids.is_empty() {
            return None;
        }
        let indices = block_ids
            .iter()
            .map(|block_id| {
                page.blocks
                    .iter()
                    .position(|block| block.block_id == *block_id)
            })
            .collect::<Option<Vec<_>>>()?;
        Self::at_indices(page, indices)
    }

    pub(super) fn at_indices(
        page: CardPage,
        indices: Vec<usize>,
    ) -> Option<PreparedPageBlockConversion> {
        if indices.is_empty() {
            return None;
        }
        let roots = indices
            .into_iter()
            .map(|index| {
                let block = &page.blocks[index];
                let editable = block.editable_content()?;
                VerifiedPageTextBlockKind::parse(editable.kind)?;
                Some(PreparedPageBlockConversionRoot {
                    index,
                    block_id: block.block_id.clone(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(PreparedPageBlockConversion { page, roots })
    }
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn turn_page_block_into(
        &mut self,
        block_id: &str,
        kind: CardPageBlockKind,
        cx: &App,
    ) {
        let Some(target) = VerifiedPageTextBlockKind::parse(kind) else {
            return;
        };
        let Some(page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let indices = self
            .editor
            .page_block_context_menu_root_indices(&page, block_id);
        let Some(prepared) = PreparedPageBlockConversion::at_indices(page, indices) else {
            return;
        };
        self.apply_prepared_page_block_conversion(prepared, target, cx);
    }

    pub(in crate::ui::board_workspace::page::editor) fn apply_prepared_page_block_conversion(
        &mut self,
        prepared: PreparedPageBlockConversion,
        target: VerifiedPageTextBlockKind,
        cx: &App,
    ) {
        let PreparedPageBlockConversion { mut page, roots } = prepared;
        let page_block_id = page.block_id.clone();
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        for root in roots {
            page.blocks[root.index]
                .editable_content_mut()
                .expect("validated page block must remain editable")
                .set_kind(target.card_kind());
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::EnqueueMutation {
                    page_id: page_block_id.clone(),
                    mutation: PageMutation::ConvertBlock(target.conversion_request(root.block_id)),
                },
            ));
        }
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenu,
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }
}
