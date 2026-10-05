use crate::ui::board_workspace::PageDocumentAction;
use gpui::{Bounds, Context, Pixels, WeakEntity, Window};

use super::{
    InlineDatabaseToolbarTarget, InlineToolbarDialogAnchors, SurfaceState, ToolbarDialogKind,
};
use crate::model::NotionCollectionViewId;
use crate::ui::surface::{BoardViewState, InlineDatabaseView, NotionDateViewState};

pub(super) enum BoardToolbarAction {
    OpenDialog {
        dialog: ToolbarDialogKind,
        parent: Option<WeakEntity<SurfaceState>>,
    },
    SelectView {
        provider_view_id: NotionCollectionViewId,
        inline_view: Option<WeakEntity<InlineDatabaseView>>,
    },
    SetMinimized(bool),
    ToggleAiAutofill(Option<WeakEntity<SurfaceState>>),
    ExpandDatabase(InlineDatabaseToolbarTarget),
    CreatePage,
    ToggleUndated(Option<InlineDatabaseToolbarTarget>),
    ToggleSearch,
    SetSearchQuery(String),
    CloseSearch,
    ClearSearch,
    Measured(BoardToolbarMeasurement),
}

#[derive(Clone)]
pub(super) struct BoardToolbarMeasurement {
    pub(super) toolbar_bounds: Bounds<Pixels>,
    pub(super) undated_badge_bounds: Option<Bounds<Pixels>>,
    pub(super) inline_dialog_anchors: Option<InlineToolbarDialogAnchors>,
}

pub(super) fn handle_board_toolbar_action(
    surface: &mut SurfaceState,
    action: BoardToolbarAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        BoardToolbarAction::OpenDialog { dialog, parent } => {
            if let Some(parent) = parent {
                surface.open_inline_toolbar_dialog(dialog, &parent, cx);
            } else {
                surface.open_toolbar_dialog(dialog, cx);
            }
        }
        BoardToolbarAction::SelectView {
            provider_view_id,
            inline_view,
        } => select_toolbar_view(surface, provider_view_id, inline_view, cx),
        BoardToolbarAction::SetMinimized(minimized) => {
            surface.notion_chrome.inline_toolbar_minimized = minimized;
            surface.database_search.close();
            surface.board_view.inline_toolbar_dialog_anchors = Default::default();
            cx.notify();
        }
        BoardToolbarAction::ToggleAiAutofill(parent) => {
            surface.toggle_notion_ai_autofill(parent.as_ref(), cx)
        }
        BoardToolbarAction::ExpandDatabase(target) => {
            target
                .parent_surface
                .update(cx, |surface, cx| {
                    surface.open_notion_workspace(target.board_url, target.database_title, cx);
                })
                .expect("inline database navigation requires its parent surface");
        }
        BoardToolbarAction::CreatePage => {
            surface.dispatch_page_document_action(PageDocumentAction::CreatePrimary, cx)
        }
        BoardToolbarAction::ToggleUndated(target) => {
            surface.toggle_date_undated_dialog_from_toolbar(target, cx);
        }
        BoardToolbarAction::ToggleSearch => surface.toggle_toolbar_search(cx),
        BoardToolbarAction::SetSearchQuery(value) => {
            surface.database_search.set_query(value);
            surface.board_view.clear_hover();
            cx.notify();
        }
        BoardToolbarAction::CloseSearch => {
            surface.database_search.close();
            cx.notify();
        }
        BoardToolbarAction::ClearSearch => {
            surface.database_search.clear_query();
            cx.notify();
        }
        BoardToolbarAction::Measured(measurement) => {
            measurement.apply(&mut surface.board_view, &mut surface.date_view)
        }
    }
}

fn select_toolbar_view(
    surface: &mut SurfaceState,
    provider_view_id: NotionCollectionViewId,
    inline_view: Option<WeakEntity<InlineDatabaseView>>,
    cx: &mut Context<SurfaceState>,
) {
    if let Some(inline_view) = inline_view {
        inline_view
            .update(cx, |view, cx| {
                view.select_provider_view(provider_view_id, cx)
            })
            .expect("inline database view tab requires its inline host");
    } else {
        surface.select_database_view(provider_view_id, cx);
    }
}

impl BoardToolbarMeasurement {
    fn apply(self, board_view: &mut BoardViewState, date_view: &mut NotionDateViewState) {
        board_view.toolbar_bounds = Some(self.toolbar_bounds);
        date_view.undated_badge_bounds = self.undated_badge_bounds;
        if let Some(anchors) = self.inline_dialog_anchors {
            board_view.inline_toolbar_dialog_anchors = anchors;
        }
    }
}
