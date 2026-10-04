use std::time::Instant;

use crate::model::{CardPageSimpleTableCellAddress, PageColumnPair};
use crate::ui::CardPage;
use crate::ui::CardPageBlockKind;
use crate::ui::{days_from_civil, ColorSpec, Tone};
use gpui::{Pixels, Point};

mod page_commands;
mod page_projection;
mod page_rows;
mod page_sections;

pub use page_commands::{PageCommand, PageCommandTarget, PAGE_COMMANDS};
pub(crate) use page_projection::visible_row_spacing;
pub use page_projection::{
    LoadedCardPage, LoadedCardPageData, LoadedCardPageSection, LoadedCardPageVisibleRow,
};
pub(crate) use page_projection::{
    LoadedCardPageDocumentUnit, LoadedCardPageSimpleTableAnnotationRun,
    LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableCellAccess,
    LoadedCardPageSimpleTableCellLocation, LoadedCardPageSimpleTableRowPosition, PageColumnFlow,
    PageDocumentListAllocation, PageDocumentOuterItemId, PageDocumentUnitKey,
    PageDocumentUnitLayoutRevision, PageFlowCalloutPresentationSpec, PageFlowCalloutSegment,
    PageFlowColumns, PageFlowColumnsPresentationSpec, PageFlowDecoratorLayer,
    PageFlowDecoratorPlanRevision, PageFlowExtentEnvelope, PageFlowLaneKey, PageFlowNode,
    PageFlowNodeId, PageFlowNodeKey, PageFlowNodePath, PageFlowPaintedCalloutPrefix,
    PageFlowProjection, PageFlowSection, PageFlowSequence, PageFlowSequenceId,
    PageFlowUnitAdjacency, PageVisibleRowSpacing, PAGE_FLOW_BLOCK_INDENT,
};

#[derive(Clone)]
pub struct Card {
    pub title: String,
    pub block_id: String,
    pub height: f32,
    pub has_content: bool,
    pub icon: Option<crate::ui::PageShellIcon>,
    pub fill_override: Option<u32>,
}

#[derive(Clone, Copy)]
pub struct ColumnStyle {
    pub pill: ColorSpec,
    pub lane: ColorSpec,
    pub default_card_fill: Option<u32>,
    pub tone: Tone,
}

pub struct ColumnState {
    pub title: String,
    pub style: ColumnStyle,
    pub cards: Vec<Card>,
}

#[derive(Clone)]
pub struct DragState {
    pub card: Card,
    pub source_column: usize,
    pub source_index: usize,
    pub pointer_start: Point<Pixels>,
    pub pointer_position: Point<Pixels>,
    pub cursor_offset: Point<Pixels>,
    pub moved: bool,
}

#[derive(Clone, Copy)]
pub struct CardPlacement {
    pub card_index: usize,
    pub top: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
pub struct DropTarget {
    pub column_index: usize,
    pub card_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CardLocation {
    pub column_index: usize,
    pub card_index: usize,
}

impl CardLocation {
    pub const fn new(column_index: usize, card_index: usize) -> Self {
        Self {
            column_index,
            card_index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl CivilDate {
    pub fn ordinal(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HoveredCardAction {
    Edit,
    More,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolbarDialogKind {
    Actions,
    Filter,
    Sort,
    Automations,
    Templates,
    Properties,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropertyPickerIconKind {
    Text,
    Number,
    Checkbox,
    Formula,
    Rollup,
    Time,
    Person,
    Relation,
    Date,
    List,
    Status,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HoveredCard {
    pub column_index: usize,
    pub card_index: usize,
    pub action: Option<HoveredCardAction>,
}

#[derive(Clone)]
pub struct PageComposerState {
    pub page_id: Option<String>,
    pub active: bool,
    pub slash_command_open: bool,
    pub block_kind: CardPageBlockKind,
    pub text: String,
    pub selected_command_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageComposerHandoff {
    pub(crate) page_id: String,
    pub(crate) block_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageSlashMenuState {
    pub block_id: String,
    pub trigger_offset: usize,
    pub query: String,
    pub selected_command_index: usize,
}

/// The open `@` mention menu: which block, where its `@` sits, the text typed
/// after it, and the selected row across all sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMentionMenuState {
    pub block_id: String,
    pub trigger_offset: usize,
    pub query: String,
    pub selected_index: usize,
}

/// The nested option list open inside the date picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageMentionPickerSubmenu {
    DateFormat,
    TimeFormat,
    Remind,
}

/// The date picker popover opened by clicking a date mention chip. The draft
/// is committed to Notion on every change, so it always mirrors the token.
#[derive(Clone, Debug, PartialEq)]
pub struct PageMentionPickerState {
    pub block_id: String,
    /// Byte offset of the `‣` token in the block text.
    pub offset_utf8: usize,
    pub draft: crate::model::PageMentionDate,
    /// First day of the month the calendar shows.
    pub visible_month: chrono::NaiveDate,
    pub submenu: Option<PageMentionPickerSubmenu>,
    /// Window-space bounds of the chip the picker hangs from.
    pub anchor: gpui::Bounds<Pixels>,
    /// Text typed into the date field that has not been applied yet.
    pub field_text: String,
}

/// Workspace members offered by the mention menu's People section.
#[derive(Clone, Debug, Default)]
pub(crate) struct PageMentionPeopleState {
    pub(crate) users: Option<std::sync::Arc<[crate::model::NotionWorkspaceUser]>>,
    pub(crate) loading: bool,
}

/// Pages offered by the mention menu's Link to page section: recents for an
/// empty query, title search results otherwise.
#[derive(Clone, Debug)]
pub(crate) struct PageMentionPagesState {
    /// The query the results were requested for.
    pub(crate) query: String,
    /// The query the current `results` answer, once a load completed.
    pub(crate) loaded_query: Option<String>,
    pub(crate) results: std::sync::Arc<[crate::model::PageShellSearchResult]>,
    pub(crate) generation: u64,
    pub(crate) search_session_id: String,
    pub(crate) flow_number: u32,
    pub(crate) loading: bool,
}

impl Default for PageMentionPagesState {
    fn default() -> Self {
        Self {
            query: String::new(),
            loaded_query: None,
            results: std::sync::Arc::from(Vec::new()),
            generation: 0,
            search_session_id: uuid::Uuid::new_v4().to_string(),
            flow_number: 0,
            loading: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageTextSelection {
    pub anchor_block_id: String,
    pub anchor_offset: usize,
    pub focus_block_id: String,
    pub focus_offset: usize,
    pub pointer_active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PageBlockSelection {
    pub block_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageBlockDragScrollTarget {
    Standalone,
    SelectedPage,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageBlockDragAutoScroll {
    pub target: PageBlockDragScrollTarget,
    pub velocity: f32,
    pub epoch: usize,
}

#[derive(Clone)]
pub(crate) struct PageEditSnapshot {
    pub page: CardPage,
    pub focus: Option<PageEditFocus>,
    pub text_selection: Option<PageTextSelection>,
    pub scope: PageEditScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageEditScope {
    WholePage,
    SimpleTableCell(CardPageSimpleTableCellAddress),
    ColumnRatios(PageColumnPair),
}

#[derive(Clone)]
pub(crate) struct PageTextEditGroup {
    pub target: PageTextEditGroupTarget,
    pub last_edit_at: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageEditFocus {
    Block {
        block_id: String,
        offset: usize,
    },
    SimpleTableCell {
        address: CardPageSimpleTableCellAddress,
        offset: usize,
    },
}

impl PageEditFocus {
    pub(crate) fn block(block_id: impl Into<String>, offset: usize) -> Self {
        Self::Block {
            block_id: block_id.into(),
            offset,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageTextEditGroupTarget {
    Block(String),
    SimpleTableCell(CardPageSimpleTableCellAddress),
}

#[derive(Default)]
pub(crate) struct PageEditHistory {
    pub undo_stack: Vec<PageEditSnapshot>,
    pub redo_stack: Vec<PageEditSnapshot>,
    pub text_group: Option<PageTextEditGroup>,
}

impl Default for PageComposerState {
    fn default() -> Self {
        Self {
            page_id: None,
            active: false,
            slash_command_open: false,
            block_kind: CardPageBlockKind::Text,
            text: String::new(),
            selected_command_index: 0,
        }
    }
}

impl PageComposerState {
    pub fn should_render(&self) -> bool {
        self.active || self.slash_command_open || !self.text.is_empty()
    }
}

#[derive(Clone)]
pub enum CardPeekState {
    Loading { block_id: String, title: String },
    Loaded(LoadedCardPage),
    Error { block_id: String, title: String },
}

impl CardPeekState {
    pub fn block_id(&self) -> &str {
        match self {
            Self::Loading { block_id, .. } => block_id,
            Self::Loaded(page) => &page.data.page.block_id,
            Self::Error { block_id, .. } => block_id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Loading { title, .. } => title,
            Self::Loaded(page) => &page.data.page.title,
            Self::Error { title, .. } => title,
        }
    }
}
