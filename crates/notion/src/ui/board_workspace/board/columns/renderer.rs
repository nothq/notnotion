use super::host::BoardColumnAction;
use crate::ui::{
    board_workspace::PageShellIconRenderer,
    surface::{BoardViewState, DatabaseSearchState},
    view_actions::ViewActionSink,
    AppearanceMode, ColumnState, IconSet, Theme,
};

pub(crate) struct BoardColumnsRenderer<'a> {
    pub(in crate::ui::board_workspace) columns: &'a [ColumnState],
    pub(in crate::ui::board_workspace) board_view: &'a BoardViewState,
    pub(in crate::ui::board_workspace) database_search: &'a DatabaseSearchState,
    pub(in crate::ui::board_workspace) theme: Theme,
    pub(in crate::ui::board_workspace) appearance_mode: AppearanceMode,
    pub(in crate::ui::board_workspace) icons: &'a IconSet,
    pub(in crate::ui::board_workspace) page_icons: PageShellIconRenderer,
    pub(in crate::ui::board_workspace) actions: ViewActionSink<BoardColumnAction>,
}
