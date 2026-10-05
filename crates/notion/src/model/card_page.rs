use super::{CardPageDiscussion, DatabasePropertyOption, PageShellIcon, PageTextAnnotation};
use serde::{Deserialize, Serialize};

mod block;
mod color;
mod column_layout;
mod column_ratio;
mod editable;
mod hierarchy;
mod resource;
mod table;

pub use block::{
    CardPageAliasBlock, CardPageBlock, CardPageBlockContent, CardPageBlockLastEdited,
    CardPageLayoutBlock, CardPageStructuralBlock,
};
pub use color::{CardPageBlockColor, CardPageBlockColorValue};
pub(crate) use column_layout::{
    CardPageColumnEffectiveShare, CardPageColumnLayout, CardPageColumnLayoutEntry,
    CardPageColumnLayoutIndex, CardPageColumnLayoutRead, CardPageColumnWeightTotal,
};
pub use column_ratio::CardPageColumnRatio;
pub use editable::{
    CardPageBlockKind, CardPageCodeLanguage, CardPageCodeSettings, CardPageCodeWrap,
    CardPageEditableBlock, CardPageEditableReadOnlyReason, CardPageQuoteSize, CardPageToDoState,
};
pub use resource::{
    CardPageAttachmentDisplaySource, CardPageBlockApiType, CardPageHttpsUrl, CardPageImageBlock,
    CardPageImageDisplayHost, CardPageImageFetchKey, CardPageImageSizeHint, CardPageImageSource,
    CardPageImageWidthTier, CardPageNotionAttachmentPointer, CardPageResourceBlock,
    CardPageUnsupportedLeafBlock,
};
pub(crate) use table::CardPageSimpleTableCellIndex;
pub use table::{
    CardPageSimpleTableBlock, CardPageSimpleTableCell, CardPageSimpleTableCellAddress,
    CardPageSimpleTableCellReadOnlyReason, CardPageSimpleTableCellRoundTrip,
    CardPageSimpleTableColumn, CardPageSimpleTableColumnId, CardPageSimpleTableColumnWidth,
    CardPageSimpleTableRowBlock, CardPageWritableSimpleTableCell,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardSummary {
    pub block_id: String,
    pub title: String,
    pub height: f32,
    #[serde(default = "default_card_summary_has_content")]
    pub has_content: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PageShellIcon>,
}

fn default_card_summary_has_content() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardPage {
    pub block_id: String,
    pub title: String,
    pub status: Option<String>,
    pub properties: Vec<CardPageProperty>,
    pub blocks: Vec<CardPageBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discussions: Vec<CardPageDiscussion>,
    #[serde(default)]
    pub comments_writable: bool,
    #[serde(default)]
    pub format: CardPageFormat,
}

/// The page-wide layout Notion keeps in a page's `format`: small text, full
/// width and the typeface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageFormat {
    #[serde(default)]
    pub small_text: bool,
    #[serde(default)]
    pub full_width: bool,
    #[serde(default)]
    pub font: CardPageFont,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageFont {
    #[default]
    Default,
    Serif,
    Mono,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageProperty {
    #[serde(default)]
    pub property_id: String,
    pub label: String,
    #[serde(default = "default_card_page_property_type")]
    pub property_type: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_options: Vec<DatabasePropertyOption>,
}

fn default_card_page_property_type() -> String {
    "unknown".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageTextAnnotationSpan {
    pub start_utf8: usize,
    pub end_utf8: usize,
    pub annotation: PageTextAnnotation,
}

impl CardPageTextAnnotationSpan {
    pub fn new(
        text: &str,
        start_utf8: usize,
        end_utf8: usize,
        annotation: PageTextAnnotation,
    ) -> Result<Self, String> {
        if start_utf8 >= end_utf8 {
            return Err(format!(
                "page-text annotation span must be non-empty, received {start_utf8}..{end_utf8}"
            ));
        }
        if end_utf8 > text.len() {
            return Err(format!(
                "page-text annotation span {start_utf8}..{end_utf8} exceeds UTF-8 length {}",
                text.len()
            ));
        }
        if !text.is_char_boundary(start_utf8) || !text.is_char_boundary(end_utf8) {
            return Err(format!(
                "page-text annotation span {start_utf8}..{end_utf8} splits a UTF-8 scalar"
            ));
        }
        Ok(Self {
            start_utf8,
            end_utf8,
            annotation,
        })
    }
}
