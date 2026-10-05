use super::PageBlockPlacement;
use crate::model::CardPageCodeSettings;

#[derive(Clone, Debug)]
pub struct CreatePageCodeBlockRequest {
    block_id: String,
    parent_block_id: String,
    placement: PageBlockPlacement,
    settings: CardPageCodeSettings,
}

impl CreatePageCodeBlockRequest {
    pub fn new(
        block_id: String,
        parent_block_id: String,
        placement: PageBlockPlacement,
        settings: CardPageCodeSettings,
    ) -> Result<Self, String> {
        validate_distinct_ids(
            &block_id,
            &parent_block_id,
            "a Code creation requires non-empty block and parent IDs",
            "a Code block cannot parent itself",
        )?;
        Ok(Self {
            block_id,
            parent_block_id,
            placement,
            settings,
        })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub fn parent_block_id(&self) -> &str {
        &self.parent_block_id
    }

    pub const fn placement(&self) -> &PageBlockPlacement {
        &self.placement
    }

    pub const fn settings(&self) -> &CardPageCodeSettings {
        &self.settings
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageCodeBlockSourceText(PageCodeBlockSourceTextKind);

#[derive(Clone, Debug, PartialEq, Eq)]
enum PageCodeBlockSourceTextKind {
    SlashToken(String),
    TripleBacktick { persisted_text: String },
}

impl PageCodeBlockSourceText {
    pub fn slash_token(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if !value.starts_with('/') || value.chars().any(char::is_whitespace) {
            return Err("a Code slash replacement requires one complete slash token".to_string());
        }
        Ok(Self(PageCodeBlockSourceTextKind::SlashToken(value)))
    }

    pub fn triple_backtick(persisted_text: impl Into<String>) -> Result<Self, String> {
        let persisted_text = persisted_text.into();
        if !matches!(persisted_text.as_str(), "" | "`" | "``" | "```") {
            return Err(
                "a Code backtick replacement requires a verified backtick-only source".to_string(),
            );
        }
        Ok(Self(PageCodeBlockSourceTextKind::TripleBacktick {
            persisted_text,
        }))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value == "```" {
            return Self::triple_backtick(value);
        }
        Self::slash_token(value)
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            PageCodeBlockSourceTextKind::SlashToken(value) => value,
            PageCodeBlockSourceTextKind::TripleBacktick { .. } => "```",
        }
    }

    pub(crate) fn expected_persisted_text(&self) -> &str {
        match &self.0 {
            PageCodeBlockSourceTextKind::SlashToken(value) => value,
            PageCodeBlockSourceTextKind::TripleBacktick { persisted_text } => persisted_text,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReplacePageBlockWithCodeRequest {
    source_block_id: String,
    code_block_id: String,
    source_text: PageCodeBlockSourceText,
    settings: CardPageCodeSettings,
}

impl ReplacePageBlockWithCodeRequest {
    pub fn new(
        source_block_id: String,
        code_block_id: String,
        source_text: PageCodeBlockSourceText,
        settings: CardPageCodeSettings,
    ) -> Result<Self, String> {
        validate_distinct_ids(
            &source_block_id,
            &code_block_id,
            "a Code replacement requires two non-empty block IDs",
            "a Code replacement requires a fresh block ID",
        )?;
        Ok(Self {
            source_block_id,
            code_block_id,
            source_text,
            settings,
        })
    }

    pub fn source_block_id(&self) -> &str {
        &self.source_block_id
    }

    pub fn code_block_id(&self) -> &str {
        &self.code_block_id
    }

    pub const fn source_text(&self) -> &PageCodeBlockSourceText {
        &self.source_text
    }

    pub const fn settings(&self) -> &CardPageCodeSettings {
        &self.settings
    }
}

#[derive(Clone, Debug)]
pub struct RestorePageBlockFromCodeRequest {
    source_block_id: String,
    code_block_id: String,
    source_text: PageCodeBlockSourceText,
}

impl RestorePageBlockFromCodeRequest {
    pub fn new(
        source_block_id: String,
        code_block_id: String,
        source_text: PageCodeBlockSourceText,
    ) -> Result<Self, String> {
        validate_distinct_ids(
            &source_block_id,
            &code_block_id,
            "a Code restore requires two non-empty block IDs",
            "a Code restore requires distinct source and Code IDs",
        )?;
        Ok(Self {
            source_block_id,
            code_block_id,
            source_text,
        })
    }

    pub fn source_block_id(&self) -> &str {
        &self.source_block_id
    }

    pub fn code_block_id(&self) -> &str {
        &self.code_block_id
    }

    pub const fn source_text(&self) -> &PageCodeBlockSourceText {
        &self.source_text
    }
}

fn validate_distinct_ids(
    first: &str,
    second: &str,
    missing_message: &str,
    duplicate_message: &str,
) -> Result<(), String> {
    if first.is_empty() || second.is_empty() {
        return Err(missing_message.to_string());
    }
    if first == second {
        return Err(duplicate_message.to_string());
    }
    Ok(())
}
