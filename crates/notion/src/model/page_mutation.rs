use super::CardPage;

mod annotations;
mod block_requests;
mod code_requests;
mod column_requests;
mod line_break;
mod mention;
mod selection;
mod table_requests;

use selection::{validate_text_selection_block_ids, validate_text_selection_structural_mutations};

pub use annotations::{
    EditPageBlockTextRequest, PageTextAnnotation, PageTextAnnotationKind, PageTextColor,
    PageTextEditTarget,
};
pub(crate) use block_requests::{
    validate_column_safe_page_block_mutations, ColumnSafePageBlockMutationSequence,
};
pub use block_requests::{
    ConvertPageBlockRequest, ConvertPageBlockToDividerRequest, CreatePageBlockRequest,
    DeletePageBlockRequest, DuplicatePageAliasRequest, MergePageBlocksRequest,
    PageBlockStructuralMutation, ReorderPageBlockSubtreesRequest,
    ReplacePageBlockTextAndConvertRequest, ReplacePageBlockTextRequest,
    ReplacePageBlockWithDividerRequest, RestorePageBlockFromDividerRequest,
    SetPageBlockColorRequest, SetPageCodeLanguageRequest, SetPageCodeWrapRequest,
    SetPageIconRequest, SetPageQuoteSizeRequest, SetPageToDoStateRequest, SplitPageBlockRequest,
};
pub use code_requests::{
    CreatePageCodeBlockRequest, PageCodeBlockSourceText, ReplacePageBlockWithCodeRequest,
    RestorePageBlockFromCodeRequest,
};
pub use column_requests::ResizePageColumnsRequest;
pub(crate) use column_requests::{PageColumnPair, PageColumnWeightPair};
pub use line_break::{
    BreakPageTextSelectionEffect, BreakPageTextSelectionRequest, PageTextCaretTarget,
    PageTextLineBreak,
};
pub use mention::{
    mention_date_label, parse_mention_date_query, relative_day_label, InsertPageMentionRequest,
    PageMention, PageMentionDate, PageMentionDateFormat, PageMentionKind, PageMentionReminder,
    PageMentionTimeFormat, ParsedMentionDate, UpdatePageMentionRequest, PAGE_MENTION_TOKEN,
    PAGE_MENTION_TOKEN_STR,
};
pub use selection::{
    PageTextSelectionAction, PageTextSelectionEdit, PageTextSelectionEndpoint,
    PastePageTextSelectionRequest, ReplacePageTextSelectionEffect, ReplacePageTextSelectionRequest,
};
pub use table_requests::ReplacePageSimpleTableCellRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotionPageBlockKind {
    Text,
    Header,
    SubHeader,
    SubSubHeader,
    Header4,
    BulletedList,
    NumberedList,
    ToDo,
    Toggle,
    Page,
    Callout,
    Quote,
    Code,
    Image,
    Table,
    Divider,
    LinkToPage,
    Alias,
    Other(String),
}

impl NotionPageBlockKind {
    pub(crate) fn from_api_type(block_type: &str) -> Self {
        match block_type {
            "text" => Self::Text,
            "header" => Self::Header,
            "sub_header" => Self::SubHeader,
            "sub_sub_header" => Self::SubSubHeader,
            "header_4" => Self::Header4,
            "bulleted_list" => Self::BulletedList,
            "numbered_list" => Self::NumberedList,
            "to_do" => Self::ToDo,
            "toggle" => Self::Toggle,
            "page" => Self::Page,
            "callout" => Self::Callout,
            "quote" => Self::Quote,
            "code" => Self::Code,
            "image" => Self::Image,
            "table" => Self::Table,
            "divider" => Self::Divider,
            "link_to_page" => Self::LinkToPage,
            "alias" => Self::Alias,
            other => Self::Other(other.to_string()),
        }
    }

    pub(crate) fn api_type(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::Header => "header",
            Self::SubHeader => "sub_header",
            Self::SubSubHeader => "sub_sub_header",
            Self::Header4 => "header_4",
            Self::BulletedList => "bulleted_list",
            Self::NumberedList => "numbered_list",
            Self::ToDo => "to_do",
            Self::Toggle => "toggle",
            Self::Page => "page",
            Self::Callout => "callout",
            Self::Quote => "quote",
            Self::Code => "code",
            Self::Image => "image",
            Self::Table => "table",
            Self::Divider => "divider",
            Self::LinkToPage => "link_to_page",
            Self::Alias => "alias",
            Self::Other(block_type) => block_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageBlockPlacement {
    Append,
    Before(String),
    After(String),
}

#[derive(Clone, Debug)]
pub enum PageMutation {
    CreateBlock(CreatePageBlockRequest),
    CreateCodeBlock(CreatePageCodeBlockRequest),
    DuplicateAlias(DuplicatePageAliasRequest),
    ReplaceBlockWithCode(ReplacePageBlockWithCodeRequest),
    RestoreBlockFromCode(RestorePageBlockFromCodeRequest),
    ReplaceBlockWithDivider(ReplacePageBlockWithDividerRequest),
    ConvertBlockToDivider(ConvertPageBlockToDividerRequest),
    RestoreBlockFromDivider(RestorePageBlockFromDividerRequest),
    ReplaceBlockText(ReplacePageBlockTextRequest),
    ReplaceSimpleTableCell(ReplacePageSimpleTableCellRequest),
    ReplaceBlockTextAndConvert(ReplacePageBlockTextAndConvertRequest),
    ReplaceTextSelection(ReplacePageTextSelectionRequest),
    BreakTextSelection(BreakPageTextSelectionRequest),
    PasteTextSelection(PastePageTextSelectionRequest),
    SplitBlock(SplitPageBlockRequest),
    MergeBlocks(MergePageBlocksRequest),
    ConvertBlock(ConvertPageBlockRequest),
    SetToDoState(SetPageToDoStateRequest),
    SetCodeLanguage(SetPageCodeLanguageRequest),
    SetCodeWrap(SetPageCodeWrapRequest),
    SetPageIcon(SetPageIconRequest),
    SetBlockColor(SetPageBlockColorRequest),
    SetQuoteSize(SetPageQuoteSizeRequest),
    ResizeColumns(ResizePageColumnsRequest),
    DeleteBlock(DeletePageBlockRequest),
    ReorderSubtrees(ReorderPageBlockSubtreesRequest),
    InsertMention(InsertPageMentionRequest),
    UpdateMention(UpdatePageMentionRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageMutationEffect {
    NoAdditionalContext,
    BlockCreated { block_id: String },
    BlockSplit(SplitPageBlockEffect),
    BlocksMerged(MergePageBlocksEffect),
    TextSelectionReplaced(ReplacePageTextSelectionEffect),
    TextSelectionBroken(BreakPageTextSelectionEffect),
}

impl PageMutationEffect {
    pub fn created_block_id(&self) -> Option<&str> {
        match self {
            Self::BlockCreated { block_id } => Some(block_id),
            Self::BlockSplit(effect) => Some(&effect.new_block_id),
            Self::TextSelectionBroken(effect) => effect.created_block_id(),
            Self::NoAdditionalContext | Self::BlocksMerged(_) | Self::TextSelectionReplaced(_) => {
                None
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPageBlockEffect {
    pub source_block_id: String,
    pub new_block_id: String,
    pub parent_block_id: String,
    pub moved_child_block_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergePageBlocksEffect {
    pub source_block_id: String,
    pub target_block_id: String,
    pub source_parent_block_id: String,
    pub source_children: MergePageBlockChildrenEffect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergePageBlockChildrenEffect {
    LiftedToSourceParent { child_block_ids: Vec<String> },
    RemainUnderRemovedSource { child_block_ids: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct PageMutationRequest {
    pub page_block_id: String,
    pub mutation: PageMutation,
}

#[derive(Clone, Debug)]
pub struct PageMutationResult {
    pub page: CardPage,
    pub effect: PageMutationEffect,
}
