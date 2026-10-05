use gpui::App;

use super::render::CalendarRenderer;
use super::{
    alpha, calendar_control_key, div, img, px, rgb, rgba, AnyElement, CalendarAction, Div,
    FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled, Window, CALENDAR_HEADER_BOTTOM_MARGIN,
    CALENDAR_HEADER_HEIGHT,
};

impl CalendarRenderer<'_> {
    pub(super) fn render_calendar_header(&self, label: &str, cx: &mut App) -> Div {
        div()
            .h(px(CALENDAR_HEADER_HEIGHT))
            .mb(px(CALENDAR_HEADER_BOTTOM_MARGIN))
            .ml(px(1.0))
            .self_stretch()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .mx(px(8.0))
                    .text_size(px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(self.render_manage_in_calendar_button(cx))
                    .child(self.render_calendar_previous_button(cx))
                    .child(self.render_calendar_today_button())
                    .child(self.render_calendar_next_button(cx)),
            )
    }

    fn render_manage_in_calendar_button(&self, cx: &mut App) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id("notion-calendar-manage")
            .role(Role::Button)
            .aria_label("Manage in Calendar")
            .focusable()
            .tab_stop(true)
            .h(px(24.0))
            .mr(px(6.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(self.theme.calendar_control_border))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.06)))
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(CalendarAction::OpenExternalCalendar, window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !calendar_control_key(event) {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(CalendarAction::OpenExternalCalendar, window, cx);
            })
            .child(img(self.icons.property_date.render(cx)).size(px(16.0)))
            .child("Manage in Calendar")
            .into_any_element()
    }

    fn render_calendar_previous_button(&self, cx: &mut App) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id("notion-calendar-previous-month")
            .role(Role::Button)
            .aria_label("Previous month")
            .focusable()
            .tab_stop(true)
            .size(px(24.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.06)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(CalendarAction::ShowPreviousMonth, window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if activate_calendar_control(event, window, cx) {
                    key_actions.emit(CalendarAction::ShowPreviousMonth, window, cx);
                }
            })
            .child(img(self.icons.calendar_chevron_left.render(cx)).size(px(20.0)))
            .into_any_element()
    }

    fn render_calendar_today_button(&self) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id("notion-calendar-today")
            .role(Role::Button)
            .aria_label("Today")
            .focusable()
            .tab_stop(true)
            .h(px(24.0))
            .px(px(6.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.06)))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(CalendarAction::ShowToday, window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if activate_calendar_control(event, window, cx) {
                    key_actions.emit(CalendarAction::ShowToday, window, cx);
                }
            })
            .child("Today")
            .into_any_element()
    }

    fn render_calendar_next_button(&self, cx: &mut App) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id("notion-calendar-next-month")
            .role(Role::Button)
            .aria_label("Next month")
            .focusable()
            .tab_stop(true)
            .size(px(24.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.06)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(CalendarAction::ShowNextMonth, window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if activate_calendar_control(event, window, cx) {
                    key_actions.emit(CalendarAction::ShowNextMonth, window, cx);
                }
            })
            .child(img(self.icons.calendar_chevron_right.render(cx)).size(px(20.0)))
            .into_any_element()
    }

    pub(super) fn calendar_grid_color(&self) -> gpui::Hsla {
        rgb(self.theme.calendar_grid_border).into()
    }
}

fn activate_calendar_control(event: &KeyDownEvent, window: &mut Window, cx: &mut App) -> bool {
    if !calendar_control_key(event) {
        return false;
    }
    window.prevent_default();
    cx.stop_propagation();
    true
}
