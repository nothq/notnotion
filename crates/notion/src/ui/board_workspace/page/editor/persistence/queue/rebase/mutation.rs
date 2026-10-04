use crate::model::{
    BreakPageTextSelectionRequest, CardPage, CardPageSimpleTableCellIndex,
    CardPageWritableSimpleTableCell, PageMutation, PageTextSelectionEdit,
    PageTextSelectionEndpoint, PastePageTextSelectionRequest, ReplacePageSimpleTableCellRequest,
    ReplacePageTextSelectionRequest,
};

use super::block_mutation::{
    convert_block, convert_to_divider, delete_block, duplicate_alias, merge_blocks, replace_text,
    replace_with_code, replace_with_divider, restore_from_code, restore_from_divider,
    set_page_icon, split_block,
};
use super::column_ratio::{resize_columns, validate_resize_source};
use super::hierarchy::{create_block, create_code_block, editable_mut, reorder_subtrees, NewBlock};
use super::selection::{break_text_selection, paste_text_selection, replace_text_selection};
use super::text::target_text;
use super::text_offset::{rebase_text_offset, utf8_offset_from_utf16, TextOffsetBias};
use super::{PageWriteProjector, PageWriteReplayError, ReplayPageSide};

impl PageWriteProjector {
    pub(super) fn rebase_and_apply_mutation(
        &mut self,
        mutation: &mut PageMutation,
    ) -> Result<(), PageWriteReplayError> {
        let local_mutation = mutation.clone();
        self.rebase_mutation_request(mutation)?;
        apply_page_mutation(&mut self.local, &local_mutation)?;
        apply_page_mutation(&mut self.authority, mutation)
    }

    fn rebase_mutation_request(
        &self,
        mutation: &mut PageMutation,
    ) -> Result<(), PageWriteReplayError> {
        match mutation {
            PageMutation::SplitBlock(request) => self.rebase_split_request(request),
            PageMutation::ReplaceTextSelection(request) => self.rebase_replacement_request(request),
            PageMutation::BreakTextSelection(request) => self.rebase_break_request(request),
            PageMutation::PasteTextSelection(request) => self.rebase_paste_request(request),
            PageMutation::ResizeColumns(request) => {
                validate_resize_source(&self.local, request)?;
                validate_resize_source(&self.authority, request)
            }
            PageMutation::CreateBlock(_)
            | PageMutation::CreateCodeBlock(_)
            | PageMutation::DuplicateAlias(_)
            | PageMutation::ReplaceBlockWithCode(_)
            | PageMutation::RestoreBlockFromCode(_)
            | PageMutation::ReplaceBlockWithDivider(_)
            | PageMutation::ConvertBlockToDivider(_)
            | PageMutation::RestoreBlockFromDivider(_)
            | PageMutation::ReplaceBlockText(_)
            | PageMutation::ReplaceBlockTextAndConvert(_)
            | PageMutation::MergeBlocks(_)
            | PageMutation::ConvertBlock(_)
            | PageMutation::SetToDoState(_)
            | PageMutation::SetCodeLanguage(_)
            | PageMutation::SetCodeWrap(_)
            | PageMutation::SetPageIcon(_)
            | PageMutation::SetBlockColor(_)
            | PageMutation::SetQuoteSize(_)
            | PageMutation::DeleteBlock(_)
            | PageMutation::ReorderSubtrees(_) => Ok(()),
            PageMutation::ReplaceSimpleTableCell(request) => {
                self.validate_simple_table_cell_request(request)
            }
            PageMutation::InsertMention(request) => self.rebase_insert_mention_request(request),
            PageMutation::UpdateMention(request) => self.rebase_update_mention_request(request),
        }
    }

    fn validate_simple_table_cell_request(
        &self,
        request: &ReplacePageSimpleTableCellRequest,
    ) -> Result<(), PageWriteReplayError> {
        let index = CardPageSimpleTableCellIndex::new(&self.authority).map_err(|detail| {
            PageWriteReplayError::conflict("replace simple-table cell", detail)
        })?;
        let cell = index
            .cell(&self.authority, request.target())
            .map_err(|detail| {
                PageWriteReplayError::conflict("replace simple-table cell", detail)
            })?;
        CardPageWritableSimpleTableCell::try_from(cell.clone())
            .map(|_| ())
            .map_err(|detail| PageWriteReplayError::conflict("replace simple-table cell", detail))
    }

    fn rebase_split_request(
        &self,
        request: &mut crate::model::SplitPageBlockRequest,
    ) -> Result<(), PageWriteReplayError> {
        let local_text = target_text(
            &self.local,
            &request.block_id,
            "split block",
            ReplayPageSide::QueueBaseline,
        )?;
        let authority_text = target_text(
            &self.authority,
            &request.block_id,
            "split block",
            ReplayPageSide::Authority,
        )?;
        let local_utf8 =
            utf8_offset_from_utf16(&request.block_id, local_text, request.split_offset_utf16)?;
        let authority_utf8 = rebase_text_offset(
            &request.block_id,
            local_text,
            authority_text,
            local_utf8,
            TextOffsetBias::After,
        )?;
        request.split_offset_utf16 = authority_text[..authority_utf8].encode_utf16().count();
        Ok(())
    }

    fn rebase_replacement_request(
        &self,
        request: &mut ReplacePageTextSelectionRequest,
    ) -> Result<(), PageWriteReplayError> {
        let (start, end) = self.rebase_selection_endpoints(request.start(), request.end())?;
        *request = ReplacePageTextSelectionRequest::new(
            start,
            end,
            request.selected_block_ids().to_vec(),
            PageTextSelectionEdit {
                replacement: request.replacement().to_string(),
                action: request.action(),
            },
            request.structural_mutations().to_vec(),
        )
        .map_err(|detail| {
            PageWriteReplayError::invalid_request("replace text selection", detail)
        })?;
        Ok(())
    }

    fn rebase_break_request(
        &self,
        request: &mut BreakPageTextSelectionRequest,
    ) -> Result<(), PageWriteReplayError> {
        let (start, end) = self.rebase_selection_endpoints(request.start(), request.end())?;
        *request = BreakPageTextSelectionRequest::new(
            start,
            end,
            request.selected_block_ids().to_vec(),
            request.line_break().clone(),
            request.structural_mutations().to_vec(),
        )
        .map_err(|detail| PageWriteReplayError::invalid_request("break text selection", detail))?;
        Ok(())
    }

    fn rebase_paste_request(
        &self,
        request: &mut PastePageTextSelectionRequest,
    ) -> Result<(), PageWriteReplayError> {
        let (start, end) = self.rebase_selection_endpoints(request.start(), request.end())?;
        *request = PastePageTextSelectionRequest::new(
            start,
            end,
            request.new_block_id().to_string(),
            request.text().to_string(),
        )
        .map_err(|detail| PageWriteReplayError::invalid_request("paste text selection", detail))?;
        Ok(())
    }

    fn rebase_selection_endpoints(
        &self,
        start: &PageTextSelectionEndpoint,
        end: &PageTextSelectionEndpoint,
    ) -> Result<(PageTextSelectionEndpoint, PageTextSelectionEndpoint), PageWriteReplayError> {
        let start = self.rebase_selection_endpoint(start, TextOffsetBias::After)?;
        let end = self.rebase_selection_endpoint(end, TextOffsetBias::Before)?;
        Ok((start, end))
    }

    fn rebase_selection_endpoint(
        &self,
        endpoint: &PageTextSelectionEndpoint,
        bias: TextOffsetBias,
    ) -> Result<PageTextSelectionEndpoint, PageWriteReplayError> {
        let local = target_text(
            &self.local,
            &endpoint.block_id,
            "text selection",
            ReplayPageSide::QueueBaseline,
        )?;
        let authority = target_text(
            &self.authority,
            &endpoint.block_id,
            "text selection",
            ReplayPageSide::Authority,
        )?;
        let offset = rebase_text_offset(
            &endpoint.block_id,
            local,
            authority,
            endpoint.offset_utf8,
            bias,
        )?;
        Ok(PageTextSelectionEndpoint {
            block_id: endpoint.block_id.clone(),
            offset_utf8: offset,
        })
    }
}

pub(super) fn apply_page_mutation(
    page: &mut CardPage,
    mutation: &PageMutation,
) -> Result<(), PageWriteReplayError> {
    match mutation {
        PageMutation::CreateBlock(request) => create_block(
            page,
            NewBlock {
                block_id: &request.block_id,
                parent_id: &request.parent_block_id,
                kind: &request.kind,
                text: &request.text,
            },
            &request.placement,
        ),
        PageMutation::CreateCodeBlock(request) => create_code_block(page, request),
        PageMutation::DuplicateAlias(request) => duplicate_alias(page, request),
        PageMutation::ReplaceBlockWithCode(request) => replace_with_code(page, request),
        PageMutation::RestoreBlockFromCode(request) => restore_from_code(page, request),
        PageMutation::ReplaceBlockWithDivider(request) => replace_with_divider(page, request),
        PageMutation::ConvertBlockToDivider(request) => convert_to_divider(page, request),
        PageMutation::RestoreBlockFromDivider(request) => restore_from_divider(page, request),
        PageMutation::ReplaceBlockText(request) => {
            replace_text(page, &request.block_id, &request.text)
        }
        PageMutation::ReplaceSimpleTableCell(request) => CardPageSimpleTableCellIndex::new(page)
            .and_then(|index| index.replace(page, request.target(), request.cell().clone()))
            .map_err(|detail| {
                PageWriteReplayError::invalid_request("replace simple-table cell", detail)
            }),
        PageMutation::ReplaceBlockTextAndConvert(request) => {
            replace_text(page, &request.block_id, &request.text)?;
            convert_block(page, &request.block_id, &request.kind)
        }
        PageMutation::ReplaceTextSelection(request) => replace_text_selection(page, request),
        PageMutation::BreakTextSelection(request) => break_text_selection(page, request),
        PageMutation::PasteTextSelection(request) => paste_text_selection(page, request),
        PageMutation::SplitBlock(request) => split_block(page, request),
        PageMutation::MergeBlocks(request) => merge_blocks(page, request),
        PageMutation::ConvertBlock(request) => {
            convert_block(page, &request.block_id, &request.kind)
        }
        PageMutation::SetToDoState(request) => set_to_do_state(page, request),
        PageMutation::SetCodeLanguage(request) => set_code_language(page, request),
        PageMutation::SetCodeWrap(request) => set_code_wrap(page, request),
        PageMutation::SetPageIcon(request) => {
            set_page_icon(page, request.block_id(), request.icon().cloned())
        }
        PageMutation::SetBlockColor(request) => set_block_color(page, request),
        PageMutation::SetQuoteSize(request) => set_quote_size(page, request),
        PageMutation::ResizeColumns(request) => resize_columns(page, request),
        PageMutation::DeleteBlock(request) => delete_block(page, &request.block_id),
        PageMutation::ReorderSubtrees(request) => reorder_subtrees(page, request),
        PageMutation::InsertMention(request) => super::mention::insert_mention(page, request),
        PageMutation::UpdateMention(request) => super::mention::update_mention(page, request),
    }
}

fn set_to_do_state(
    page: &mut CardPage,
    request: &crate::model::SetPageToDoStateRequest,
) -> Result<(), PageWriteReplayError> {
    if editable_mut(page, &request.block_id, "set to-do state")?.set_to_do_state(request.state) {
        return Ok(());
    }
    Err(PageWriteReplayError::conflict(
        "set to-do state",
        format!("block {} is not a to-do", request.block_id),
    ))
}

fn set_code_language(
    page: &mut CardPage,
    request: &crate::model::SetPageCodeLanguageRequest,
) -> Result<(), PageWriteReplayError> {
    let applied = editable_mut(page, request.block_id(), "set code language")?
        .set_code_language(request.language().clone());
    if applied {
        return Ok(());
    }
    Err(PageWriteReplayError::conflict(
        "set code language",
        format!("block {} is not code", request.block_id()),
    ))
}

fn set_code_wrap(
    page: &mut CardPage,
    request: &crate::model::SetPageCodeWrapRequest,
) -> Result<(), PageWriteReplayError> {
    let applied =
        editable_mut(page, request.block_id(), "set code wrap")?.set_code_wrap(request.wrap());
    if applied {
        return Ok(());
    }
    Err(PageWriteReplayError::conflict(
        "set code wrap",
        format!("block {} is not code", request.block_id()),
    ))
}

fn set_block_color(
    page: &mut CardPage,
    request: &crate::model::SetPageBlockColorRequest,
) -> Result<(), PageWriteReplayError> {
    for block_id in request.block_ids() {
        let index = super::hierarchy::block_index(page, block_id, "set block color")?;
        page.blocks[index].color = request.color();
    }
    Ok(())
}

fn set_quote_size(
    page: &mut CardPage,
    request: &crate::model::SetPageQuoteSizeRequest,
) -> Result<(), PageWriteReplayError> {
    for block_id in request.block_ids() {
        let applied =
            editable_mut(page, block_id, "set quote size")?.set_quote_size(request.size());
        if !applied {
            return Err(PageWriteReplayError::conflict(
                "set quote size",
                format!("block {block_id} is not a quote"),
            ));
        }
    }
    Ok(())
}
