use super::{PageMention, PAGE_MENTION_TOKEN, PAGE_MENTION_TOKEN_STR};

/// Replace a text range with one mention token.
///
/// The range covers the typed trigger text (`@today`); `trailing_space` adds
/// the plain space Notion inserts after a committed mention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertPageMentionRequest {
    block_id: String,
    start_utf8: usize,
    end_utf8: usize,
    mention: PageMention,
    trailing_space: bool,
}

impl InsertPageMentionRequest {
    pub fn new(
        block_id: impl Into<String>,
        start_utf8: usize,
        end_utf8: usize,
        mention: PageMention,
        trailing_space: bool,
    ) -> Result<Self, String> {
        let block_id = block_id.into();
        if block_id.trim().is_empty() {
            return Err("a mention insertion requires a block id".to_string());
        }
        if start_utf8 > end_utf8 {
            return Err(format!(
                "a mention insertion range must not be reversed, received {start_utf8}..{end_utf8}"
            ));
        }
        Ok(Self {
            block_id,
            start_utf8,
            end_utf8,
            mention,
            trailing_space,
        })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub const fn start_utf8(&self) -> usize {
        self.start_utf8
    }

    pub const fn end_utf8(&self) -> usize {
        self.end_utf8
    }

    pub const fn mention(&self) -> &PageMention {
        &self.mention
    }

    pub const fn trailing_space(&self) -> bool {
        self.trailing_space
    }

    /// Move the replaced range, used when a queued write is rebased onto a
    /// newer authoritative page.
    pub(crate) fn set_range(&mut self, start_utf8: usize, end_utf8: usize) -> Result<(), String> {
        if start_utf8 > end_utf8 {
            return Err(format!(
                "a mention insertion range must not be reversed, received {start_utf8}..{end_utf8}"
            ));
        }
        self.start_utf8 = start_utf8;
        self.end_utf8 = end_utf8;
        Ok(())
    }

    /// The text that replaces the range in the editable block.
    pub fn inserted_text(&self) -> String {
        if self.trailing_space {
            format!("{PAGE_MENTION_TOKEN} ")
        } else {
            PAGE_MENTION_TOKEN_STR.to_string()
        }
    }
}

/// Replace the annotation of the mention token that starts at `offset_utf8`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePageMentionRequest {
    block_id: String,
    offset_utf8: usize,
    mention: PageMention,
}

impl UpdatePageMentionRequest {
    pub fn new(
        block_id: impl Into<String>,
        offset_utf8: usize,
        mention: PageMention,
    ) -> Result<Self, String> {
        let block_id = block_id.into();
        if block_id.trim().is_empty() {
            return Err("a mention update requires a block id".to_string());
        }
        Ok(Self {
            block_id,
            offset_utf8,
            mention,
        })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub const fn offset_utf8(&self) -> usize {
        self.offset_utf8
    }

    pub const fn mention(&self) -> &PageMention {
        &self.mention
    }

    /// Move the token offset, used when a queued write is rebased onto a
    /// newer authoritative page.
    pub(crate) fn set_offset(&mut self, offset_utf8: usize) {
        self.offset_utf8 = offset_utf8;
    }
}
