use gpui::{App, Context, Window};

use crate::ui::board_workspace::PageEditSession;
use crate::ui::SurfaceState;

pub(in crate::ui::board_workspace) enum PageBlockRenderAction {
    TogglePageLinkIcon { page_id: String, block_id: String },
    DismissPageLinkIcon,
    ToggleDisclosure { page_id: String, block_id: String },
    Edit(PageBlockEditRenderAction),
    ToggleCodeLanguagePicker(String),
    ToggleContextMenu(String),
    DismissInteraction,
}

pub(in crate::ui::board_workspace) enum PageBlockEditRenderAction {
    InsertFirstToggleChild(String),
    ToggleToDo(String),
    CopyCode(String),
    InsertRelative { block_id: String, before: bool },
}

pub(super) fn handle(
    surface: &mut SurfaceState,
    action: PageBlockRenderAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageBlockRenderAction::TogglePageLinkIcon { page_id, block_id } => {
            surface.toggle_page_link_icon_picker(page_id, block_id, cx)
        }
        PageBlockRenderAction::DismissPageLinkIcon => {
            if surface.page_editor.page_link_icons.dismiss_picker() {
                cx.notify();
            }
        }
        PageBlockRenderAction::ToggleDisclosure { page_id, block_id } => {
            surface.toggle_page_block_disclosure(&page_id, &block_id, window, cx)
        }
        PageBlockRenderAction::Edit(action) => {
            let transition = {
                let mut edit =
                    PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                edit.apply_block_render_edit(action, cx);
                edit.finish(())
            };
            surface.apply_page_edit_transition(transition, cx);
        }
        PageBlockRenderAction::ToggleCodeLanguagePicker(block_id) => {
            surface.toggle_page_code_language_picker(block_id, window, cx)
        }
        PageBlockRenderAction::ToggleContextMenu(block_id) => {
            surface.toggle_page_block_context_menu(block_id, window, cx)
        }
        PageBlockRenderAction::DismissInteraction => {
            surface.page_editor.dismiss_page_block_interaction(cx)
        }
    }
}

impl PageEditSession<'_> {
    fn apply_block_render_edit(&mut self, action: PageBlockEditRenderAction, cx: &mut App) {
        match action {
            PageBlockEditRenderAction::InsertFirstToggleChild(block_id) => {
                self.insert_first_page_toggle_child(&block_id, cx);
            }
            PageBlockEditRenderAction::ToggleToDo(block_id) => {
                self.toggle_page_to_do_state(&block_id, cx)
            }
            PageBlockEditRenderAction::CopyCode(block_id) => self.copy_page_code_text(&block_id),
            PageBlockEditRenderAction::InsertRelative { block_id, before } => {
                self.insert_page_block_relative(&block_id, before, cx)
            }
        }
    }
}
