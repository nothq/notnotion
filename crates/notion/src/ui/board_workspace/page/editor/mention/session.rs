use gpui::Context;

use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::actions::{PageMentionMenuCommit, PageMentionMutationEffect, PageMentionPickerEffect};
use super::commit::PageMentionMenuHostUpdate;
use super::picker::commit::{PageMentionPickerFinish, PageMentionPickerHostUpdate};
use crate::ui::surface::PageEditorState;
use crate::ui::SurfaceState;

pub(super) enum PageMentionEditFinish {
    None,
    SyncPickerField,
    ClosePickerAndFocus { block_id: String, cursor: usize },
}

pub(super) struct PageMentionPostEdit {
    pub(super) focus: Option<(String, usize)>,
    pub(super) notify: bool,
}

impl PageMentionEditFinish {
    pub(super) fn apply(
        self,
        editor: &mut PageEditorState,
        cx: &mut Context<SurfaceState>,
    ) -> PageMentionPostEdit {
        match self {
            Self::None => PageMentionPostEdit {
                focus: None,
                notify: false,
            },
            Self::SyncPickerField => {
                editor.mention.sync_picker_field_text(cx);
                PageMentionPostEdit {
                    focus: None,
                    notify: true,
                }
            }
            Self::ClosePickerAndFocus { block_id, cursor } => {
                editor.mention.close_picker();
                PageMentionPostEdit {
                    focus: Some((block_id, cursor)),
                    notify: true,
                }
            }
        }
    }
}

impl PageEditSession<'_> {
    pub(super) fn apply_page_mention_mutation(
        &mut self,
        effect: PageMentionMutationEffect,
    ) -> PageMentionEditFinish {
        match effect {
            PageMentionMutationEffect::CommitMenu(commit) => {
                self.apply_page_mention_menu_commit(commit)
            }
            PageMentionMutationEffect::Picker(effect) => {
                self.apply_page_mention_picker_effect(effect)
            }
            PageMentionMutationEffect::InviteUnavailable => {
                self.effects.push(PageEditEffect::Error(
                    "Inviting members is not available in notnotion yet.".to_string(),
                ));
                PageMentionEditFinish::None
            }
        }
    }

    fn apply_page_mention_menu_commit(
        &mut self,
        commit: PageMentionMenuCommit,
    ) -> PageMentionEditFinish {
        let page = self.documents.page_containing_block(&commit.menu.block_id);
        match self.editor.prepare_page_mention_menu_commit(page, commit) {
            PageMentionMenuHostUpdate::None => {}
            PageMentionMenuHostUpdate::Notify => self.effects.push(PageEditEffect::Notify),
            PageMentionMenuHostUpdate::Apply(plan) => {
                let plan = *plan;
                for mutation in plan.mutations {
                    self.effects.push(PageEditEffect::Write(
                        PageEditWriteEffect::EnqueueMutation {
                            page_id: plan.page_id.clone(),
                            mutation,
                        },
                    ));
                }
                self.effects
                    .push(PageEditEffect::ReplaceLoadedPage(plan.page));
                self.effects.push(PageEditEffect::FocusBlock {
                    block_id: plan.focus_block_id,
                    offset: plan.cursor,
                });
                self.effects.push(PageEditEffect::Notify);
            }
        }
        PageMentionEditFinish::None
    }

    fn apply_page_mention_picker_effect(
        &mut self,
        effect: PageMentionPickerEffect,
    ) -> PageMentionEditFinish {
        let block_id = match &effect {
            PageMentionPickerEffect::Commit(picker) | PageMentionPickerEffect::Clear(picker) => {
                &picker.block_id
            }
        };
        let page = self.documents.page_containing_block(block_id);
        match self.editor.prepare_page_mention_picker_effect(page, effect) {
            PageMentionPickerHostUpdate::None => PageMentionEditFinish::None,
            PageMentionPickerHostUpdate::Notify => {
                self.effects.push(PageEditEffect::Notify);
                PageMentionEditFinish::None
            }
            PageMentionPickerHostUpdate::SyncField => PageMentionEditFinish::SyncPickerField,
            PageMentionPickerHostUpdate::Apply { plan, finish } => {
                let plan = *plan;
                let finish = match finish {
                    PageMentionPickerFinish::SyncField => PageMentionEditFinish::SyncPickerField,
                    PageMentionPickerFinish::CloseAndFocus => {
                        PageMentionEditFinish::ClosePickerAndFocus {
                            block_id: plan.focus_block_id.clone(),
                            cursor: plan.cursor,
                        }
                    }
                };
                self.effects.push(PageEditEffect::Write(
                    PageEditWriteEffect::EnqueueMutation {
                        page_id: plan.page_id,
                        mutation: plan.mutation,
                    },
                ));
                self.effects
                    .push(PageEditEffect::ReplaceLoadedPage(plan.page));
                finish
            }
        }
    }
}
