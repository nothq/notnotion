use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

use gpui::{px, Entity, ListAlignment, ListState, SharedString, UniformListScrollHandle};
use gpui_components::text_input::TextInput;

use crate::model::{
    CardPage, NotionCommentDraft, NotionCommentDraftSegment, NotionCommentTargetId,
    NotionDiscussionId, NotionUserId, NotionWorkspaceUser,
};

mod rows;

use rows::{comment_list_rows, comment_thread_rows};

#[derive(Clone)]
pub(crate) enum NotionCommentSegmentRow {
    Plain(SharedString),
    Mention(SharedString),
    Unsupported(SharedString),
}

#[derive(Clone)]
pub(crate) struct NotionCommentRow {
    pub(crate) stable_id: SharedString,
    pub(crate) author: SharedString,
    pub(crate) author_initial: SharedString,
    pub(crate) timestamp: SharedString,
    pub(crate) segments: Arc<[NotionCommentSegmentRow]>,
}

#[derive(Clone)]
pub(crate) struct NotionCommentThreadRow {
    pub(crate) stable_id: SharedString,
    pub(crate) discussion_id: NotionDiscussionId,
    pub(crate) comments: Arc<[NotionCommentRow]>,
    pub(crate) replyable: bool,
    pub(crate) read_only_reason: Option<SharedString>,
}

#[derive(Clone)]
pub(crate) enum NotionCommentsListRow {
    ThreadHeader {
        stable_id: SharedString,
        read_only_reason: Option<SharedString>,
    },
    Comment {
        comment: NotionCommentRow,
    },
    Reply {
        stable_id: SharedString,
        discussion_id: NotionDiscussionId,
        enabled: bool,
    },
}

#[derive(Clone)]
pub(crate) struct NotionCommentMentionRow {
    pub(crate) user_id: NotionUserId,
    pub(crate) label: SharedString,
    pub(crate) initial: SharedString,
}

pub(crate) struct NotionCommentsPanelState {
    pub(crate) session: Arc<()>,
    pub(crate) page_id: NotionCommentTargetId,
    pub(crate) target_id: NotionCommentTargetId,
    pub(crate) can_comment: bool,
    pub(crate) threads: Arc<[NotionCommentThreadRow]>,
    pub(crate) rows: Arc<[NotionCommentsListRow]>,
    pub(crate) thread_list_state: ListState,
    pub(crate) visible_users: Arc<[NotionWorkspaceUser]>,
    pub(crate) mention_rows: Arc<[NotionCommentMentionRow]>,
    pub(crate) mention_scroll_handle: UniformListScrollHandle,
    pub(crate) reply_to: Option<NotionDiscussionId>,
    pub(crate) committed_segments: Vec<NotionCommentDraftSegment>,
    pub(crate) plain_tail: String,
    pub(crate) mention_start: Option<usize>,
    pub(crate) input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) focus_requested: Cell<bool>,
    pub(crate) mutation_in_flight: bool,
}

#[derive(Default)]
pub(crate) struct NotionCommentsState {
    pub(crate) panel: Option<NotionCommentsPanelState>,
}

impl NotionCommentsState {
    pub(crate) fn reconcile_page(&mut self, page: &CardPage) {
        let target_invalid = self.panel.as_mut().is_some_and(|panel| {
            panel.page_id.as_str() == page.block_id && panel.replace_page(page).is_err()
        });
        if target_invalid {
            self.dismiss_notion_comments_panel_without_notify();
        }
    }
}

impl NotionCommentsPanelState {
    pub(crate) fn new(page: &CardPage, target_id: NotionCommentTargetId) -> Result<Self, String> {
        validate_target(page, &target_id)?;
        let threads = comment_thread_rows(page, &target_id);
        let rows = comment_list_rows(&threads);
        Ok(Self {
            session: Arc::new(()),
            page_id: page.block_id.parse().map_err(str::to_string)?,
            target_id,
            can_comment: page.comments_writable,
            thread_list_state: ListState::new(rows.len(), ListAlignment::Top, px(112.0)),
            threads,
            rows,
            visible_users: Arc::new([]),
            mention_rows: Arc::new([]),
            mention_scroll_handle: UniformListScrollHandle::new(),
            reply_to: None,
            committed_segments: Vec::new(),
            plain_tail: String::new(),
            mention_start: None,
            input: RefCell::new(None),
            focus_requested: Cell::new(true),
            mutation_in_flight: false,
        })
    }

    pub(crate) fn replace_page(&mut self, page: &CardPage) -> Result<(), String> {
        if page.block_id != self.page_id.as_str() {
            return Err(format!(
                "Notion comments panel expected page {}, received {}",
                self.page_id.as_str(),
                page.block_id
            ));
        }
        validate_target(page, &self.target_id)?;
        self.can_comment = page.comments_writable;
        self.threads = comment_thread_rows(page, &self.target_id);
        self.rows = comment_list_rows(&self.threads);
        self.thread_list_state.reset(self.rows.len());
        if self.reply_to.as_ref().is_some_and(|discussion_id| {
            !self
                .threads
                .iter()
                .any(|thread| thread.discussion_id == *discussion_id && thread.replyable)
        }) {
            self.reply_to = None;
        }
        Ok(())
    }

    pub(crate) fn replace_users(&mut self, users: Vec<NotionWorkspaceUser>) {
        self.visible_users = users.into();
        self.rebuild_mention_rows();
    }

    pub(crate) fn set_plain_tail(&mut self, value: String) {
        self.plain_tail = value;
        self.mention_start = mention_start(&self.plain_tail);
        self.rebuild_mention_rows();
    }

    pub(crate) fn select_mention(&mut self, user_id: &NotionUserId) -> Result<(), String> {
        let mention_start = self
            .mention_start
            .ok_or_else(|| "Notion comment mention picker is not active".to_string())?;
        if !self
            .visible_users
            .iter()
            .any(|user| user.user_id == *user_id)
        {
            return Err(format!(
                "Notion mention user {} is not in the loaded workspace set",
                user_id.as_str()
            ));
        }
        let prefix = self.plain_tail[..mention_start].to_string();
        if !prefix.is_empty() {
            self.committed_segments
                .push(NotionCommentDraftSegment::Plain(prefix));
        }
        self.committed_segments
            .push(NotionCommentDraftSegment::Mention(user_id.clone()));
        self.plain_tail.clear();
        self.mention_start = None;
        self.mention_rows = Arc::new([]);
        self.mention_scroll_handle = UniformListScrollHandle::new();
        self.focus_requested.set(true);
        Ok(())
    }

    pub(crate) fn remove_most_recent_mention(&mut self) -> bool {
        let Some(index) = self
            .committed_segments
            .iter()
            .rposition(|segment| matches!(segment, NotionCommentDraftSegment::Mention(_)))
        else {
            return false;
        };
        self.committed_segments.remove(index);
        self.focus_requested.set(true);
        true
    }

    pub(crate) fn draft(&self) -> Result<NotionCommentDraft, String> {
        let mut segments = self.committed_segments.clone();
        if !self.plain_tail.is_empty() {
            segments.push(NotionCommentDraftSegment::Plain(self.plain_tail.clone()));
        }
        NotionCommentDraft::new(segments)
    }

    pub(crate) fn reset_composer(&mut self) {
        self.reply_to = None;
        self.committed_segments.clear();
        self.plain_tail.clear();
        self.mention_start = None;
        self.mention_rows = Arc::new([]);
        self.mention_scroll_handle = UniformListScrollHandle::new();
        self.input.borrow_mut().take();
        self.focus_requested.set(true);
        self.mutation_in_flight = false;
    }

    pub(crate) fn mention_label(&self, user_id: &NotionUserId) -> SharedString {
        self.visible_users
            .iter()
            .find(|user| user.user_id == *user_id)
            .map(|user| SharedString::from(format!("@{}", user.name)))
            .unwrap_or_else(|| SharedString::from("@Unavailable user"))
    }

    fn rebuild_mention_rows(&mut self) {
        let Some(start) = self.mention_start else {
            self.mention_rows = Arc::new([]);
            return;
        };
        let query = self.plain_tail[start + '@'.len_utf8()..].to_lowercase();
        self.mention_rows = self
            .visible_users
            .iter()
            .filter(|user| query.is_empty() || user.name.to_lowercase().contains(&query))
            .map(|user| NotionCommentMentionRow {
                user_id: user.user_id.clone(),
                label: SharedString::from(user.name.clone()),
                initial: SharedString::from(
                    user.name
                        .chars()
                        .find(|character| !character.is_whitespace())
                        .map(|character| character.to_uppercase().to_string())
                        .unwrap_or_else(|| "?".to_string()),
                ),
            })
            .collect::<Vec<_>>()
            .into();
        self.mention_scroll_handle = UniformListScrollHandle::new();
    }
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

fn mention_start(value: &str) -> Option<usize> {
    let start = value.rfind('@')?;
    let suffix = &value[start + '@'.len_utf8()..];
    if suffix.chars().any(char::is_whitespace) {
        return None;
    }
    if start > 0
        && !value[..start]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    Some(start)
}
