use gpui::{AnyElement, Context};

use crate::ui::view_actions::ViewActionSink;
use crate::ui::SurfaceState;

use super::ShareDialogView;

impl SurfaceState {
    /// The popover under the top bar's Share button.
    pub(crate) fn render_top_bar_popover(&self, cx: &mut Context<Self>) -> AnyElement {
        let right_inset = self.page_layout().consuming_ai_width() + 8.0;
        let dialog = self
            .notion_chrome
            .share_dialog
            .as_ref()
            .expect("sharing overlay requires dialog state");
        ShareDialogView {
            dialog,
            theme: self.theme,
            right_inset,
            actions: ViewActionSink::new(cx, |surface, event, _, cx| {
                surface.dispatch_notion_share_event(event, cx);
            }),
        }
        .render(cx)
    }
}
