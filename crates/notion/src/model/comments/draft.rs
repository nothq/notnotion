use crate::model::{NotionUserId, NotionWorkspaceUser};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotionCommentDraftSegment {
    Plain(String),
    Mention(NotionUserId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotionCommentDraft {
    segments: Vec<NotionCommentDraftSegment>,
}

impl NotionCommentDraft {
    pub fn new(segments: Vec<NotionCommentDraftSegment>) -> Result<Self, String> {
        let mut normalized = Vec::with_capacity(segments.len());
        for segment in segments {
            match segment {
                NotionCommentDraftSegment::Plain(text) if text.is_empty() => {}
                NotionCommentDraftSegment::Plain(text) => {
                    if let Some(NotionCommentDraftSegment::Plain(previous)) = normalized.last_mut()
                    {
                        previous.push_str(&text);
                    } else {
                        normalized.push(NotionCommentDraftSegment::Plain(text));
                    }
                }
                NotionCommentDraftSegment::Mention(user_id) => {
                    normalized.push(NotionCommentDraftSegment::Mention(user_id));
                }
            }
        }
        let has_content = normalized.iter().any(|segment| match segment {
            NotionCommentDraftSegment::Plain(text) => !text.trim().is_empty(),
            NotionCommentDraftSegment::Mention(_) => true,
        });
        if !has_content {
            return Err("Notion comment draft must not be empty".to_string());
        }
        Ok(Self {
            segments: normalized,
        })
    }

    pub(crate) fn segments(&self) -> &[NotionCommentDraftSegment] {
        &self.segments
    }

    pub(crate) fn mentioned_user_ids(&self) -> impl Iterator<Item = &NotionUserId> {
        self.segments.iter().filter_map(|segment| match segment {
            NotionCommentDraftSegment::Mention(user_id) => Some(user_id),
            NotionCommentDraftSegment::Plain(_) => None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NotionCommentWorkspaceUsers {
    pub users: Vec<NotionWorkspaceUser>,
}
