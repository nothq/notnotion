use super::super::super::rich_text::annotations::{
    apply_mention_update_to_editable, apply_plain_text_annotation_replacement, mention_at_offset,
};
use super::super::actions::PageMentionPickerEffect;
use super::super::state::PageMentionPickerIdentity;
use crate::model::{
    PageMention, PageMutation, ReplacePageBlockTextRequest, UpdatePageMentionRequest,
    PAGE_MENTION_TOKEN_STR,
};
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPage, PageEditFocus, PageMentionPickerState};

enum PreparedPickerUpdate {
    Unchanged,
    Update(Box<PreparedPickerMutation>),
}

struct PreparedPickerMutation {
    page: CardPage,
    block_index: usize,
    picker: PageMentionPickerState,
    mutation: PageMutation,
    replacement: PickerReplacement,
}

enum PickerReplacement {
    Mention(UpdatePageMentionRequest),
    Text { previous: String, next: String },
}

pub(in crate::ui::board_workspace::page::editor) struct PageMentionPickerCommitPlan {
    pub(in crate::ui::board_workspace::page::editor) page_id: String,
    pub(in crate::ui::board_workspace::page::editor) page: CardPage,
    pub(in crate::ui::board_workspace::page::editor) mutation: PageMutation,
    pub(in crate::ui::board_workspace::page::editor) focus_block_id: String,
    pub(in crate::ui::board_workspace::page::editor) cursor: usize,
}

pub(in crate::ui::board_workspace::page::editor) enum PageMentionPickerHostUpdate {
    None,
    Notify,
    SyncField,
    Apply {
        plan: Box<PageMentionPickerCommitPlan>,
        finish: PageMentionPickerFinish,
    },
}

pub(in crate::ui::board_workspace::page::editor) enum PageMentionPickerFinish {
    SyncField,
    CloseAndFocus,
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor::mention) fn prepare_page_mention_picker_effect(
        &mut self,
        page: Option<CardPage>,
        effect: PageMentionPickerEffect,
    ) -> PageMentionPickerHostUpdate {
        let picker = match &effect {
            PageMentionPickerEffect::Commit(picker) | PageMentionPickerEffect::Clear(picker) => {
                picker
            }
        };
        if !self
            .mention
            .picker_matches(&PageMentionPickerIdentity::new(picker))
        {
            return PageMentionPickerHostUpdate::None;
        }
        let Some(page) = page else {
            return self.closed_page_mention_picker_update();
        };
        match effect {
            PageMentionPickerEffect::Commit(picker) => {
                self.prepare_page_mention_picker_commit(page, picker)
            }
            PageMentionPickerEffect::Clear(picker) => {
                self.prepare_page_mention_picker_clear(page, picker)
            }
        }
    }

    fn prepare_page_mention_picker_commit(
        &mut self,
        page: CardPage,
        picker: PageMentionPickerState,
    ) -> PageMentionPickerHostUpdate {
        match prepare_picker_update(page, picker) {
            None => self.closed_page_mention_picker_update(),
            Some(PreparedPickerUpdate::Unchanged) => PageMentionPickerHostUpdate::SyncField,
            Some(PreparedPickerUpdate::Update(prepared)) => {
                let prepared = *prepared;
                self.record_page_structural_edit(
                    prepared.page(),
                    Some(prepared.focus_before_edit()),
                );
                PageMentionPickerHostUpdate::Apply {
                    plan: Box::new(prepared.finish()),
                    finish: PageMentionPickerFinish::SyncField,
                }
            }
        }
    }

    fn prepare_page_mention_picker_clear(
        &mut self,
        page: CardPage,
        picker: PageMentionPickerState,
    ) -> PageMentionPickerHostUpdate {
        let Some(prepared) = prepare_picker_clear(page, picker) else {
            return self.closed_page_mention_picker_update();
        };
        self.record_page_structural_edit(prepared.page(), Some(prepared.focus_before_edit()));
        PageMentionPickerHostUpdate::Apply {
            plan: Box::new(prepared.finish()),
            finish: PageMentionPickerFinish::CloseAndFocus,
        }
    }

    fn closed_page_mention_picker_update(&mut self) -> PageMentionPickerHostUpdate {
        if self.mention.close_picker() {
            PageMentionPickerHostUpdate::Notify
        } else {
            PageMentionPickerHostUpdate::None
        }
    }
}

impl PreparedPickerMutation {
    fn page(&self) -> &CardPage {
        &self.page
    }

    fn focus_before_edit(&self) -> PageEditFocus {
        PageEditFocus::block(&self.picker.block_id, self.picker.offset_utf8)
    }

    fn finish(mut self) -> PageMentionPickerCommitPlan {
        let editable = self.page.blocks[self.block_index]
            .editable_content_mut()
            .expect("validated picker target must remain editable");
        match &self.replacement {
            PickerReplacement::Mention(request) => {
                apply_mention_update_to_editable(editable, request)
                    .expect("validated picker target holds a mention token");
            }
            PickerReplacement::Text { previous, next } => {
                apply_plain_text_annotation_replacement(editable, previous, next);
                editable.text.clone_from(next);
            }
        }
        PageMentionPickerCommitPlan {
            page_id: self.page.block_id.clone(),
            page: self.page,
            mutation: self.mutation,
            focus_block_id: self.picker.block_id,
            cursor: self.picker.offset_utf8,
        }
    }
}

fn prepare_picker_update(
    page: CardPage,
    picker: PageMentionPickerState,
) -> Option<PreparedPickerUpdate> {
    let block_index = page
        .blocks
        .iter()
        .position(|block| block.block_id == picker.block_id)?;
    let current = page.blocks[block_index]
        .editable_content()
        .and_then(|editable| mention_at_offset(editable, picker.offset_utf8))
        .cloned();
    let Some(PageMention::Date(current)) = current else {
        return None;
    };
    if current == picker.draft {
        return Some(PreparedPickerUpdate::Unchanged);
    }
    let request = UpdatePageMentionRequest::new(
        picker.block_id.clone(),
        picker.offset_utf8,
        PageMention::Date(picker.draft.clone()),
    )
    .expect("an open picker targets a valid block");
    Some(PreparedPickerUpdate::Update(Box::new(
        PreparedPickerMutation {
            page,
            block_index,
            picker,
            mutation: PageMutation::UpdateMention(request.clone()),
            replacement: PickerReplacement::Mention(request),
        },
    )))
}

fn prepare_picker_clear(
    page: CardPage,
    picker: PageMentionPickerState,
) -> Option<PreparedPickerMutation> {
    let block_index = page
        .blocks
        .iter()
        .position(|block| block.block_id == picker.block_id)?;
    let previous = page.blocks[block_index]
        .editable_content()
        .map(|editable| editable.text.clone())?;
    if !previous
        .get(picker.offset_utf8..)
        .is_some_and(|text| text.starts_with(PAGE_MENTION_TOKEN_STR))
    {
        return None;
    }
    let token_end = picker.offset_utf8 + PAGE_MENTION_TOKEN_STR.len();
    let next = format!(
        "{}{}",
        &previous[..picker.offset_utf8],
        &previous[token_end..]
    );
    let mutation = PageMutation::ReplaceBlockText(ReplacePageBlockTextRequest {
        block_id: picker.block_id.clone(),
        text: next.clone(),
    });
    Some(PreparedPickerMutation {
        page,
        block_index,
        picker,
        mutation,
        replacement: PickerReplacement::Text { previous, next },
    })
}
