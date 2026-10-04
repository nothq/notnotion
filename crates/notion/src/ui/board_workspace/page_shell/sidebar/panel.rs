use super::{
    alpha, div, px, relative, rgb, AnyElement, FontWeight, InteractiveElement, IntoElement,
    NotionSidebarRow, ParentElement, SidebarRenderer, Styled, SIDEBAR_ROW_HEIGHT,
};
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_panel_header(
        &self,
        row: &NotionSidebarRow,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .id(row.element_id.clone())
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .mb(px(1.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(relative(1.0))
                    .text_color(rgb(super::notion_sidebar_panel_muted(self.appearance_mode)))
                    .child(row.label.clone()),
            )
            .into_any_element()
    }

    pub(super) fn render_notion_sidebar_calendar_event(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        let secondary_label = row
            .secondary_label
            .clone()
            .expect("calendar sidebar row requires a time label");
        let element = div()
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(
                div()
                    .size(px(20.0))
                    .mr(px(8.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_notion_page_shell_icon(&row.icon, 10.0, cx)),
            )
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(relative(1.2))
                    .text_color(rgb(self.notion_sidebar_row_text_color()))
                    .child(row.label.clone()),
            )
            .child(
                div()
                    .ml(px(8.0))
                    .flex_none()
                    .whitespace_nowrap()
                    .text_size(px(12.0))
                    .line_height(relative(1.2))
                    .text_color(rgb(super::notion_sidebar_panel_muted(self.appearance_mode)))
                    .child(secondary_label),
            );
        self.bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone())
            .into_any_element()
    }

    pub(super) fn render_notion_sidebar_panel_action(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        let element = div()
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(
                div()
                    .size(px(20.0))
                    .mr(px(8.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_notion_page_shell_icon(&row.icon, 16.0, cx)),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(relative(1.2))
                    .text_color(rgb(super::notion_sidebar_panel_muted(self.appearance_mode)))
                    .child(row.label.clone()),
            );
        self.bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone())
            .into_any_element()
    }

    pub(super) fn render_notion_sidebar_panel_empty(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(row.element_id.clone())
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .px(px(8.0))
            .flex()
            .items_center()
            .child(
                div()
                    .size(px(20.0))
                    .mr(px(8.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_notion_page_shell_icon(&row.icon, 16.0, cx)),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(relative(1.5))
                    .text_color(rgb(super::notion_sidebar_muted(self.appearance_mode)))
                    .child(row.label.clone()),
            )
            .into_any_element()
    }

    pub(super) fn render_notion_sidebar_skeleton(&self, row: &NotionSidebarRow) -> AnyElement {
        div()
            .id(row.element_id.clone())
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .px(px(8.0))
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(142.0))
                    .h(px(10.0))
                    .rounded(px(5.0))
                    .bg(alpha(self.notion_sidebar_row_text_color(), 0.12)),
            )
            .into_any_element()
    }
}
