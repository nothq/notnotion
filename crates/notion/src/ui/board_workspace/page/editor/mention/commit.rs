use super::super::generated_notion_record_id;
use super::super::rich_text::annotations::apply_mention_insert_to_editable;
use super::actions::PageMentionMenuCommit;
use super::state::PageMentionMenuIdentity;
use super::PageMentionMenuRow;
use crate::model::{
    CreatePageBlockRequest, InsertPageMentionRequest, NotionPageBlockKind, PageBlockPlacement,
    PageMention, PageMutation,
};
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPage, CardPageBlock, CardPageBlockKind, PageEditFocus};

struct PreparedMentionInsertion {
    page: CardPage,
    block_index: usize,
    menu: crate::ui::PageMentionMenuState,
    mention: PageMention,
    new_page: Option<NewMentionPage>,
}

struct NewMentionPage {
    block_id: String,
    title: String,
}

pub(super) struct PageMentionCommitPlan {
    pub(super) page_id: String,
    pub(super) page: CardPage,
    pub(super) mutations: Vec<PageMutation>,
    pub(super) focus_block_id: String,
    pub(super) cursor: usize,
}

enum MentionInsertionPreparation {
    MissingTarget,
    InvalidText,
    Ready(Box<PreparedMentionInsertion>),
}

pub(super) enum PageMentionMenuHostUpdate {
    None,
    Notify,
    Apply(Box<PageMentionCommitPlan>),
}

impl PageEditorState {
    pub(super) fn prepare_page_mention_menu_commit(
        &mut self,
        page: Option<CardPage>,
        commit: PageMentionMenuCommit,
    ) -> PageMentionMenuHostUpdate {
        let identity = PageMentionMenuIdentity::new(&commit.menu);
        let Some(page) = page else {
            return PageMentionMenuHostUpdate::None;
        };
        let prepared = match prepare_mention_insertion(page, commit) {
            MentionInsertionPreparation::MissingTarget => return PageMentionMenuHostUpdate::None,
            MentionInsertionPreparation::InvalidText => {
                return if self.mention.dismiss_menu(&identity) {
                    PageMentionMenuHostUpdate::Notify
                } else {
                    PageMentionMenuHostUpdate::None
                };
            }
            MentionInsertionPreparation::Ready(prepared) => *prepared,
        };
        if !self.mention.finish_menu_commit(&identity) {
            return PageMentionMenuHostUpdate::None;
        }
        self.record_page_structural_edit(prepared.page(), Some(prepared.focus_before_edit()));
        PageMentionMenuHostUpdate::Apply(Box::new(prepared.finish()))
    }
}

impl PreparedMentionInsertion {
    fn page(&self) -> &CardPage {
        &self.page
    }

    fn focus_before_edit(&self) -> PageEditFocus {
        PageEditFocus::block(&self.menu.block_id, self.menu.trigger_offset)
    }

    fn finish(mut self) -> PageMentionCommitPlan {
        let page_id = self.page.block_id.clone();
        let start = self.menu.trigger_offset;
        let end = start + 1 + self.menu.query.len();
        let request = InsertPageMentionRequest::new(
            self.menu.block_id.clone(),
            start,
            end,
            self.mention,
            true,
        )
        .expect("a validated mention menu range forms a valid insertion request");
        let mut mutations = Vec::with_capacity(1 + usize::from(self.new_page.is_some()));
        if let Some(new_page) = &self.new_page {
            mutations.push(PageMutation::CreateBlock(CreatePageBlockRequest {
                block_id: new_page.block_id.clone(),
                parent_block_id: page_id.clone(),
                kind: NotionPageBlockKind::Page,
                text: new_page.title.clone(),
                placement: PageBlockPlacement::Append,
            }));
        }
        mutations.push(PageMutation::InsertMention(request.clone()));
        let editable = self.page.blocks[self.block_index]
            .editable_content_mut()
            .expect("validated mention target must remain editable");
        apply_mention_insert_to_editable(editable, &request);
        if let Some(new_page) = self.new_page {
            self.page.blocks.push(CardPageBlock::editable(
                new_page.block_id,
                page_id.clone(),
                0,
                CardPageBlockKind::PageLink,
                new_page.title,
            ));
        }
        let cursor = start + request.inserted_text().len();
        PageMentionCommitPlan {
            page_id,
            page: self.page,
            mutations,
            focus_block_id: self.menu.block_id,
            cursor,
        }
    }
}

fn prepare_mention_insertion(
    page: CardPage,
    commit: PageMentionMenuCommit,
) -> MentionInsertionPreparation {
    let PageMentionMenuCommit { menu, row } = commit;
    let Some(block_index) = page
        .blocks
        .iter()
        .position(|block| block.block_id == menu.block_id)
    else {
        return MentionInsertionPreparation::MissingTarget;
    };
    let valid = page.blocks[block_index]
        .editable_content()
        .is_some_and(|editable| {
            let end = menu.trigger_offset + 1 + menu.query.len();
            end <= editable.text.len()
                && editable.text.is_char_boundary(menu.trigger_offset)
                && editable.text.is_char_boundary(end)
                && editable.text[menu.trigger_offset..end].strip_prefix('@')
                    == Some(menu.query.as_str())
        });
    if !valid {
        return MentionInsertionPreparation::InvalidText;
    }
    let Some((mention, new_page)) = mention_target_for_row(row) else {
        return MentionInsertionPreparation::InvalidText;
    };
    MentionInsertionPreparation::Ready(Box::new(PreparedMentionInsertion {
        page,
        block_index,
        menu,
        mention,
        new_page,
    }))
}

type MentionTarget = (PageMention, Option<NewMentionPage>);

fn mention_target_for_row(row: PageMentionMenuRow) -> Option<MentionTarget> {
    Some(match row {
        PageMentionMenuRow::Date { mention, .. } | PageMentionMenuRow::Reminder { mention, .. } => {
            (PageMention::Date(mention), None)
        }
        PageMentionMenuRow::Person { user } => (
            PageMention::User {
                user_id: user.user_id.as_str().to_string(),
                display_name: user.name,
            },
            None,
        ),
        PageMentionMenuRow::Page { result } => (
            PageMention::Page {
                block_id: result.block_id,
                title: result.title,
            },
            None,
        ),
        PageMentionMenuRow::NewPage { title } => {
            let block_id = generated_notion_record_id();
            (
                PageMention::Page {
                    block_id: block_id.clone(),
                    title: title.clone(),
                },
                Some(NewMentionPage { block_id, title }),
            )
        }
        PageMentionMenuRow::Invite { .. } => return None,
    })
}
