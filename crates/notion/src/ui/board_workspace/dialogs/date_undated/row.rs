use gpui::{
    div, px, AnyElement, App, AppContext, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};

use super::{
    CalendarPageDrag, DateUndatedAction, DateUndatedRenderer, DateUndatedRow, DateUndatedRowView,
    DATE_UNDATED_ROW_GAP, DATE_UNDATED_ROW_HEIGHT,
};
use crate::ui::{alpha, rgb, FluentBuilder, FontWeight, KeyDownEvent};

impl DateUndatedRenderer {
    pub(super) fn render_row(&self, row: &DateUndatedRowView, cx: &mut App) -> AnyElement {
        let drag_enabled = self.drag_enabled && !row.assignment_pending;
        let mut rendered_row = self.render_row_shell(row, cx);
        if drag_enabled {
            let drag = CalendarPageDrag::new(&row.row.item, self.theme);
            let actions = self.actions.clone();
            rendered_row.interactivity().on_drag(
                drag,
                move |dragged: &CalendarPageDrag, _, window, cx: &mut App| {
                    cx.stop_propagation();
                    actions.emit(DateUndatedAction::DragStarted, window, cx);
                    cx.new(|_| dragged.clone())
                },
            );
        }
        div()
            .h(px(DATE_UNDATED_ROW_HEIGHT + DATE_UNDATED_ROW_GAP))
            .flex_none()
            .child(rendered_row)
            .into_any_element()
    }

    fn render_row_shell(
        &self,
        row: &DateUndatedRowView,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        let assign_item = row.row.item.clone();
        let key_assign_item = assign_item.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();

        div()
            .id(row.row.element_id.clone())
            .role(Role::MenuItem)
            .aria_label(row.row.title.clone())
            .focusable()
            .tab_stop(true)
            .h(px(DATE_UNDATED_ROW_HEIGHT))
            .w_full()
            .px(px(8.0))
            .rounded(px(5.0))
            .when(!row.assignment_pending, |row| row.cursor_pointer())
            .opacity(if row.assignment_pending { 0.45 } else { 1.0 })
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
            .on_click(move |_, window, cx| {
                if cx.has_active_drag() {
                    return;
                }
                cx.stop_propagation();
                click_actions.emit(DateUndatedAction::Assign(assign_item.clone()), window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !date_undated_control_key(event) {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    DateUndatedAction::Assign(key_assign_item.clone()),
                    window,
                    cx,
                );
            })
            .flex()
            .items_center()
            .child(self.render_row_icon(&row.row, cx))
            .child(self.render_row_title(&row.row))
            .child(self.render_row_open_button(&row.row))
    }

    fn render_row_icon(&self, row: &DateUndatedRow, cx: &mut App) -> AnyElement {
        let icon = row
            .item
            .icon
            .as_ref()
            .map(|icon| self.page_icons.render(icon, 20.0, cx))
            .unwrap_or_else(|| self.page_icons.builtin("page", 20.0, cx));
        div()
            .size(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(icon)
            .into_any_element()
    }

    fn render_row_title(&self, row: &DateUndatedRow) -> AnyElement {
        div()
            .ml(px(8.0))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .truncate()
            .text_size(px(14.0))
            .line_height(px(16.8))
            .text_color(rgb(self.theme.text_primary))
            .child(row.title.clone())
            .into_any_element()
    }

    fn render_row_open_button(&self, row: &DateUndatedRow) -> AnyElement {
        let open_item = row.item.clone();
        let key_item = open_item.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id(format!("{}-open", row.element_id.as_ref()))
            .role(Role::Button)
            .aria_label(format!("Open {}", row.title.as_ref()))
            .focusable()
            .tab_stop(true)
            .h(px(24.0))
            .flex_none()
            .px(px(6.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .line_height(px(16.8))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_secondary))
            .on_click(move |_, window, cx| {
                if cx.has_active_drag() {
                    return;
                }
                cx.stop_propagation();
                click_actions.emit(DateUndatedAction::Open(open_item.clone()), window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !date_undated_control_key(event) {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(DateUndatedAction::Open(key_item.clone()), window, cx);
            })
            .child("Open")
            .into_any_element()
    }

    pub(super) fn render_pagination_sentinel(&self, pending: bool) -> AnyElement {
        div()
            .h(px(DATE_UNDATED_ROW_HEIGHT + DATE_UNDATED_ROW_GAP))
            .flex_none()
            .child(
                div()
                    .h(px(DATE_UNDATED_ROW_HEIGHT))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .when(pending, |row| {
                        row.child(
                            div()
                                .w(px(152.0))
                                .h(px(10.0))
                                .rounded(px(4.0))
                                .bg(alpha(self.theme.text_primary, 0.055)),
                        )
                    }),
            )
            .into_any_element()
    }
}

fn date_undated_control_key(event: &KeyDownEvent) -> bool {
    !event.keystroke.modifiers.modified()
        && matches!(event.keystroke.key.as_str(), "enter" | "space")
}
