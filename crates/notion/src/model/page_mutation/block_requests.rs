use std::collections::HashSet;

use super::{NotionPageBlockKind, PageBlockPlacement};
use crate::model::{
    CardPageBlockColor, CardPageCodeLanguage, CardPageCodeWrap, CardPageQuoteSize,
    CardPageToDoState, PageShellIcon,
};

mod reorder;

pub(crate) use reorder::{
    validate_column_safe_page_block_mutations, ColumnSafePageBlockMutationSequence,
};

#[derive(Clone, Debug)]
pub struct CreatePageBlockRequest {
    pub block_id: String,
    pub parent_block_id: String,
    pub kind: NotionPageBlockKind,
    pub text: String,
    pub placement: PageBlockPlacement,
}

#[derive(Clone, Debug)]
pub struct ReplacePageBlockTextRequest {
    pub block_id: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct SplitPageBlockRequest {
    pub block_id: String,
    pub new_block_id: String,
    pub split_offset_utf16: usize,
    pub new_block_kind: NotionPageBlockKind,
}

#[derive(Clone, Debug)]
pub struct MergePageBlocksRequest {
    pub source_block_id: String,
    pub target_block_id: String,
}

#[derive(Clone, Debug)]
pub struct ConvertPageBlockRequest {
    pub block_id: String,
    pub kind: NotionPageBlockKind,
}

#[derive(Clone, Debug)]
pub struct SetPageToDoStateRequest {
    pub block_id: String,
    pub state: CardPageToDoState,
}

#[derive(Clone, Debug)]
pub struct SetPageCodeLanguageRequest {
    block_id: String,
    language: CardPageCodeLanguage,
}

impl SetPageCodeLanguageRequest {
    pub fn new(
        block_id: impl Into<String>,
        language: CardPageCodeLanguage,
    ) -> Result<Self, String> {
        let block_id = block_id.into();
        if block_id.is_empty() {
            return Err("a code-language mutation requires a block".to_string());
        }
        Ok(Self { block_id, language })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub fn language(&self) -> &CardPageCodeLanguage {
        &self.language
    }
}

#[derive(Clone, Debug)]
pub struct SetPageCodeWrapRequest {
    block_id: String,
    wrap: CardPageCodeWrap,
}

impl SetPageCodeWrapRequest {
    pub fn new(block_id: impl Into<String>, wrap: CardPageCodeWrap) -> Result<Self, String> {
        let block_id = block_id.into();
        if block_id.is_empty() {
            return Err("a code-wrap mutation requires a block".to_string());
        }
        Ok(Self { block_id, wrap })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub const fn wrap(&self) -> CardPageCodeWrap {
        self.wrap
    }
}

#[derive(Clone, Debug)]
pub struct SetPageIconRequest {
    block_id: String,
    icon: Option<PageShellIcon>,
}

impl SetPageIconRequest {
    pub fn new(block_id: impl Into<String>, icon: Option<PageShellIcon>) -> Result<Self, String> {
        let block_id = block_id.into();
        if block_id.is_empty() {
            return Err("a page-icon mutation requires a block".to_string());
        }
        if let Some(icon) = &icon {
            if icon.value.trim().is_empty() {
                return Err("a page-icon mutation requires a non-empty icon value".to_string());
            }
            match icon.kind.as_str() {
                "emoji" | "named" => {}
                "external" => icon.validate_external()?,
                "custom" => icon.validate_custom()?,
                _ => {
                    return Err(format!(
                        "a page-icon mutation does not support icon kind {}",
                        icon.kind
                    ));
                }
            }
        }
        Ok(Self { block_id, icon })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub fn icon(&self) -> Option<&PageShellIcon> {
        self.icon.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct SetPageBlockColorRequest {
    block_ids: Vec<String>,
    color: CardPageBlockColor,
}

impl SetPageBlockColorRequest {
    pub fn new(block_ids: Vec<String>, color: CardPageBlockColor) -> Result<Self, String> {
        validate_unique_block_ids(&block_ids, "block-color")?;
        Ok(Self { block_ids, color })
    }

    pub fn block_ids(&self) -> &[String] {
        &self.block_ids
    }

    pub const fn color(&self) -> CardPageBlockColor {
        self.color
    }
}

#[derive(Clone, Debug)]
pub struct SetPageQuoteSizeRequest {
    block_ids: Vec<String>,
    size: CardPageQuoteSize,
}

impl SetPageQuoteSizeRequest {
    pub fn new(block_ids: Vec<String>, size: CardPageQuoteSize) -> Result<Self, String> {
        validate_unique_block_ids(&block_ids, "quote-size")?;
        Ok(Self { block_ids, size })
    }

    pub fn block_ids(&self) -> &[String] {
        &self.block_ids
    }

    pub const fn size(&self) -> CardPageQuoteSize {
        self.size
    }
}

fn validate_unique_block_ids(block_ids: &[String], mutation: &str) -> Result<(), String> {
    if block_ids.is_empty() {
        return Err(format!("a {mutation} mutation requires at least one block"));
    }
    let mut unique = HashSet::with_capacity(block_ids.len());
    if let Some(block_id) = block_ids
        .iter()
        .find(|block_id| !unique.insert(block_id.as_str()))
    {
        return Err(format!(
            "a {mutation} mutation contains duplicate block {block_id}"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ReplacePageBlockTextAndConvertRequest {
    pub block_id: String,
    pub text: String,
    pub kind: NotionPageBlockKind,
}

#[derive(Clone, Debug)]
pub struct ReplacePageBlockWithDividerRequest {
    source_block_id: String,
    divider_block_id: String,
}

impl ReplacePageBlockWithDividerRequest {
    pub fn new(source_block_id: String, divider_block_id: String) -> Result<Self, String> {
        validate_distinct_block_ids(
            &source_block_id,
            &divider_block_id,
            "a divider replacement requires two non-empty block IDs",
            "a divider replacement requires a fresh block ID",
        )?;
        Ok(Self {
            source_block_id,
            divider_block_id,
        })
    }

    pub fn source_block_id(&self) -> &str {
        &self.source_block_id
    }

    pub fn divider_block_id(&self) -> &str {
        &self.divider_block_id
    }
}

#[derive(Clone, Debug)]
pub struct ConvertPageBlockToDividerRequest {
    source_block_id: String,
    continuation_block_id: String,
}

impl ConvertPageBlockToDividerRequest {
    pub fn new(source_block_id: String, continuation_block_id: String) -> Result<Self, String> {
        validate_distinct_block_ids(
            &source_block_id,
            &continuation_block_id,
            "a divider conversion requires two non-empty block IDs",
            "a divider conversion requires a fresh continuation ID",
        )?;
        Ok(Self {
            source_block_id,
            continuation_block_id,
        })
    }

    pub fn source_block_id(&self) -> &str {
        &self.source_block_id
    }

    pub fn continuation_block_id(&self) -> &str {
        &self.continuation_block_id
    }
}

#[derive(Clone, Debug)]
pub struct RestorePageBlockFromDividerRequest {
    divider_block_id: String,
    continuation_block_id: String,
    text: String,
}

impl RestorePageBlockFromDividerRequest {
    pub fn new(
        divider_block_id: String,
        continuation_block_id: String,
        text: String,
    ) -> Result<Self, String> {
        validate_distinct_block_ids(
            &divider_block_id,
            &continuation_block_id,
            "a divider restore requires two non-empty block IDs",
            "a divider restore requires a distinct continuation ID",
        )?;
        Ok(Self {
            divider_block_id,
            continuation_block_id,
            text,
        })
    }

    pub fn divider_block_id(&self) -> &str {
        &self.divider_block_id
    }

    pub fn continuation_block_id(&self) -> &str {
        &self.continuation_block_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

fn validate_distinct_block_ids(
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

#[derive(Clone, Debug)]
pub struct DuplicatePageAliasRequest {
    source_block_id: String,
    new_block_id: String,
}

impl DuplicatePageAliasRequest {
    pub fn new(source_block_id: String, new_block_id: String) -> Result<Self, String> {
        validate_distinct_block_ids(
            &source_block_id,
            &new_block_id,
            "an alias duplicate requires two non-empty block IDs",
            "an alias duplicate requires a fresh block ID",
        )?;
        Ok(Self {
            source_block_id,
            new_block_id,
        })
    }

    pub fn source_block_id(&self) -> &str {
        &self.source_block_id
    }

    pub fn new_block_id(&self) -> &str {
        &self.new_block_id
    }
}

#[derive(Clone, Debug)]
pub struct DeletePageBlockRequest {
    pub block_id: String,
}

#[derive(Clone, Debug)]
pub struct ReorderPageBlockSubtreesRequest {
    pub target_parent_block_id: String,
    pub block_ids: Vec<String>,
    pub placement: PageBlockPlacement,
}

#[derive(Clone, Debug)]
pub enum PageBlockStructuralMutation {
    Reorder(ReorderPageBlockSubtreesRequest),
    Delete(DeletePageBlockRequest),
}
