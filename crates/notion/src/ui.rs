mod app_state;
mod block_images;
mod board_workspace;
mod constants;
mod external_icons;
mod icons;
#[cfg(any(test, feature = "test-support"))]
mod mention_test_support;
mod named_icons;
mod page;
#[cfg(any(test, feature = "test-support"))]
mod regression_test_support;
mod search;
mod surface;
mod svg_icons;
#[cfg(test)]
mod tests;
mod theme;
mod timeline_helpers;
mod top_bar_helpers;
mod view_actions;

#[cfg(test)]
use crate::model::BoardColumn;
use crate::model::{
    BoardDateValue, BoardItem, BoardSnapshot, CardPage, CardPageBlock, CardPageBlockColor,
    CardPageBlockColorValue, CardPageBlockContent, CardPageBlockKind, CardPageEditableBlock,
    CardPageImageBlock, CardPageImageFetchKey, CardPageImageSource, CardPageLayoutBlock,
    CardPageProperty, CardPageQuoteSize, CardPageSimpleTableBlock, CardPageSimpleTableColumn,
    CardPageStructuralBlock, CardPageToDoState, CardPageUnsupportedLeafBlock, MoveCardRequest,
    NotionBootstrapApi, NotionCachedWorkspaceBootstrap, NotionWorkspaceApi,
    NotionWorkspaceBootstrap, NotionWorkspaceLoad, PageShellIcon, PageShellLink,
    PageShellSidebarItem, PageShellSidebarSection, PageShellSnapshot, TableViewColumn,
    TimelineViewConfig, ViewTab, ViewTabKind,
};
use gpui::{AnyElement, IntoElement};

#[cfg(feature = "test-support")]
pub use mention_test_support::{
    drive_mention_escape, drive_today_mention_commit, mention_token_text, NotionDateMentionOutcome,
    NotionMentionEscapeOutcome,
};
#[cfg(feature = "test-support")]
pub use regression_test_support::{
    drive_selected_page_overlay_wheel_scroll, drive_standalone_document_wheel_scroll,
    NotionPageScrollOffset, NotionPageScrollOutcome, NotionPageScrollSurface,
};
pub(crate) use surface::{
    DraggedNotionSidebarResize, NotionAiMode, NotionSearchPreviewState, NotionSearchRequestToken,
    NotionSearchResultsState, NotionSidebarNodeKey, NotionSidebarSectionKey, NotionSidebarTab,
    NotionStartup, NotionSurfaceResources, NotionSurfaceServices, PageBlockInputPropsKey,
    NOTION_PAGE_MAIN_MIN_WIDTH, NOTION_PAGE_SIDEBAR_MIN_WIDTH,
};
#[cfg(any(test, feature = "test-support"))]
pub use surface::{NotionFixtureInput, NotionFixtureSource};
pub use surface::{NotionPendingNavigation, SurfaceRoot, SurfaceState};
#[cfg(test)]
pub(crate) use surface::{NotionSearchRecentsCommit, NotionSearchState};

use app_model::{AppearanceMode, Viewport};
pub(crate) use app_state::visible_row_spacing;
pub use app_state::{
    Card, CardLocation, CardPeekState, CardPlacement, CivilDate, ColumnState, ColumnStyle,
    DragState, DropTarget, HoveredCard, HoveredCardAction, LoadedCardPage, LoadedCardPageData,
    LoadedCardPageVisibleRow, PageCommand, PageCommandTarget, PageComposerState,
    PageMentionMenuState, PageMentionPickerState, PageMentionPickerSubmenu, PageSlashMenuState,
    PropertyPickerIconKind, ToolbarDialogKind, PAGE_COMMANDS,
};
pub(crate) use app_state::{
    LoadedCardPageDocumentUnit, LoadedCardPageSimpleTableAnnotationRun,
    LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableCellAccess,
    LoadedCardPageSimpleTableRowPosition, PageBlockDragAutoScroll, PageBlockDragScrollTarget,
    PageBlockSelection, PageColumnFlow, PageComposerHandoff, PageDocumentListAllocation,
    PageDocumentOuterItemId, PageDocumentUnitKey, PageDocumentUnitLayoutRevision, PageEditFocus,
    PageEditHistory, PageEditScope, PageEditSnapshot, PageFlowCalloutPresentationSpec,
    PageFlowCalloutSegment, PageFlowColumns, PageFlowColumnsPresentationSpec,
    PageFlowDecoratorLayer, PageFlowDecoratorPlanRevision, PageFlowExtentEnvelope, PageFlowLaneKey,
    PageFlowNode, PageFlowNodeId, PageFlowNodeKey, PageFlowNodePath, PageFlowPaintedCalloutPrefix,
    PageFlowProjection, PageFlowSection, PageFlowSequence, PageFlowSequenceId,
    PageFlowUnitAdjacency, PageMentionPagesState, PageMentionPeopleState, PageTextEditGroup,
    PageTextEditGroupTarget, PageTextSelection, PageVisibleRowSpacing, PAGE_FLOW_BLOCK_INDENT,
};
pub(crate) use block_images::*;
pub use constants::ColorSpec;
pub use theme::Tone;

pub(crate) use std::{collections::HashSet, sync::Arc};

pub(crate) use gpui::prelude::FluentBuilder;
#[cfg(test)]
pub(crate) use gpui::HeadlessAppContext;
pub(crate) use gpui::{
    div, img, point, px, relative, rgb, BoxShadow, ClipboardItem, Context, Div, Element,
    FontWeight, Image, ImageFormat, InteractiveElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ObjectFit, ParentElement, Pixels, Point, ScrollDelta,
    ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Styled, StyledImage, Window,
};
#[cfg(test)]
pub(crate) use gpui_platform::{current_headless_renderer, current_platform};

pub(crate) use constants::*;
pub(crate) use external_icons::*;
pub(crate) use icons::*;
pub(crate) use named_icons::*;
pub(crate) use page::block_color::*;
pub(crate) use page::command_support::*;
pub(crate) use page::property_format::*;
pub(crate) use svg_icons::*;
pub(crate) use theme::*;
pub(crate) use timeline_helpers::*;
pub(crate) use top_bar_helpers::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionPageRow {
    pub id: String,
    pub title: String,
}

pub fn notion_board_title(board: &BoardSnapshot) -> String {
    if board.page_title.trim().is_empty() {
        board.database_title.clone()
    } else {
        board.page_title.clone()
    }
}

pub fn notion_page_row(page: &CardPage) -> NotionPageRow {
    NotionPageRow {
        id: page.block_id.clone(),
        title: page.title.clone(),
    }
}

pub fn render_notion_empty_state(message: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child("Notion")
        .child(message.into())
        .into_any_element()
}
