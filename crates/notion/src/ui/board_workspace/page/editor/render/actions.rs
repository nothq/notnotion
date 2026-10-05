use gpui::{Context, Window};

use crate::ui::SurfaceState;

mod block;
mod composer;
mod document;
mod drag;
mod table;

pub(in crate::ui::board_workspace) use block::{PageBlockEditRenderAction, PageBlockRenderAction};
pub(in crate::ui::board_workspace) use composer::PageComposerRenderAction;
pub(in crate::ui::board_workspace) use document::PageDocumentRenderAction;
pub(in crate::ui::board_workspace) use drag::{PageDragRenderAction, PageLinearRowsObservation};
pub(in crate::ui::board_workspace) use table::{PageTableHistoryRestore, PageTableRenderAction};

pub(in crate::ui::board_workspace) enum PageRenderAction {
    Block(PageBlockRenderAction),
    Drag(PageDragRenderAction),
    Table(PageTableRenderAction),
    Document(PageDocumentRenderAction),
    Composer(PageComposerRenderAction),
}

pub(in crate::ui::board_workspace) fn handle_page_render_action(
    surface: &mut SurfaceState,
    action: PageRenderAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageRenderAction::Block(action) => block::handle(surface, action, window, cx),
        PageRenderAction::Drag(action) => drag::handle(surface, action, window, cx),
        PageRenderAction::Table(action) => table::handle(surface, action, window, cx),
        PageRenderAction::Document(action) => document::handle(surface, action, window, cx),
        PageRenderAction::Composer(action) => composer::handle(surface, action, cx),
    }
}
