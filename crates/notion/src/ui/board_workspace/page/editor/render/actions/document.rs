use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::StatusPropertyPickerAction;
use gpui::{Context, Pixels, Point, SharedString, Window};

use crate::model::CardPageProperty;
use crate::ui::SurfaceState;

pub(in crate::ui::board_workspace) enum PageDocumentRenderAction {
    OpenWorkspace {
        url: String,
        label: String,
    },
    OpenStatusPicker {
        page_id: SharedString,
        property: CardPageProperty,
        position: Point<Pixels>,
    },
    OpenComments(String),
    NavigateSelectedPageBack,
    CloseSelectedPage,
}

pub(super) fn handle(
    surface: &mut SurfaceState,
    action: PageDocumentRenderAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageDocumentRenderAction::OpenWorkspace { url, label } => {
            surface.open_notion_workspace(url, label, cx)
        }
        PageDocumentRenderAction::OpenStatusPicker {
            page_id,
            property,
            position,
        } => surface.dispatch_status_property_picker_action(
            StatusPropertyPickerAction::open_page(page_id, property, position),
            cx,
        ),
        PageDocumentRenderAction::OpenComments(page_id) => {
            surface.open_notion_page_comments(page_id.clone(), page_id, cx)
        }
        PageDocumentRenderAction::NavigateSelectedPageBack => {
            surface.dispatch_page_document_action(PageDocumentAction::NavigateBack, cx)
        }
        PageDocumentRenderAction::CloseSelectedPage => {
            surface.dispatch_page_document_action(PageDocumentAction::CloseSelected, cx)
        }
    }
}
