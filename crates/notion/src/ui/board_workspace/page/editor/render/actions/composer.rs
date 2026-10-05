use gpui::Context;

use crate::ui::{PageCommandTarget, SurfaceState};

pub(in crate::ui::board_workspace) enum PageComposerRenderAction {
    Select(PageCommandTarget),
    Hover(usize),
    CloseMenu,
}

pub(super) fn handle(
    surface: &mut SurfaceState,
    action: PageComposerRenderAction,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageComposerRenderAction::Select(target) => surface.dispatch_page_composer_action(
            crate::ui::board_workspace::page::commands::state::PageComposerAction::Select(target),
            cx,
        ),
        PageComposerRenderAction::Hover(index) => {
            surface.page_editor.input.composer.selected_command_index = index;
            cx.notify();
        }
        PageComposerRenderAction::CloseMenu => {
            surface.page_editor.close_page_composer_slash_menu(cx)
        }
    }
}
