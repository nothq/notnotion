use super::super::{
    div, img, lightning_icon, px, relative, render_svg_image, rgb, rgba, AnyElement, Div,
    FontWeight, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Styled,
};
use super::primary::{ToolbarDialogAction, ToolbarDialogRenderer};

impl ToolbarDialogRenderer {
    pub(super) fn render_automation_dialog(&self, cx: &mut gpui::App) -> AnyElement {
        (self
            .render_dialog_surface(px(392.0), 168.0, 112.0)
            .h(px(207.0))
            .px(px(18.0))
            .pt(px(16.0))
            .pb(px(14.0))
            .relative()
            .flex()
            .flex_col()
            .child(self.render_automation_dialog_close_button(cx))
            .child(self.render_automation_dialog_empty_state(cx)))
        .into_any_element()
    }

    pub(super) fn render_actions_dialog(&self, cx: &mut gpui::App) -> AnyElement {
        let actions = [
            "Copy link",
            "Duplicate",
            "Move to Trash",
            "Lock database",
            "Import",
            "Merge with CSV",
            "Export",
            "Updates & analytics",
            "Version history",
            "Notify me",
            "Connections",
            "Open in Mac app",
        ];

        (self
            .render_dialog_surface(px(336.0), 40.0, 8.0)
            .pt(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .child(self.render_actions_dialog_search(cx))
            .child(self.render_actions_dialog_rows(actions))
            .child(self.render_actions_dialog_footer()))
        .into_any_element()
    }

    fn render_automation_dialog_close_button(&self, cx: &mut gpui::App) -> AnyElement {
        (div()
            .absolute()
            .top(px(12.0))
            .right(px(12.0))
            .size(px(24.0))
            .rounded_full()
            .bg(rgba(self.theme.dialog_input_bg))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ToolbarDialogAction::Close
                }),
            )
            .child(img(self.icons.close.render(cx)).size(px(14.0))))
        .into_any_element()
    }

    fn render_automation_dialog_empty_state(&self, cx: &mut gpui::App) -> AnyElement {
        (div()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .child(self.render_automation_dialog_icon(cx))
            .child(self.render_automation_dialog_title())
            .child(self.render_automation_dialog_description()))
        .into_any_element()
    }

    fn render_automation_dialog_icon(&self, cx: &mut gpui::App) -> AnyElement {
        (img(render_svg_image(lightning_icon(0xd4a037), cx)).size(px(18.0))).into_any_element()
    }

    fn render_automation_dialog_title(&self) -> AnyElement {
        (div()
            .text_size(px(14.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(self.theme.text_primary))
            .child("Automations"))
        .into_any_element()
    }

    fn render_automation_dialog_description(&self) -> AnyElement {
        (div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .child(self.automation_dialog_description_line("Automatically edit properties, create"))
            .child(self.automation_dialog_description_line("pages, send updates, and more")))
        .into_any_element()
    }

    fn automation_dialog_description_line(&self, text: &'static str) -> Div {
        div()
            .text_size(px(12.0))
            .line_height(relative(1.4))
            .text_color(rgb(self.theme.text_muted))
            .child(text)
    }

    fn render_actions_dialog_search(&self, cx: &mut gpui::App) -> AnyElement {
        (div()
            .mx(px(8.0))
            .mb(px(4.0))
            .h(px(32.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgba(self.theme.dialog_input_bg))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(img(self.icons.toolbar_search.render(cx)).size(px(14.0)))
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("Search actions..."),
            ))
        .into_any_element()
    }

    fn render_actions_dialog_rows(&self, actions: [&'static str; 12]) -> AnyElement {
        (div().children(
            actions
                .into_iter()
                .map(|label| self.render_actions_dialog_row(label)),
        ))
        .into_any_element()
    }

    fn render_actions_dialog_row(&self, label: &'static str) -> AnyElement {
        (div()
            .mx(px(4.0))
            .h(px(28.0))
            .px(px(12.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_primary))
            .child(label))
        .into_any_element()
    }

    fn render_actions_dialog_footer(&self) -> Div {
        div()
            .mx(px(8.0))
            .mt(px(4.0))
            .border_t_1()
            .border_color(rgba(self.theme.surface_border))
            .pt(px(8.0))
            .pb(px(4.0))
            .child(
                (div()
                    .h(px(28.0))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(rgb(self.theme.text_secondary))
                    .child("Learn about databases"))
                .into_any_element(),
            )
    }

    pub(super) fn render_templates_dialog(&self, _cx: &mut gpui::App) -> AnyElement {
        (self
            .render_dialog_surface(px(360.0), 168.0, 76.0)
            .px(px(18.0))
            .pt(px(16.0))
            .pb(px(14.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                (div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child(format!("Templates for {}", self.database_title)))
                .into_any_element(),
            )
            .child(
                (div()
                    .text_size(px(13.0))
                    .line_height(relative(1.5))
                    .text_color(rgb(self.theme.text_muted))
                    .child("Create a reusable page template for this database."))
                .into_any_element(),
            )
            .child(
                (div()
                    .h(px(36.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(rgba(self.theme.surface_border))
                    .bg(rgba(self.theme.dialog_input_bg))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child("New template"))
                .into_any_element(),
            ))
        .into_any_element()
    }
}
