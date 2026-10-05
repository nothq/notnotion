use std::collections::HashSet;

use gpui::Context;

use super::super::editing::PageEditSession;
use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::{CardPage, CardPageBlockKind, SurfaceState};
use crate::model::PageTextEditTarget;

impl SurfaceState {
    pub(super) fn turn_selected_page_blocks_into_text(&mut self, cx: &mut Context<Self>) {
        let selection = {
            let session = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            session.page_rich_text_selection_targets()
        };
        let Some((page, targets)) = selection else {
            return;
        };
        let Some(block_ids) = selected_non_text_block_ids(&page, &targets) else {
            return;
        };
        let Some(prepared) =
            super::super::block_ops::PreparedPageBlockConversion::for_ids(page, &block_ids)
        else {
            return;
        };
        let transition = {
            let mut edit = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            edit.apply_prepared_page_block_conversion(
                prepared,
                VerifiedPageTextBlockKind::text(),
                cx,
            );
            edit.finish(())
        };
        self.apply_page_edit_transition(transition, cx);
        self.page_editor
            .synchronize_page_text_input_selections(&self.page_documents, None, cx);
    }
}

fn selected_non_text_block_ids(
    page: &CardPage,
    targets: &[PageTextEditTarget],
) -> Option<Vec<String>> {
    let mut seen = HashSet::new();
    let mut block_ids = Vec::new();
    for block_id in targets.iter().map(PageTextEditTarget::block_id) {
        if !seen.insert(block_id) {
            continue;
        }
        let editable = page
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)?
            .editable_content()?;
        if editable.kind != CardPageBlockKind::Text {
            block_ids.push(block_id.to_string());
        }
    }
    Some(block_ids)
}
