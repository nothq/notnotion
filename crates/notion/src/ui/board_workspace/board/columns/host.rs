use super::BoardColumnsRenderer;
use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::{view_actions::ViewActionSink, CardLocation, HoveredCardAction, SurfaceState};
use gpui::{Context, Pixels, Point, Window};

pub(in crate::ui::board_workspace) enum BoardColumnAction {
    HoverHeader {
        column_index: usize,
        hovered: bool,
    },
    HoverCard {
        location: CardLocation,
        hovered: bool,
    },
    HoverAction {
        location: CardLocation,
        action: HoveredCardAction,
        hovered: bool,
    },
    MouseDown {
        location: CardLocation,
        position: Point<Pixels>,
    },
    CreatePage(usize),
}

impl SurfaceState {
    pub(crate) fn board_columns_renderer(&self, cx: &Context<Self>) -> BoardColumnsRenderer<'_> {
        BoardColumnsRenderer {
            columns: &self.columns,
            board_view: &self.board_view,
            database_search: &self.database_search,
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            icons: &self.icons,
            page_icons: self.page_shell_icon_renderer(cx),
            actions: ViewActionSink::new(cx, handle_board_column_action),
        }
    }
}

fn handle_board_column_action(
    surface: &mut SurfaceState,
    action: BoardColumnAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let changed = match action {
        BoardColumnAction::HoverHeader {
            column_index,
            hovered,
        } => surface
            .board_view
            .set_column_header_hover(column_index, hovered),
        BoardColumnAction::HoverCard { location, hovered } => {
            surface.board_view.set_card_hover(location, hovered)
        }
        BoardColumnAction::HoverAction {
            location,
            action,
            hovered,
        } => surface
            .board_view
            .set_card_action_hover(location, action, hovered),
        BoardColumnAction::MouseDown { location, position } => {
            surface.handle_card_mouse_down(location, position, cx);
            false
        }
        BoardColumnAction::CreatePage(column_index) => {
            surface.dispatch_page_document_action(
                PageDocumentAction::CreateInColumn(column_index),
                cx,
            );
            false
        }
    };
    if changed {
        cx.notify();
    }
}
