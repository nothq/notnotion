use serde::{Deserialize, Serialize};

use super::identities::{NotionCommentId, NotionCommentTargetId, NotionDiscussionId};
use crate::model::NotionUserId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardPageCommentAuthor {
    pub user_id: NotionUserId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_photo: Option<String>,
}

impl CardPageCommentAuthor {
    pub(crate) fn new(
        user_id: NotionUserId,
        name: String,
        profile_photo: Option<String>,
    ) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err(format!(
                "Notion comment author {} has an empty display name",
                user_id.as_str()
            ));
        }
        Ok(Self {
            user_id,
            name,
            profile_photo,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CardPageCommentRichSegment {
    Plain {
        text: String,
    },
    Mention {
        user_id: NotionUserId,
        display_name: String,
    },
    Unsupported {
        display: String,
    },
}

impl CardPageCommentRichSegment {
    pub(crate) fn plain(text: String) -> Result<Self, String> {
        if text.is_empty() {
            return Err("Notion comment plain-text segment must not be empty".to_string());
        }
        Ok(Self::Plain { text })
    }

    pub(crate) fn mention(user_id: NotionUserId, display_name: String) -> Result<Self, String> {
        if display_name.trim().is_empty() {
            return Err(format!(
                "Notion comment mention {} has an empty display name",
                user_id.as_str()
            ));
        }
        Ok(Self::Mention {
            user_id,
            display_name,
        })
    }

    pub(crate) fn unsupported(display: String) -> Self {
        Self::Unsupported { display }
    }

    pub fn display_text(&self) -> String {
        match self {
            Self::Plain { text } => text.clone(),
            Self::Mention { display_name, .. } => format!("@{display_name}"),
            Self::Unsupported { display } => display.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardPageComment {
    pub comment_id: NotionCommentId,
    pub author: CardPageCommentAuthor,
    pub created_time_ms: u64,
    pub last_edited_time_ms: u64,
    pub rich_text: Vec<CardPageCommentRichSegment>,
}

impl CardPageComment {
    pub(crate) fn new(
        comment_id: NotionCommentId,
        author: CardPageCommentAuthor,
        created_time_ms: u64,
        last_edited_time_ms: u64,
        rich_text: Vec<CardPageCommentRichSegment>,
    ) -> Result<Self, String> {
        if rich_text.is_empty() {
            return Err(format!(
                "Notion comment {} has empty rich text",
                comment_id.as_str()
            ));
        }
        Ok(Self {
            comment_id,
            author,
            created_time_ms,
            last_edited_time_ms,
            rich_text,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "access", rename_all = "snake_case")]
pub enum CardPageDiscussionAccess {
    Writable,
    ReadOnly { reason: String },
}

impl CardPageDiscussionAccess {
    pub const fn is_writable(&self) -> bool {
        matches!(self, Self::Writable)
    }

    pub(crate) fn for_discussion(
        resolved: bool,
        discussion_type: &str,
        property_id: Option<&str>,
    ) -> Self {
        if property_id.is_some() {
            return Self::ReadOnly {
                reason: "Property comments are not supported yet".to_string(),
            };
        }
        if resolved {
            return Self::ReadOnly {
                reason: "Resolved discussion".to_string(),
            };
        }
        if discussion_type == "default" {
            return Self::Writable;
        }
        Self::ReadOnly {
            reason: format!("Unsupported discussion type: {discussion_type}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CardPageDiscussion {
    pub discussion_id: NotionDiscussionId,
    pub target_block_id: NotionCommentTargetId,
    pub resolved: bool,
    pub access: CardPageDiscussionAccess,
    pub comments: Vec<CardPageComment>,
}

impl CardPageDiscussion {
    pub(crate) fn new(
        discussion_id: NotionDiscussionId,
        target_block_id: NotionCommentTargetId,
        resolved: bool,
        access: CardPageDiscussionAccess,
        comments: Vec<CardPageComment>,
    ) -> Self {
        Self {
            discussion_id,
            target_block_id,
            resolved,
            access,
            comments,
        }
    }
}
