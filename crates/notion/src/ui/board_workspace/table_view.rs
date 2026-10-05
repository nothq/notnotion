use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::StatusPropertyPickerAction;
use gpui::{Pixels, Point, SharedString};

use super::{
    div, px, AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState, ViewTabKind, Window, ACTIVE_VIEW_HEIGHT,
    BOARD_VIEWPORT_X, BOARD_WIDTH,
};
use crate::ui::{view_actions::ViewActionSink, BoardItem, ToolbarDialogKind};

mod cells;
mod renderer;

pub(super) use renderer::TableViewGeometry;
use renderer::{TableViewRenderer, TableViewResources};

pub(super) enum TableViewAction {
    Toolbar(ToolbarDialogKind),
    Item(BoardItem),
    StatusPicker {
        page_id: SharedString,
        property_id: SharedString,
        current_value: SharedString,
        position: Point<Pixels>,
    },
}

impl SurfaceState {
    pub(crate) fn render_active_view(
        &self,
        columns: Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) -> Div {
        let board_viewport_width = self.board_viewport().width;
        match self.board.active_view_kind() {
            ViewTabKind::Board | ViewTabKind::Unknown => div().h(px(ACTIVE_VIEW_HEIGHT)).child(
                (div()
                    .id("board-scroll")
                    .h_full()
                    .w(px(board_viewport_width))
                    .overflow_x_scroll()
                    .track_scroll(&self.board_view.scroll_handle)
                    .on_scroll_wheel(cx.listener(Self::handle_board_scroll))
                    .child(
                        div().w(px(self.board_viewport().content_width())).child(
                            div()
                                .ml(px(BOARD_VIEWPORT_X))
                                .w(px(BOARD_WIDTH))
                                .flex()
                                .gap_3()
                                .children(columns),
                        ),
                    ))
                .into_any_element(),
            ),
            ViewTabKind::Table => table_view_renderer(self, cx)
                .render_table_view(self.viewport.board_viewport_width(), cx),
            ViewTabKind::List => {
                super::list_gallery_view::database_view_renderer(self, cx).render_list_view(cx)
            }
            ViewTabKind::Gallery => {
                super::list_gallery_view::database_view_renderer(self, cx).render_gallery_view(cx)
            }
            ViewTabKind::Timeline => self.render_timeline_view(cx),
            ViewTabKind::Calendar => self.render_calendar_view(cx),
        }
    }
}

pub(super) fn table_view_renderer<'a>(
    surface: &'a SurfaceState,
    cx: &Context<SurfaceState>,
) -> TableViewRenderer<'a> {
    TableViewRenderer::new(
        &surface.board,
        &surface.database_search,
        TableViewResources {
            theme: surface.theme,
            appearance_mode: surface.appearance_mode,
            icons: surface.icons.clone(),
            page_icons: surface.page_shell_icon_renderer(cx),
            status_editing_available: !surface.board.is_locked
                && surface.notion_startup.workspace_api().is_some()
                && surface.notion_startup.board_url().is_some(),
            actions: ViewActionSink::new(cx, handle_table_view_action),
        },
    )
}

fn handle_table_view_action(
    surface: &mut SurfaceState,
    action: TableViewAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        TableViewAction::Toolbar(dialog) => surface.open_toolbar_dialog(dialog, cx),
        TableViewAction::Item(item) => surface
            .dispatch_page_document_action(PageDocumentAction::OpenBoardItem((item).clone()), cx),
        TableViewAction::StatusPicker {
            page_id,
            property_id,
            current_value,
            position,
        } => surface.dispatch_status_property_picker_action(
            StatusPropertyPickerAction::open_table(page_id, property_id, current_value, position),
            cx,
        ),
    }
}
