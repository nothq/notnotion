use crate::ui::board_workspace::PageEditSession;

use gpui::{Context, Window};

use super::{PageRichTextRenderer, PageRichTextToolbarLayout, ToolbarAction};
use crate::ui::{view_actions::ViewActionSink, AnyElement, SurfaceState};

impl SurfaceState {
    pub(crate) fn render_page_rich_text_toolbar(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let has_selection = self
            .page_editor
            .page_text_selection
            .as_ref()
            .is_some_and(|selection| !selection.pointer_active);
        let dialog = self.page_editor.page_rich_text_dialog;
        if !has_selection && dialog.is_none() {
            return None;
        }
        let anchor = self.page_editor.page_rich_text_anchor(cx)?;
        Some(
            PageRichTextRenderer {
                theme: self.theme,
                appearance_mode: self.appearance_mode,
                link_input: self.page_editor.page_rich_text_link_input.borrow().clone(),
                actions: ViewActionSink::new(cx, handle_rich_text_toolbar_action),
            }
            .render_page_rich_text_toolbar(PageRichTextToolbarLayout {
                anchor,
                viewport_width: self.viewport.logical_width as f32,
                viewport_height: self.viewport.logical_height as f32,
                has_selection,
                dialog,
            }),
        )
    }
}

fn handle_rich_text_toolbar_action(
    surface: &mut SurfaceState,
    action: ToolbarAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let action = match action {
        ToolbarAction::NormalText => return surface.turn_selected_page_blocks_into_text(cx),
        ToolbarAction::Color => {
            surface.page_editor.open_page_rich_text_color_dialog();
            cx.notify();
            return;
        }
        action => action,
    };
    let transition = {
        let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
        match action {
            ToolbarAction::ApplyLink => edit.apply_page_rich_text_link(cx),
            ToolbarAction::RemoveLink => edit.remove_page_rich_text_link(cx),
            ToolbarAction::SetColor { color, background } => {
                edit.set_page_rich_text_color(color, background, cx)
            }
            ToolbarAction::Toggle(annotation) => edit.toggle_page_text_annotation(annotation, cx),
            ToolbarAction::Link => edit.open_page_rich_text_link_dialog(cx),
            ToolbarAction::Clear => edit.clear_selected_page_text_formatting(cx),
            ToolbarAction::NormalText | ToolbarAction::Color => {
                unreachable!("host toolbar actions were dispatched before editing")
            }
        }
        edit.finish(())
    };
    surface.apply_page_edit_transition(transition, cx);
}
