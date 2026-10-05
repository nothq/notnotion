use std::sync::Arc;

mod calendar;
mod code_settings;
mod page_icon;
mod workspace;

pub(crate) use calendar::{CalendarDateAssignmentSource, CalendarItemsQuery};
pub use calendar::{
    CreateCalendarPageRequest, LoadCalendarItemsRequest, LoadCalendarItemsResult,
    SetCalendarPageDateRangeRequest, SetCalendarPageDateRequest,
};
pub(crate) use code_settings::CardPageCodeSettingsBackend;
pub use code_settings::CardPageCodeSettingsCapability;
pub(crate) use page_icon::CustomEmojiPageIconImage;
pub use page_icon::{
    CreateCustomEmojiPageIconRequest, NotionCustomEmoji, NotionCustomEmojiLibrary,
    UploadPageIconRequest,
};
pub use workspace::{NotionWorkspaceApi, NotionWorkspaceOperationFailure, NotionWorkspaceResult};

use crate::model::{
    BoardSnapshot, NotionFilterPageId, NotionLaunchRoute, NotionWorkspaceUser,
    PageShellCalendarEvent, PageShellIcon, PageShellSidebarItem,
};

pub type NotionBootstrapOutcome = Result<NotionWorkspaceBootstrap, NotionBootstrapFailure>;

pub enum NotionCachedBootstrapOutcome {
    Miss,
    Loaded(Box<NotionCachedWorkspaceBootstrap>),
    Failed(NotionCacheFailure),
}

#[derive(Clone, Debug)]
pub enum NotionCacheFailure {
    Invalid { diagnostic: String },
    Unreadable { diagnostic: String },
}

impl std::fmt::Display for NotionCacheFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { diagnostic } | Self::Unreadable { diagnostic } => {
                formatter.write_str(diagnostic)
            }
        }
    }
}

pub trait NotionBootstrapApi: Send + Sync + 'static {
    fn load_cached_workspace(&self) -> NotionCachedBootstrapOutcome;
    fn bootstrap_workspace(&self) -> NotionBootstrapOutcome;
}

#[derive(Debug)]
pub struct NotionBootstrapFailure {
    message: String,
    recovery_disposition: NotionRecoveryDisposition,
}

impl NotionBootstrapFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recovery_disposition: NotionRecoveryDisposition::Preserve,
        }
    }

    pub(crate) const fn recovering(
        message: String,
        recovery_disposition: NotionRecoveryDisposition,
    ) -> Self {
        Self {
            message,
            recovery_disposition,
        }
    }

    pub(crate) fn into_parts(self) -> (String, NotionRecoveryDisposition) {
        (self.message, self.recovery_disposition)
    }
}

impl std::fmt::Display for NotionBootstrapFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NotionBootstrapFailure {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NotionRecoveryDisposition {
    #[default]
    Preserve,
    ReloadCache,
    DiscardStateAndReloadCache,
}

impl NotionRecoveryDisposition {
    pub(crate) const fn reloads_workspace_cache(self) -> bool {
        matches!(self, Self::ReloadCache | Self::DiscardStateAndReloadCache)
    }

    pub(crate) const fn discards_previous_state(self) -> bool {
        matches!(self, Self::DiscardStateAndReloadCache)
    }

    pub(crate) const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::DiscardStateAndReloadCache, _) | (_, Self::DiscardStateAndReloadCache) => {
                Self::DiscardStateAndReloadCache
            }
            (Self::ReloadCache, _) | (_, Self::ReloadCache) => Self::ReloadCache,
            (Self::Preserve, Self::Preserve) => Self::Preserve,
        }
    }
}

#[derive(Clone)]
pub struct NotionCachedWorkspaceBootstrap {
    pub route: NotionLaunchRoute,
    pub workspace: BoardSnapshot,
    pub code_settings: CardPageCodeSettingsCapability,
}

#[derive(Clone)]
pub struct NotionWorkspaceBootstrap {
    pub route: NotionLaunchRoute,
    pub workspace: BoardSnapshot,
    pub workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub code_settings: CardPageCodeSettingsCapability,
    pub(crate) previous_state_disposition: NotionPreviousStateDisposition,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NotionPreviousStateDisposition {
    #[default]
    PreserveCompatible,
    Discard,
}

#[derive(Clone)]
pub struct NotionWorkspaceLoad {
    pub workspace: BoardSnapshot,
    pub workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub code_settings: CardPageCodeSettingsCapability,
}

pub type DatabaseFilterUser = NotionWorkspaceUser;

#[derive(Clone, Debug)]
pub struct LoadDatabaseFilterUsersResult {
    pub users: Vec<DatabaseFilterUser>,
}

#[derive(Clone, Debug)]
pub struct SearchDatabaseFilterRelationPagesRequest {
    collection_id: String,
    query: String,
    selected_page_ids: Vec<NotionFilterPageId>,
}

impl SearchDatabaseFilterRelationPagesRequest {
    pub fn new(
        collection_id: String,
        query: String,
        selected_page_ids: Vec<NotionFilterPageId>,
    ) -> Result<Self, String> {
        if collection_id.trim().is_empty() {
            return Err(
                "Notion relation filter collection ID must not be empty or whitespace".to_string(),
            );
        }
        Ok(Self {
            collection_id,
            query,
            selected_page_ids,
        })
    }

    pub(crate) fn into_parts(self) -> (String, String, Vec<NotionFilterPageId>) {
        (self.collection_id, self.query, self.selected_page_ids)
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseFilterRelationPage {
    pub page_id: NotionFilterPageId,
    pub title: String,
    pub icon: PageShellIcon,
}

#[derive(Clone, Debug)]
pub struct SearchDatabaseFilterRelationPagesResult {
    pub pages: Vec<DatabaseFilterRelationPage>,
}

#[derive(Clone, Debug)]
pub enum NotionRouteSource {
    Explicit(NotionLaunchRoute),
    LastOpened,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarChildrenRequest {
    pub parent_block_id: String,
    pub current_board_url: String,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarChildrenResult {
    pub children: Vec<PageShellSidebarItem>,
    pub sidebar_children_resolved: bool,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarCalendarRequest {
    pub current_board_url: String,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarCalendarResult {
    pub available: bool,
    pub events: Vec<PageShellCalendarEvent>,
}

#[derive(Clone, Debug)]
pub struct MoveCardRequest {
    pub block_id: String,
    pub source_column_title: String,
    pub target_column_title: String,
    pub before_block_id: Option<String>,
    pub after_block_id: Option<String>,
}

impl MoveCardRequest {
    pub fn new(
        block_id: String,
        source_column_title: String,
        target_column_title: String,
        before_block_id: Option<String>,
        after_block_id: Option<String>,
    ) -> Self {
        Self {
            block_id,
            source_column_title,
            target_column_title,
            before_block_id,
            after_block_id,
        }
    }
}
