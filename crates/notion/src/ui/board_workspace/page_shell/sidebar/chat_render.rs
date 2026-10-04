use super::{
    alpha, div, px, relative, rgb, AnyElement, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, NotionSidebarRow, ParentElement, SidebarRenderer, Styled,
};
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_chat_agents(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(row.element_id.clone())
            .w_full()
            .h(px(166.0))
            .grid()
            .grid_cols(2)
            .gap(px(1.0))
            .children([
                self.render_notion_sidebar_agent_card("Notion AI", "notion-ai", false, cx),
                self.render_notion_sidebar_agent_card("New agent", "add", true, cx),
                self.render_notion_sidebar_agent_card("More", "more", false, cx),
            ])
            .into_any_element()
    }

    fn render_notion_sidebar_agent_card(
        &self,
        label: &'static str,
        icon: &'static str,
        dashed: bool,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .h(px(82.0))
            .p(px(8.0))
            .rounded(px(8.0))
            .when(dashed, |this| {
                this.border_1()
                    .border_dashed()
                    .border_color(alpha(self.notion_sidebar_row_text_color(), 0.22))
            })
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(44.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!dashed, |this| {
                        this.bg(alpha(self.notion_sidebar_top_text_color(), 0.08))
                    })
                    .child(self.render_notion_sidebar_builtin_icon_at_size(icon, 24.0, cx)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(relative(1.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.notion_sidebar_top_text_color()))
                    .child(label),
            )
            .into_any_element()
    }

    pub(super) fn render_notion_sidebar_chat_thread(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        let element = div()
            .w_full()
            .min_h(px(30.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .when(row.active, |this| this.bg(self.notion_sidebar_hover_bg()))
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(
                div()
                    .size(px(20.0))
                    .flex_none()
                    .rounded_full()
                    .bg(alpha(self.notion_sidebar_top_text_color(), 0.08))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_notion_page_shell_icon(&row.icon, 16.0, cx)),
            )
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .line_height(relative(1.2))
                    .text_color(rgb(self.notion_sidebar_top_text_color()))
                    .child(row.label.clone()),
            )
            .when(row.secondary_label.is_some() && !row.active, |this| {
                this.child(
                    div()
                        .size(px(8.0))
                        .flex_none()
                        .rounded_full()
                        .bg(rgb(0x2383e2)),
                )
            });
        self.bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone())
            .into_any_element()
    }
}
