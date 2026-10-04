use crate::ui::{view_actions::ViewNotifier, Context, SurfaceState};

use super::PageShellIconRenderer;

impl SurfaceState {
    pub(crate) fn page_shell_icon_renderer(&self, cx: &Context<Self>) -> PageShellIconRenderer {
        PageShellIconRenderer::new(
            self.appearance_mode,
            self.icons.clone(),
            self.notion_resources.clone(),
            ViewNotifier::new(cx),
        )
    }
}
