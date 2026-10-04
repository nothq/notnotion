use gpui::{AnyElement, Context};

use super::{NotionAiRenderer, NotionAiViewState};
use crate::ui::{surface::SurfaceState, view_actions::ViewActionSink};

impl SurfaceState {
    pub(crate) fn render_notion_ai_button(&self, cx: &mut Context<Self>) -> AnyElement {
        self.notion_ai_renderer(cx).render_notion_ai_button()
    }

    pub(crate) fn render_notion_ai_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        self.notion_ai_renderer(cx).render_notion_ai_panel(cx)
    }

    fn notion_ai_renderer(&self, cx: &mut Context<Self>) -> NotionAiRenderer<'_> {
        NotionAiRenderer {
            state: NotionAiViewState {
                mode: self.notion_chrome.notion_ai_mode,
                mode_menu_open: self.notion_chrome.notion_ai_mode_menu_open,
                mode_menu_active: self.notion_chrome.notion_ai_mode_menu_active,
                input: &self.notion_chrome.notion_ai_input,
                input_active: self.notion_chrome.notion_ai_input_active,
                block_context: self.notion_chrome.notion_ai_block_context.as_deref(),
            },
            app_bg: self.theme.app_bg,
            page_title: &self.board.page_title,
            page_icon: self.icons.page.render(cx),
            panel_width: self.page_layout().ai_panel_width(),
            actions: ViewActionSink::new(cx, |this, action, _, cx| {
                if this.notion_chrome.apply_notion_ai_action(action) {
                    cx.notify();
                }
            }),
        }
    }
}
