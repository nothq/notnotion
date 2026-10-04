use super::super::{
    database_view_control_property_rows, database_view_rows, DatabaseViewControlPropertyRow,
    DatabaseViewRow, PageMutationCoordinator,
};
#[cfg(any(test, feature = "test-support"))]
use super::NotionFixtureSource;
use super::{
    board_presence_images, civil_date_month_start, empty_board_snapshot, local_today_civil_date,
    notion_sidebar_should_be_visible, AppearanceMode, BoardSnapshot, BoardViewState, CardPage,
    DatabaseFilterUiState, DatabaseSearchState, HashMap, IconSet, LoadedCardPage,
    NotionChromeState, NotionDateViewState, NotionSidebarState, NotionStartup, PageEditorState,
    ScrollHandle, SurfacePresentationState, SurfaceState, Theme, Viewport,
};
use crate::ui::surface::NotionCommentsState;
use crate::ui::{ColumnState, Image, NotionSurfaceResources};

struct StartupProjection {
    board: BoardSnapshot,
    database_view_rows: crate::ui::Arc<[DatabaseViewRow]>,
    database_view_control_property_rows: crate::ui::Arc<[DatabaseViewControlPropertyRow]>,
    snapshot_pages: Option<HashMap<String, CardPage>>,
    theme: Theme,
    favorited: bool,
    presence_images: Vec<Option<crate::ui::Arc<Image>>>,
    columns: Vec<ColumnState>,
    standalone_page_content: Option<LoadedCardPage>,
    hydrate_standalone_page: bool,
    timeline_scroll_handle: ScrollHandle,
    date_view: NotionDateViewState,
    database_search: DatabaseSearchState,
    database_filter: DatabaseFilterUiState,
    notion_chrome: NotionChromeState,
    notion_sidebar: NotionSidebarState,
    page_code_settings: crate::model::CardPageCodeSettingsCapability,
}

struct StartupSurfaceContext {
    startup: NotionStartup,
    appearance_mode: AppearanceMode,
    viewport: Viewport,
    surface_active: bool,
    notion_resources: NotionSurfaceResources,
}

impl StartupProjection {
    fn new(startup: &NotionStartup, appearance_mode: AppearanceMode, viewport: Viewport) -> Self {
        let (board, snapshot_pages) = startup_board(startup);
        let columns =
            super::initialization::build_columns(&board, snapshot_pages.as_ref(), appearance_mode);
        let standalone_page_content = board.page_content.clone().map(LoadedCardPage::new);
        let hydrate_standalone_page =
            standalone_page_content.is_some() && matches!(startup, NotionStartup::Ready(_));
        let calendar_visible_month = civil_date_month_start(local_today_civil_date());
        let date_view = NotionDateViewState::new(board.items.clone(), calendar_visible_month);
        let mut database_search = DatabaseSearchState::default();
        database_search.rebuild_index(&board, &columns);
        let notion_chrome = NotionChromeState::new(notion_sidebar_should_be_visible(&board));
        let notion_sidebar = NotionSidebarState::new(&board);
        Self {
            theme: Theme::for_appearance_mode(appearance_mode),
            favorited: board.is_favorited,
            presence_images: board_presence_images(&board),
            timeline_scroll_handle: BoardViewState::new_timeline_scroll_handle(&board, viewport),
            database_filter: DatabaseFilterUiState::for_board(&board),
            database_view_rows: database_view_rows(&board),
            database_view_control_property_rows: database_view_control_property_rows(&board),
            board,
            snapshot_pages,
            columns,
            standalone_page_content,
            hydrate_standalone_page,
            date_view,
            database_search,
            notion_chrome,
            notion_sidebar,
            page_code_settings: startup_code_settings(startup),
        }
    }
}

impl SurfaceState {
    pub(crate) fn from_startup(
        startup: NotionStartup,
        appearance_mode: AppearanceMode,
        viewport: Viewport,
        surface_active: bool,
        notion_resources: NotionSurfaceResources,
    ) -> Self {
        let projection = StartupProjection::new(&startup, appearance_mode, viewport);
        let context = StartupSurfaceContext {
            startup,
            appearance_mode,
            viewport,
            surface_active,
            notion_resources,
        };
        let mut state = Self::from_projection(context, projection);
        state
            .notion_sidebar
            .rebuild_rows(state.board.page_shell.as_ref());
        state
    }

    fn from_projection(context: StartupSurfaceContext, projection: StartupProjection) -> Self {
        Self {
            surface_active: context.surface_active,
            appearance_mode: context.appearance_mode,
            theme: projection.theme,
            viewport: context.viewport,
            preview_width: context.viewport.app_width(),
            notion_startup: context.startup,
            notion_startup_generation: 0,
            board: projection.board,
            database_view_rows: projection.database_view_rows,
            database_view_controls: super::super::DatabaseViewControlsState {
                property_rows: projection.database_view_control_property_rows,
                mutation_in_flight: false,
            },
            snapshot_pages: projection.snapshot_pages,
            page_documents: crate::ui::surface::PageDocuments::new(
                projection.standalone_page_content,
                projection.hydrate_standalone_page,
            ),
            columns: projection.columns,
            icons: std::sync::Arc::new(IconSet::new(context.appearance_mode)),
            notion_resources: context.notion_resources,
            page_code_settings: projection.page_code_settings,
            presence_images: projection.presence_images,
            favorited: projection.favorited,
            board_view: BoardViewState::new(projection.timeline_scroll_handle),
            date_view: projection.date_view,
            notion_sidebar: projection.notion_sidebar,
            database_search: projection.database_search,
            database_filter: projection.database_filter,
            notion_search: Default::default(),
            page_mutations: PageMutationCoordinator::default(),
            page_editor: PageEditorState::default(),
            notion_chrome: projection.notion_chrome,
            comments: NotionCommentsState::default(),
            presentation: SurfacePresentationState::default(),
        }
    }
}

fn startup_code_settings(startup: &NotionStartup) -> crate::model::CardPageCodeSettingsCapability {
    match startup {
        NotionStartup::Ready(bootstrap) => bootstrap.code_settings.clone(),
        NotionStartup::Loading {
            cached: Some(cached),
            ..
        }
        | NotionStartup::Error {
            cached: Some(cached),
            ..
        } => cached.code_settings.clone(),
        NotionStartup::Loading { cached: None, .. } | NotionStartup::Error { cached: None, .. } => {
            crate::model::CardPageCodeSettingsCapability::memory()
        }
        #[cfg(any(test, feature = "test-support"))]
        NotionStartup::Fixture(_) => crate::model::CardPageCodeSettingsCapability::memory(),
    }
}

/// The startup board, with its preloaded pages when the startup source has them.
type StartupBoard = (BoardSnapshot, Option<HashMap<String, CardPage>>);

fn startup_board(startup: &NotionStartup) -> StartupBoard {
    match startup {
        #[cfg(any(test, feature = "test-support"))]
        NotionStartup::Fixture(input) => match &input.source {
            NotionFixtureSource::WorkspaceApi(_) => (input.board.clone(), None),
            NotionFixtureSource::SnapshotPages(snapshot_pages) => {
                (input.board.clone(), Some(snapshot_pages.clone()))
            }
        },
        NotionStartup::Ready(bootstrap) => (bootstrap.workspace.clone(), None),
        NotionStartup::Loading {
            cached: Some(cached),
            ..
        }
        | NotionStartup::Error {
            cached: Some(cached),
            ..
        } => (cached.workspace.clone(), None),
        NotionStartup::Loading { cached: None, .. } | NotionStartup::Error { cached: None, .. } => {
            (empty_board_snapshot(), None)
        }
    }
}
