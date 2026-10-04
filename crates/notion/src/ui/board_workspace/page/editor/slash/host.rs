use gpui::{Context, Window};

use super::super::PageEditSession;
use super::PageSlashMenuRenderer;
use crate::ui::{view_actions::ViewActionSink, PageCommandTarget, SurfaceState};

pub(super) enum PageSlashAction {
    Dismiss,
    Apply {
        block_id: String,
        target: PageCommandTarget,
    },
    Hover {
        block_id: String,
        index: usize,
    },
}

impl SurfaceState {
    pub(in crate::ui::board_workspace::page::editor) fn page_slash_menu_renderer(
        &self,
        cx: &Context<Self>,
    ) -> Option<PageSlashMenuRenderer> {
        Some(PageSlashMenuRenderer {
            menu: self.page_editor.page_slash_menu.clone()?,
            theme: self.theme,
            icons: self.icons.clone(),
            viewport: self.viewport,
            input_resources: self.page_editor.input.resources(),
            actions: ViewActionSink::new(cx, handle_page_slash_action),
        })
    }
}

fn handle_page_slash_action(
    surface: &mut SurfaceState,
    action: PageSlashAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageSlashAction::Dismiss => surface.page_editor.dismiss_page_block_interaction(cx),
        PageSlashAction::Apply { block_id, target } => {
            let transition = {
                let mut session =
                    PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                session.apply_native_page_slash_command(&block_id, target);
                session.finish(())
            };
            surface.apply_page_edit_transition(transition, cx);
        }
        PageSlashAction::Hover { block_id, index } => {
            if surface
                .page_editor
                .hover_native_page_slash_selection(&block_id, index)
            {
                cx.notify();
            }
        }
    }
}
