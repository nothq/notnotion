use gpui::Context;

use super::actions::PageMentionAction;
use crate::ui::{view_actions::ViewActionSink, SurfaceState};

pub(in crate::ui::board_workspace::page::editor) fn page_mention_action_sink(
    cx: &Context<SurfaceState>,
) -> ViewActionSink<PageMentionAction> {
    ViewActionSink::new(cx, |surface, action, _window, cx| {
        surface.dispatch_page_mention_action(action, cx);
    })
}
