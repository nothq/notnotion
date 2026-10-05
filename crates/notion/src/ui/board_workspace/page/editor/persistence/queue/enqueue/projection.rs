use super::super::super::super::rich_text::{
    annotations::{
        apply_mention_insert_to_editable, apply_mention_update_to_editable,
        apply_plain_text_annotation_replacement,
    },
    PageProjectedText, PageWriteTextProjection,
};
use crate::model::{
    EditPageBlockTextRequest, InsertPageMentionRequest, PageMutation,
    ReplacePageBlockTextAndConvertRequest, ReplacePageBlockTextRequest,
    ReplacePageSimpleTableCellRequest, RestorePageBlockFromCodeRequest, UpdatePageMentionRequest,
};
use crate::ui::surface::PageDocuments;

impl PageDocuments {
    pub(in crate::ui::board_workspace::page::editor::persistence::queue) fn page_rich_text_projection(
        &self,
        request: &EditPageBlockTextRequest,
    ) -> PageWriteTextProjection {
        let block_ids = request
            .targets()
            .iter()
            .map(|target| target.block_id().to_string())
            .collect::<Vec<_>>();
        self.loaded_page_text_projection(&request.page_block_id, &block_ids)
    }

    pub(crate) fn loaded_page_text_projection(
        &self,
        page_id: &str,
        block_ids: &[String],
    ) -> PageWriteTextProjection {
        let data = self
            .active_page_data_with_id(page_id)
            .expect("projected page write must target a loaded page");
        PageWriteTextProjection::updates(block_ids.iter().map(|block_id| {
            if block_id == page_id {
                return PageProjectedText::title(data.page.title.clone());
            }
            let index = data
                .editable_block_indices
                .get(block_id)
                .copied()
                .expect("projected page write must target an editable block");
            PageProjectedText::block(&data.page.blocks[index])
                .expect("projected page write target must remain editable")
        }))
    }

    pub(in crate::ui::board_workspace::page::editor::persistence::queue) fn page_mutation_text_projection(
        &self,
        page_id: &str,
        mutation: &PageMutation,
    ) -> PageWriteTextProjection {
        match mutation {
            PageMutation::CreateBlock(request) => PageWriteTextProjection::update(
                PageProjectedText::plain_block(request.block_id.clone(), request.text.clone()),
            ),
            PageMutation::CreateCodeBlock(request) => PageWriteTextProjection::update(
                PageProjectedText::plain_block(request.block_id(), ""),
            ),
            PageMutation::ReplaceBlockText(request) => {
                self.replacement_text_projection(page_id, request)
            }
            PageMutation::ReplaceSimpleTableCell(request) => simple_table_projection(request),
            PageMutation::ReplaceBlockTextAndConvert(request) => {
                self.converted_replacement_text_projection(page_id, request)
            }
            PageMutation::ConvertBlockToDivider(request) => PageWriteTextProjection::update(
                PageProjectedText::plain_block(request.continuation_block_id(), ""),
            )
            .with_retired_blocks([request.source_block_id().to_string()]),
            PageMutation::RestoreBlockFromDivider(request) => PageWriteTextProjection::update(
                PageProjectedText::plain_block(request.divider_block_id(), request.text()),
            )
            .with_retired_blocks([request.continuation_block_id().to_string()]),
            PageMutation::ReplaceBlockWithDivider(request) => PageWriteTextProjection::default()
                .with_retired_blocks([request.source_block_id().to_string()]),
            PageMutation::ReplaceBlockWithCode(request) => PageWriteTextProjection::update(
                PageProjectedText::plain_block(request.code_block_id(), ""),
            )
            .with_retired_blocks([request.source_block_id().to_string()]),
            PageMutation::RestoreBlockFromCode(request) => restore_code_projection(request),
            PageMutation::DeleteBlock(request) => {
                PageWriteTextProjection::default().with_retired_blocks([request.block_id.clone()])
            }
            PageMutation::InsertMention(request) => {
                self.inserted_mention_projection(page_id, request)
            }
            PageMutation::UpdateMention(request) => {
                self.updated_mention_projection(page_id, request)
            }
            PageMutation::DuplicateAlias(_)
            | PageMutation::ReplaceTextSelection(_)
            | PageMutation::PasteTextSelection(_)
            | PageMutation::BreakTextSelection(_)
            | PageMutation::SplitBlock(_)
            | PageMutation::MergeBlocks(_)
            | PageMutation::ConvertBlock(_)
            | PageMutation::SetToDoState(_)
            | PageMutation::SetCodeLanguage(_)
            | PageMutation::SetCodeWrap(_)
            | PageMutation::SetPageIcon(_)
            | PageMutation::SetBlockColor(_)
            | PageMutation::SetQuoteSize(_)
            | PageMutation::ResizeColumns(_)
            | PageMutation::ReorderSubtrees(_) => PageWriteTextProjection::default(),
        }
    }

    fn inserted_mention_projection(
        &self,
        page_id: &str,
        request: &InsertPageMentionRequest,
    ) -> PageWriteTextProjection {
        self.edited_block_projection(page_id, request.block_id(), |editable| {
            apply_mention_insert_to_editable(editable, request)
        })
    }

    fn updated_mention_projection(
        &self,
        page_id: &str,
        request: &UpdatePageMentionRequest,
    ) -> PageWriteTextProjection {
        self.edited_block_projection(page_id, request.block_id(), |editable| {
            apply_mention_update_to_editable(editable, request)
                .expect("queued mention update must target a mention token");
        })
    }

    /// Project the editable block after applying `edit` to its current text.
    fn edited_block_projection(
        &self,
        page_id: &str,
        block_id: &str,
        edit: impl FnOnce(&mut crate::model::CardPageEditableBlock),
    ) -> PageWriteTextProjection {
        let data = self
            .active_page_data_with_id(page_id)
            .expect("queued mention edit must target a loaded page");
        let index = data
            .editable_block_indices
            .get(block_id)
            .copied()
            .expect("queued mention edit must target an editable block");
        let mut editable = data.page.blocks[index]
            .editable_content()
            .expect("indexed mention edit target must remain editable")
            .clone();
        edit(&mut editable);
        PageWriteTextProjection::update(
            PageProjectedText::editable(block_id.to_string(), &editable)
                .expect("projected mention edit annotations must remain valid"),
        )
    }

    fn converted_replacement_text_projection(
        &self,
        page_id: &str,
        request: &ReplacePageBlockTextAndConvertRequest,
    ) -> PageWriteTextProjection {
        self.replacement_text_projection(
            page_id,
            &ReplacePageBlockTextRequest {
                block_id: request.block_id.clone(),
                text: request.text.clone(),
            },
        )
    }

    fn replacement_text_projection(
        &self,
        page_id: &str,
        request: &ReplacePageBlockTextRequest,
    ) -> PageWriteTextProjection {
        if request.block_id == page_id {
            return PageWriteTextProjection::update(PageProjectedText::title(request.text.clone()));
        }
        let data = self
            .active_page_data_with_id(page_id)
            .expect("queued text replacement must target a loaded page");
        let index = data
            .editable_block_indices
            .get(&request.block_id)
            .copied()
            .expect("queued text replacement must target an editable block");
        let mut editable = data.page.blocks[index]
            .editable_content()
            .expect("indexed text replacement target must remain editable")
            .clone();
        let previous = editable.text.clone();
        apply_plain_text_annotation_replacement(&mut editable, &previous, &request.text);
        editable.text.clone_from(&request.text);
        PageWriteTextProjection::update(
            PageProjectedText::editable(request.block_id.clone(), &editable)
                .expect("projected text replacement annotations must remain valid"),
        )
    }
}

fn restore_code_projection(request: &RestorePageBlockFromCodeRequest) -> PageWriteTextProjection {
    PageWriteTextProjection::update(PageProjectedText::plain_block(
        request.source_block_id(),
        request.source_text().as_str(),
    ))
    .with_retired_blocks([request.code_block_id().to_string()])
}

fn simple_table_projection(request: &ReplacePageSimpleTableCellRequest) -> PageWriteTextProjection {
    PageWriteTextProjection::update(
        PageProjectedText::simple_table_cell(request.target().clone(), request.cell())
            .expect("writable table-cell annotations must remain canonicalizable"),
    )
}
