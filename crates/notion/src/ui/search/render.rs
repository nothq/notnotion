use std::sync::Arc;

use gpui::{App, Entity, ListState, ScrollHandle};
use gpui_components::text_input::TextInput;

use crate::model::SearchWorkspaceScope;
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::{
    surface::{
        NotionSearchListRow, NotionSearchPreviewState, NotionSearchResultsState, NotionSearchState,
    },
    view_actions::ViewActionSink,
    AppearanceMode, IconSet, Theme, Viewport,
};

use super::{input::QuickFindInputConfig, QuickFindAction};

mod controls;
mod filters;
mod footer;
mod preview;
mod preview_blocks;
mod preview_chrome;
mod preview_intro;
mod preview_style;
mod shell;

#[cfg(test)]
mod tests;

const SEARCH_MODAL_CONTENT_WIDTH: f32 = 1006.0;
const SEARCH_MODAL_OUTER_WIDTH: f32 = 1008.0;
const SEARCH_MODAL_OUTER_HEIGHT: f32 = 702.0;
const SEARCH_HEADER_HEIGHT: f32 = 48.0;
const SEARCH_FILTERS_HEIGHT: f32 = 44.0;
const SEARCH_BODY_HEIGHT: f32 = 567.0;
const SEARCH_RESULTS_WIDTH: f32 = 600.0;
const SEARCH_PREVIEW_WIDTH: f32 = 406.0;
const SEARCH_FOOTER_HEIGHT: f32 = 41.0;
const SEARCH_LOADING_HEADER_HEIGHT: f32 = 68.0;
const SEARCH_LOADING_FILTERS_HEIGHT: f32 = 48.0;
const SEARCH_LOADING_BODY_HEIGHT: f32 = 543.0;
const SEARCH_LOADING_HEADER_HORIZONTAL_INSET: f32 = 20.0;
const SEARCH_SEARCH_ICON_SIZE: f32 = 20.0;
const SEARCH_LOADING_SEARCH_ICON_SIZE: f32 = 22.0;
const SEARCH_LOADED_HEADER_LEFT_INSET: f32 = 17.0;
const SEARCH_DARK_BORDER_FILL: u32 = 0x353533;
const SEARCH_PREVIEW_DARK_HEADER_FILL: u32 = 0x202020;
const SEARCH_PREVIEW_DARK_BODY_FILL: u32 = 0x191919;
const SEARCH_PREVIEW_LIGHT_HEADER_FILL: u32 = 0xf7f6f3;
const SEARCH_PREVIEW_LIGHT_BODY_FILL: u32 = 0xffffff;
const SEARCH_PREVIEW_CARD_HEIGHT_PX: usize = 522;

#[derive(Clone)]
pub(super) struct QuickFindRenderConfig {
    pub(super) appearance_mode: AppearanceMode,
    pub(super) theme: Theme,
    pub(super) viewport: Viewport,
    pub(super) workspace_name: String,
    pub(super) search_open: bool,
    pub(super) icons: Arc<IconSet>,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) actions: ViewActionSink<QuickFindAction>,
}

#[derive(Clone)]
pub(super) struct QuickFindViewState {
    pub(super) query: String,
    pub(super) scope: SearchWorkspaceScope,
    pub(super) selected_index: usize,
    pub(super) results: NotionSearchResultsState,
    pub(super) preview: NotionSearchPreviewState,
    pub(super) preview_scroll_handle: ScrollHandle,
    pub(super) list_state: ListState,
    pub(super) list_rows: Arc<[NotionSearchListRow]>,
}

impl QuickFindViewState {
    fn new(state: &NotionSearchState) -> Self {
        Self {
            query: state.query.clone(),
            scope: state.scope,
            selected_index: state.selected_index,
            results: state.results.clone(),
            preview: state.preview.clone(),
            preview_scroll_handle: state.preview_scroll_handle.clone(),
            list_state: state.list_state.clone(),
            list_rows: state.list_rows_snapshot(),
        }
    }

    pub(super) fn visible_results(&self) -> &[crate::model::PageShellSearchResult] {
        match &self.results {
            NotionSearchResultsState::Loaded { results, .. } => results,
            NotionSearchResultsState::Idle
            | NotionSearchResultsState::Loading
            | NotionSearchResultsState::Failed => &[],
        }
    }
}

#[derive(Clone)]
pub(super) struct QuickFindView {
    pub(super) state: QuickFindViewState,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) theme: Theme,
    pub(super) viewport: Viewport,
    pub(super) icons: Arc<IconSet>,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) actions: ViewActionSink<QuickFindAction>,
    pub(super) input: Entity<TextInput>,
}

impl QuickFindView {
    pub(super) fn new(
        state: &NotionSearchState,
        config: QuickFindRenderConfig,
        cx: &mut App,
    ) -> Self {
        let input = state.input_entity(
            QuickFindInputConfig {
                query: state.query.clone(),
                workspace_name: config.workspace_name,
                request_focus: config.search_open,
                loading: matches!(&state.results, NotionSearchResultsState::Loading),
                theme: config.theme,
            },
            config.actions.clone(),
            cx,
        );
        Self {
            state: QuickFindViewState::new(state),
            appearance_mode: config.appearance_mode,
            theme: config.theme,
            viewport: config.viewport,
            icons: config.icons,
            page_icons: config.page_icons,
            actions: config.actions,
            input,
        }
    }
}
