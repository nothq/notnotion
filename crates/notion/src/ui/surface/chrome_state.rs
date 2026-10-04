use super::{
    Arc, Bounds, DatabaseProperty, InlineDatabaseViewMenuState, NotionAiMode,
    NotionPendingNavigation, NotionShareDialogState, Pixels, SharedString, SurfaceState,
    ToolbarDialogKind, WeakEntity, NOTION_PAGE_SIDEBAR_DEFAULT_WIDTH,
};
use crate::model::{DatabaseStatusProperty, NotionCollectionViewId, NotionLaunchRoute};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionNavigationDirection {
    Backward,
    Forward,
}

#[derive(Clone, Debug)]
pub(crate) enum NotionNavigationHistoryUpdate {
    Untracked,
    Push {
        source: NotionLaunchRoute,
    },
    Traverse {
        direction: NotionNavigationDirection,
        source: NotionLaunchRoute,
        target: NotionLaunchRoute,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct NotionNavigationHistory {
    backward: Vec<NotionLaunchRoute>,
    forward: Vec<NotionLaunchRoute>,
}

impl NotionNavigationHistory {
    pub(crate) fn target(&self, direction: NotionNavigationDirection) -> Option<NotionLaunchRoute> {
        match direction {
            NotionNavigationDirection::Backward => self.backward.last(),
            NotionNavigationDirection::Forward => self.forward.last(),
        }
        .cloned()
    }

    pub(crate) fn record_success(
        &mut self,
        update: NotionNavigationHistoryUpdate,
        destination: &NotionLaunchRoute,
    ) {
        match update {
            NotionNavigationHistoryUpdate::Untracked => {}
            NotionNavigationHistoryUpdate::Push { source } => {
                assert_ne!(
                    &source, destination,
                    "Notion navigation push must change routes"
                );
                self.backward.push(source);
                self.forward.clear();
            }
            NotionNavigationHistoryUpdate::Traverse {
                direction,
                source,
                target,
            } => {
                assert_eq!(
                    &target, destination,
                    "Notion history completion must install its requested target"
                );
                assert_eq!(
                    self.target(direction).as_ref(),
                    Some(&target),
                    "Notion history target changed before successful completion"
                );
                let traversed = match direction {
                    NotionNavigationDirection::Backward => self.backward.pop(),
                    NotionNavigationDirection::Forward => self.forward.pop(),
                };
                assert_eq!(
                    traversed.as_ref(),
                    Some(&target),
                    "Notion history target changed before successful completion"
                );
                match direction {
                    NotionNavigationDirection::Backward => self.forward.push(source),
                    NotionNavigationDirection::Forward => self.backward.push(source),
                }
            }
        }
    }
}

pub(crate) struct NotionChromeState {
    pub(crate) toolbar_dialog: Option<ToolbarDialogKind>,
    pub(crate) inline_toolbar_dialog: Option<InlineToolbarDialogState>,
    pub(crate) date_undated_dialog: Option<DateUndatedDialogState>,
    pub(crate) inline_database_view_menu: Option<InlineDatabaseViewMenuState>,
    pub(crate) inline_toolbar_minimized: bool,
    pub(crate) ai_autofill_dialog: Option<AiAutofillDialogState>,
    pub(crate) status_property_picker: Option<StatusPropertyPickerState>,
    pub(crate) share_dialog: Option<NotionShareDialogState>,
    pub(crate) notion_search_open: bool,
    pub(crate) notion_sidebar_visible: bool,
    pub(crate) notion_page_menu_open: bool,
    pub(crate) notion_page_top_controls_visible: bool,
    pub(crate) notion_ai_open: bool,
    pub(crate) notion_ai_input: String,
    pub(crate) notion_ai_input_active: bool,
    pub(crate) notion_ai_block_context: Option<String>,
    pub(crate) notion_ai_mode: NotionAiMode,
    pub(crate) notion_ai_mode_menu_open: bool,
    pub(crate) notion_ai_mode_menu_active: NotionAiMode,
    pub(crate) notion_sidebar_width: f32,
    pub(crate) notion_pending_navigation: Option<NotionPendingNavigation>,
    pub(crate) notion_navigation_request_id: u64,
    pub(crate) notion_navigation_history: NotionNavigationHistory,
}

impl NotionChromeState {
    pub(crate) fn top_bar_popover_open(&self) -> bool {
        self.share_dialog.is_some()
    }

    pub(crate) fn reset_for_route(&mut self, sidebar_visible: bool) {
        self.ai_autofill_dialog = None;
        self.share_dialog = None;
        self.toolbar_dialog = None;
        self.inline_toolbar_dialog = None;
        self.date_undated_dialog = None;
        self.inline_database_view_menu = None;
        self.notion_search_open = false;
        self.notion_page_menu_open = false;
        self.notion_page_top_controls_visible = false;
        self.notion_ai_open = false;
        self.notion_ai_input.clear();
        self.notion_ai_input_active = false;
        self.notion_ai_block_context = None;
        self.notion_ai_mode = NotionAiMode::Sidebar;
        self.notion_ai_mode_menu_open = false;
        self.notion_ai_mode_menu_active = NotionAiMode::Sidebar;
        self.notion_sidebar_visible = sidebar_visible;
    }

    pub(crate) fn new(notion_sidebar_visible: bool) -> Self {
        Self {
            toolbar_dialog: None,
            inline_toolbar_dialog: None,
            date_undated_dialog: None,
            inline_database_view_menu: None,
            inline_toolbar_minimized: false,
            ai_autofill_dialog: None,
            status_property_picker: None,
            share_dialog: None,
            notion_search_open: false,
            notion_sidebar_visible,
            notion_page_menu_open: false,
            notion_page_top_controls_visible: false,
            notion_ai_open: false,
            notion_ai_input: String::new(),
            notion_ai_input_active: false,
            notion_ai_block_context: None,
            notion_ai_mode: NotionAiMode::Sidebar,
            notion_ai_mode_menu_open: false,
            notion_ai_mode_menu_active: NotionAiMode::Sidebar,
            notion_sidebar_width: NOTION_PAGE_SIDEBAR_DEFAULT_WIDTH,
            notion_pending_navigation: None,
            notion_navigation_request_id: 0,
            notion_navigation_history: NotionNavigationHistory::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct StatusPropertyPickerState {
    pub(crate) session: Arc<()>,
    pub(crate) anchor: Bounds<Pixels>,
    pub(crate) source: StatusPropertyPickerSource,
    pub(crate) target: StatusPropertyPickerTarget,
    pub(crate) property: DatabaseStatusProperty,
    pub(crate) current_option_id: Option<SharedString>,
    pub(crate) commit_in_flight: bool,
}

#[derive(Clone)]
pub(crate) enum StatusPropertyPickerSource {
    Current,
    Inline(WeakEntity<SurfaceState>),
}

#[derive(Clone)]
pub(crate) enum StatusPropertyPickerTarget {
    DatabaseView {
        page_id: SharedString,
        expected_view_id: NotionCollectionViewId,
    },
    LoadedPage {
        page_id: SharedString,
    },
}

#[derive(Clone)]
pub(crate) enum DateUndatedDialogSource {
    Full {
        session: Arc<()>,
    },
    Inline {
        session: Arc<()>,
        surface: WeakEntity<SurfaceState>,
    },
}

#[derive(Clone)]
pub(crate) struct DateUndatedDialogState {
    pub(crate) anchor: Bounds<Pixels>,
    pub(crate) source: DateUndatedDialogSource,
}

#[derive(Clone)]
pub(crate) struct InlineToolbarDialogState {
    pub(crate) anchor: Bounds<Pixels>,
    pub(crate) dialog: ToolbarDialogKind,
    pub(crate) surface: WeakEntity<SurfaceState>,
}

#[derive(Clone)]
pub(crate) struct AiAutofillDialogState {
    pub(crate) anchor: Bounds<Pixels>,
    pub(crate) inline_database: bool,
    pub(crate) properties: Arc<[DatabaseProperty]>,
    pub(crate) property_picker_open: bool,
    pub(crate) selected_property_index: Option<usize>,
    pub(crate) trial_open: bool,
    pub(crate) trial_consent_checked: bool,
}
