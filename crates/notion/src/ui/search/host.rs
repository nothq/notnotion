use crate::ui::{view_actions::ViewActionSink, AnyElement, Context, SurfaceState};

use super::{
    actions::quick_find_action_input,
    render::{QuickFindRenderConfig, QuickFindView},
    QuickFindAction,
};

impl SurfaceState {
    pub(crate) fn render_notion_search_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let actions = ViewActionSink::new(cx, |surface, action, _window, cx| {
            surface.apply_quick_find_action(action, cx);
        });
        let config = QuickFindRenderConfig {
            appearance_mode: self.appearance_mode,
            theme: self.theme,
            viewport: self.viewport,
            workspace_name: self.board.page_shell.as_ref().map_or_else(
                || "Notion".to_string(),
                |shell| shell.workspace_name.clone(),
            ),
            search_open: self.notion_chrome.notion_search_open,
            icons: self.icons.clone(),
            page_icons: self.page_shell_icon_renderer(cx),
            actions,
        };
        QuickFindView::new(&self.notion_search, config, cx).render(cx)
    }

    pub(crate) fn apply_quick_find_action(
        &mut self,
        action: QuickFindAction,
        cx: &mut Context<Self>,
    ) {
        let input = quick_find_action_input(&action);
        let effects = self.notion_search.reduce_quick_find_action(action, input);
        self.execute_quick_find_effects(effects, cx);
    }
}
