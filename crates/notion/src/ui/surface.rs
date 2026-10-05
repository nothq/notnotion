use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use app_model::SurfaceFrame;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, Entity, InteractiveElement, IntoElement, ParentElement,
    Pixels, Render, SharedString, StatefulInteractiveElement, Styled, WeakEntity,
};
mod board_view_state;
mod chrome_state;
mod comment_state;
mod database_view_state;
mod date_view_state;
mod filter_state;
mod inline_database;
mod inline_database_view_state;
mod page_documents;
mod page_editor_state;
mod page_mutation_coordinator;
mod presentation_state;
mod render;
mod search_state;
mod services;
mod share_state;
mod sidebar_state;
mod startup;
mod surface_root;
mod surface_state;

pub(crate) use board_view_state::{BoardViewState, InlineToolbarDialogAnchors};
pub(crate) use chrome_state::{
    AiAutofillDialogState, DateUndatedDialogSource, DateUndatedDialogState,
    InlineToolbarDialogState, NotionChromeState, NotionNavigationDirection,
    NotionNavigationHistoryUpdate, StatusPropertyPickerSource, StatusPropertyPickerState,
    StatusPropertyPickerTarget,
};
pub(crate) use comment_state::{
    NotionCommentMentionRow, NotionCommentRow, NotionCommentSegmentRow, NotionCommentsListRow,
    NotionCommentsPanelState, NotionCommentsState,
};
pub(crate) use database_view_state::{
    database_view_control_property_rows, database_view_rows, DatabaseViewControlPropertyRow,
    DatabaseViewControlsState, DatabaseViewPropertyRow, DatabaseViewRow,
};
pub(crate) use date_view_state::{
    CalendarRangeResizePreview, DateUndatedItemsState, DateUndatedRow, NotionDateViewState,
};
pub(crate) use filter_state::{
    DatabaseFilterDialogStage, DatabaseFilterDraft, DatabaseFilterPropertyPickerTarget,
    DatabaseFilterUiState, DatabaseTextFilterOperator, CURRENT_USER_FILTER_VALUE,
    STATUS_GROUP_FILTER_VALUE_PREFIX, STATUS_OPTION_FILTER_VALUE_PREFIX,
};
pub(crate) use inline_database::InlineDatabaseView;
pub(crate) use inline_database_view_state::{
    InlineDatabaseViewMenuOwner, InlineDatabaseViewMenuSelection, InlineDatabaseViewMenuState,
    InlineDatabaseViewTabLayoutSource, InlineDatabaseViewTabsLayout,
};
pub(crate) use page_documents::{
    PageDocumentLayoutInvalidation, PageDocumentLayoutTarget, PageDocuments, PageHydrationEffect,
    PageHydrationEvent, PageHydrationReadiness,
};
pub(crate) use page_editor_state::{
    allocate_page_flow_lane_widths, PageBlockInputPropsKey, PageColumnResizeGeometry,
    PageColumnResizeOrigin, PageColumnResizeSession, PageColumnResizeSource,
    PageDocumentDragRuntime, PageDocumentFlowRuntime, PageDocumentFocusRegistryEntry,
    PageEditorState, PageFlowAnchorTarget, PageFlowCapturedSemanticAnchor, PageFlowCommittedFocus,
    PageFlowDragAuthority, PageFlowDropAnchor, PageFlowExactLayoutWidth, PageFlowLaneObservation,
    PageFlowLayoutCommit, PageFlowLayoutCommitSchedule, PageFlowLayoutFrameToken,
    PageFlowLayoutWidth, PageFlowLogicalViewportBasis, PageFlowNodeObservation,
    PageFlowObservationToken, PageFlowOuterExtentAuthority, PageFlowOuterItemExtent,
    PageFlowOuterItemMatch, PageFlowOuterItemRemap, PageFlowOuterLocalY, PageFlowPinSet,
    PageFlowPinTarget, PageFlowRenderSpan, PageFlowSectionMeasurement,
    PageFlowSemanticAnchorBaseline, PageFlowSemanticAnchorIntent,
    PageFlowSemanticAnchorTransaction, PageFlowSequenceWidth, PageFlowSurfaceKey,
    PageFlowUnitObservation, PageFlowViewport, PageFlowViewportRelativeY, PageInputResources,
    PageSimpleTableCellCompositionBaseline, PageSimpleTableCellFocusMode,
    PageSimpleTableCellForcedAnnotations, PageSimpleTableCellGeneration,
    PageSimpleTableCellObservedReplacement, PageSimpleTableCellReplacementOrigin,
    PageSimpleTableCellReplacementPlan, PageSimpleTableRuntime,
};
pub(crate) use page_mutation_coordinator::{
    CompletedPageMutationWrite, FailedPageMutationWrite, PageMutationAuthoritySnapshot,
    PageMutationCoordinator, PageMutationDispatch, PageMutationRecoveryCompletion,
    PageMutationRecoveryReplay, PageMutationRecoveryToken, PageMutationRunToken,
};
pub(crate) use presentation_state::{InlineDatabaseViews, SurfacePresentationState};
#[cfg(test)]
pub(crate) use search_state::NotionSearchRecentsCommit;
pub(crate) use search_state::{
    DatabaseSearchState, NotionSearchListRow, NotionSearchPreviewState, NotionSearchRequestToken,
    NotionSearchResultsState, NotionSearchState,
};
pub(crate) use services::{NotionSurfaceResources, NotionSurfaceServices};
pub(crate) use share_state::{
    NotionShareDialogPublicLink, NotionShareDialogRow, NotionShareDialogState,
    NotionShareRolePickerTarget,
};
pub(crate) use sidebar_state::{
    NotionSidebarCalendarState, NotionSidebarChatsState, NotionSidebarEffect,
    NotionSidebarInboxState, NotionSidebarNodeKey, NotionSidebarRow, NotionSidebarRowAction,
    NotionSidebarRowLayout, NotionSidebarSectionKey, NotionSidebarSectionRow, NotionSidebarState,
    NotionSidebarTab, NotionSidebarUpdate, NotionSidebarVisibleRow, SidebarResourceContext,
    SidebarResourceEffect, SidebarResourceEvent, SidebarWorkspacePreparation,
    SidebarWorkspaceReadiness,
};

use crate::model::DatabaseProperty;
use crate::ui::board_workspace::{
    PageBlockContextMenuState, PageBlockDragLayouts, PageBlockDragTarget, PageBlockFocusRequest,
    PageCodeSyntaxCache, PageForcedTextAnnotations, PageLinkIconController,
    PagePendingCrossBlockComposition, PagePendingRichTextComposition, PagePendingRichTextTyping,
    PageRichTextDialog, SupportedPageBlockAction,
};
#[cfg(any(test, feature = "test-support"))]
use crate::ui::NotionWorkspaceApi;
use crate::ui::{
    column_style, div, px, rgb, AppearanceMode, Arc, BoardSnapshot, Card, CardPage, ColumnState,
    Context, HashSet, IconSet, Image, ImageFormat, LoadedCardPage, MouseButton, NotionBootstrapApi,
    NotionWorkspaceBootstrap, PageBlockDragAutoScroll, PageBlockSelection, PageComposerHandoff,
    PageComposerState, PageEditHistory, PageSlashMenuState, PageTextSelection, ScrollHandle, Theme,
    ToolbarDialogKind, Viewport, Window,
};
use gpui_components::text_input::TextInput;

pub(crate) const NOTION_PAGE_SIDEBAR_DEFAULT_WIDTH: f32 = 270.0;
pub(crate) const NOTION_PAGE_SIDEBAR_MIN_WIDTH: f32 = 220.0;
pub(crate) const NOTION_PAGE_MAIN_MIN_WIDTH: f32 = 480.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionPendingNavigation {
    pub board_url: String,
    pub request_id: u64,
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone)]
pub struct NotionFixtureInput {
    pub board: BoardSnapshot,
    pub source: NotionFixtureSource,
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone)]
pub enum NotionFixtureSource {
    WorkspaceApi(crate::ui::Arc<dyn NotionWorkspaceApi>),
    SnapshotPages(HashMap<String, CardPage>),
}

#[derive(Clone)]
pub(crate) enum NotionStartup {
    #[cfg(any(test, feature = "test-support"))]
    Fixture(NotionFixtureInput),
    Loading {
        bootstrap_api: crate::ui::Arc<dyn NotionBootstrapApi>,
        request_started: bool,
        cache_request_started: bool,
        cached: Option<crate::ui::NotionCachedWorkspaceBootstrap>,
        retry_attempt: u32,
    },
    Ready(NotionWorkspaceBootstrap),
    Error {
        bootstrap_api: crate::ui::Arc<dyn NotionBootstrapApi>,
        cached: Option<crate::ui::NotionCachedWorkspaceBootstrap>,
        retry_attempt: u32,
        cache_retry: NotionStartupCacheRetry,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum NotionStartupCacheRetry {
    Preserve,
    Reload,
}

impl NotionStartupCacheRetry {
    const fn request_already_started(self) -> bool {
        matches!(self, Self::Preserve)
    }
}

pub struct SurfaceRoot {
    startup: NotionStartup,
    notion_resources: NotionSurfaceResources,
    viewport: Viewport,
    preview_width: f32,
    active: bool,
    pending_route: Option<crate::model::NotionLaunchRoute>,
    surface: Option<gpui::Entity<SurfaceState>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NotionAiMode {
    #[default]
    Sidebar,
    Floating,
    FullScreen,
}

#[derive(Clone, Copy)]
pub(crate) struct DraggedNotionSidebarResize;

impl Render for DraggedNotionSidebarResize {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

#[derive(Clone, Copy)]
pub(crate) enum NotionCachedSurfaceRegion {
    Sidebar,
    Main,
}

pub(crate) struct NotionCachedSurfaceView {
    surface: WeakEntity<SurfaceState>,
    region: NotionCachedSurfaceRegion,
}

impl NotionCachedSurfaceView {
    pub(crate) fn new(
        surface: Entity<SurfaceState>,
        region: NotionCachedSurfaceRegion,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&surface, |_, surface, cx| {
            let surface = surface.read(cx);
            if surface.page_documents.selected_page.is_none()
                && !surface.notion_chrome.notion_search_open
            {
                cx.notify();
            }
        })
        .detach();
        Self {
            surface: surface.downgrade(),
            region,
        }
    }
}

pub struct SurfaceState {
    pub(crate) surface_active: bool,
    pub(crate) appearance_mode: AppearanceMode,
    pub(crate) theme: Theme,
    pub(crate) viewport: Viewport,
    pub(crate) preview_width: f32,
    pub(crate) notion_startup: NotionStartup,
    pub(crate) notion_startup_generation: u64,
    pub(crate) board: BoardSnapshot,
    pub(crate) database_view_rows: Arc<[DatabaseViewRow]>,
    pub(crate) database_view_controls: DatabaseViewControlsState,
    pub(crate) snapshot_pages: Option<HashMap<String, CardPage>>,
    pub(crate) page_documents: PageDocuments,
    pub(crate) columns: Vec<ColumnState>,
    pub(crate) icons: Arc<IconSet>,
    pub(crate) notion_resources: NotionSurfaceResources,
    pub(crate) page_code_settings: crate::model::CardPageCodeSettingsCapability,
    pub(crate) presence_images: Vec<Option<Arc<Image>>>,
    pub(crate) favorited: bool,
    pub(crate) board_view: BoardViewState,
    pub(crate) date_view: NotionDateViewState,
    pub(crate) notion_sidebar: NotionSidebarState,
    pub(crate) database_search: DatabaseSearchState,
    pub(crate) database_filter: DatabaseFilterUiState,
    pub(crate) notion_search: NotionSearchState,
    pub(crate) page_mutations: PageMutationCoordinator,
    pub(crate) page_editor: PageEditorState,
    pub(crate) notion_chrome: NotionChromeState,
    pub(crate) comments: NotionCommentsState,
    pub(crate) presentation: SurfacePresentationState,
}

impl SurfaceState {
    pub(crate) fn print_notion_error(&self, error: impl std::fmt::Display) {
        eprintln!("notnotion: {error}");
    }
}
