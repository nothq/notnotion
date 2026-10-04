use std::collections::HashSet;

use super::draft::NotionCommentDraft;
use super::identities::{NotionCommentId, NotionCommentTargetId, NotionDiscussionId};
use crate::model::CardPage;

#[derive(Clone, Debug)]
pub struct NotionCommentMutationRequest {
    page_id: NotionCommentTargetId,
    target_id: NotionCommentTargetId,
    expected_discussion_ids: Vec<NotionDiscussionId>,
    mutation: NotionCommentMutation,
    draft: NotionCommentDraft,
}

#[derive(Clone, Debug)]
pub(crate) enum NotionCommentMutation {
    NewThread,
    Reply {
        discussion_id: NotionDiscussionId,
        expected_comment_ids: Vec<NotionCommentId>,
    },
}

impl NotionCommentMutationRequest {
    pub fn new_thread(
        page: &CardPage,
        target_id: NotionCommentTargetId,
        draft: NotionCommentDraft,
    ) -> Result<Self, String> {
        Self::new(page, target_id, NotionCommentMutation::NewThread, draft)
    }

    pub fn reply(
        page: &CardPage,
        target_id: NotionCommentTargetId,
        discussion_id: NotionDiscussionId,
        draft: NotionCommentDraft,
    ) -> Result<Self, String> {
        let discussion = page
            .discussions
            .iter()
            .find(|discussion| discussion.discussion_id == discussion_id)
            .ok_or_else(|| missing_discussion(page, &discussion_id))?;
        validate_reply_target(discussion, &target_id, &discussion_id)?;
        let expected_comment_ids = discussion
            .comments
            .iter()
            .map(|comment| comment.comment_id.clone())
            .collect();
        Self::new(
            page,
            target_id,
            NotionCommentMutation::Reply {
                discussion_id,
                expected_comment_ids,
            },
            draft,
        )
    }

    fn new(
        page: &CardPage,
        target_id: NotionCommentTargetId,
        mutation: NotionCommentMutation,
        draft: NotionCommentDraft,
    ) -> Result<Self, String> {
        validate_target(page, &target_id)?;
        let page_id = page.block_id.parse().map_err(str::to_string)?;
        let expected_discussion_ids = expected_discussion_ids(page, &target_id)?;
        Ok(Self {
            page_id,
            target_id,
            expected_discussion_ids,
            mutation,
            draft,
        })
    }

    pub(crate) fn page_id(&self) -> &NotionCommentTargetId {
        &self.page_id
    }

    pub(crate) fn target_id(&self) -> &NotionCommentTargetId {
        &self.target_id
    }

    pub(crate) fn expected_discussion_ids(&self) -> &[NotionDiscussionId] {
        &self.expected_discussion_ids
    }

    pub(crate) fn mutation(&self) -> &NotionCommentMutation {
        &self.mutation
    }

    pub(crate) fn draft(&self) -> &NotionCommentDraft {
        &self.draft
    }
}

fn missing_discussion(page: &CardPage, discussion_id: &NotionDiscussionId) -> String {
    format!(
        "Notion discussion {} is not loaded in page {}",
        discussion_id.as_str(),
        page.block_id
    )
}

fn validate_reply_target(
    discussion: &super::projection::CardPageDiscussion,
    target_id: &NotionCommentTargetId,
    discussion_id: &NotionDiscussionId,
) -> Result<(), String> {
    if discussion.target_block_id != *target_id {
        return Err(format!(
            "Notion discussion {} belongs to target {}, not {}",
            discussion_id.as_str(),
            discussion.target_block_id.as_str(),
            target_id.as_str()
        ));
    }
    if !discussion.access.is_writable() {
        return Err(format!(
            "Notion discussion {} is read-only",
            discussion_id.as_str()
        ));
    }
    Ok(())
}

fn validate_target(page: &CardPage, target_id: &NotionCommentTargetId) -> Result<(), String> {
    if page.block_id == target_id.as_str()
        || page
            .blocks
            .iter()
            .any(|block| block.block_id == target_id.as_str())
    {
        return Ok(());
    }
    Err(format!(
        "Notion comment target {} is not loaded in page {}",
        target_id.as_str(),
        page.block_id
    ))
}

fn expected_discussion_ids(
    page: &CardPage,
    target_id: &NotionCommentTargetId,
) -> Result<Vec<NotionDiscussionId>, String> {
    let mut seen = HashSet::new();
    page.discussions
        .iter()
        .filter(|discussion| discussion.target_block_id == *target_id)
        .map(|discussion| discussion.discussion_id.clone())
        .map(|discussion_id| {
            if !seen.insert(discussion_id.clone()) {
                return Err(format!(
                    "Notion page {} contains duplicate discussion {}",
                    page.block_id,
                    discussion_id.as_str()
                ));
            }
            Ok(discussion_id)
        })
        .collect()
}
