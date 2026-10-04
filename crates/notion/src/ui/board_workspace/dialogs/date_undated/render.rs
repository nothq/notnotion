use std::{collections::HashSet, sync::Arc};

use gpui::{
    uniform_list, AnyElement, App, Entity, InteractiveElement, IntoElement, ListSizingBehavior,
    MouseButton, MouseDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled,
    UniformListScrollHandle,
};
use gpui_components::backdrop::dismissible_backdrop_with_handler;
use gpui_components::text_input::TextInput;

use super::layout::{
    date_undated_dialog_background, date_undated_dialog_border, date_undated_dialog_layout,
    date_undated_dialog_shadow, DateUndatedLayoutInput,
};
use super::{
    DateUndatedAction, DateUndatedDialogLayout, DateUndatedDialogView, DateUndatedItemsState,
    DateUndatedRenderer, DateUndatedRow, DateUndatedRowView, DATE_UNDATED_BOTTOM_PADDING,
    DATE_UNDATED_DIALOG_WIDTH, DATE_UNDATED_INSTRUCTION_HEIGHT, DATE_UNDATED_ROW_HEIGHT,
    DATE_UNDATED_SEARCH_HEADER_HEIGHT,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, div, px, rgb, FluentBuilder, FontWeight};

struct DateUndatedRenderSnapshot {
    input: Entity<TextInput>,
    rows: Arc<[DateUndatedRow]>,
    assignments_in_flight: Arc<HashSet<String>>,
    scroll_handle: UniformListScrollHandle,
    pending: bool,
    pagination_armed: bool,
    loaded: bool,
    can_load_more: bool,
}

impl DateUndatedRenderSnapshot {
    fn new(state: &DateUndatedItemsState) -> Self {
        Self {
            input: state
                .input
                .borrow()
                .clone()
                .expect("open Notion no-date dialog must retain its search input"),
            rows: Arc::clone(&state.rows),
            assignments_in_flight: Arc::new(state.assignments_in_flight.clone()),
            scroll_handle: state.scroll_handle.clone(),
            pending: state.pending,
            pagination_armed: state.pagination_armed,
            loaded: state.loaded,
            can_load_more: state.can_load_more(),
        }
    }
}

impl DateUndatedDialogView<'_> {
    pub(super) fn render(self, cx: &mut App) -> AnyElement {
        let snapshot = DateUndatedRenderSnapshot::new(self.state);
        let layout = date_undated_dialog_layout(DateUndatedLayoutInput {
            anchor: self.anchor,
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
            row_count: snapshot.rows.len(),
            pending: snapshot.pending,
            loaded: snapshot.loaded,
        });
        let renderer = DateUndatedRenderer {
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            page_icons: self.page_icons,
            drag_enabled: self.drag_enabled,
            actions: self.actions.clone(),
        };
        let dismiss_actions = self.actions;

        div()
            .id("notion-date-undated-overlay")
            .absolute()
            .inset_0()
            .when(!cx.has_active_drag(), |overlay| {
                overlay.child(dismissible_backdrop_with_handler(
                    div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                    move |_, window, cx| {
                        dismiss_actions.emit(DateUndatedAction::Dismiss, window, cx);
                    },
                ))
            })
            .child(renderer.render_dialog(snapshot, layout, self.instruction, cx))
            .into_any_element()
    }
}

impl DateUndatedRenderer {
    fn render_dialog(
        &self,
        snapshot: DateUndatedRenderSnapshot,
        layout: DateUndatedDialogLayout,
        instruction: &'static str,
        cx: &mut App,
    ) -> AnyElement {
        let input = snapshot.input.clone();
        div()
            .id("notion-date-undated-dialog")
            .role(Role::Dialog)
            .aria_label("Pages without a date")
            .absolute()
            .left(px(layout.left))
            .top(px(layout.top))
            .w(px(DATE_UNDATED_DIALOG_WIDTH))
            .h(px(layout.height))
            .occlude()
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(date_undated_dialog_border(self.appearance_mode))
            .bg(date_undated_dialog_background(self.appearance_mode))
            .shadow(date_undated_dialog_shadow(self.appearance_mode))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
            .child(self.render_search_header(input))
            .child(self.render_instruction(instruction))
            .child(self.render_dialog_body(snapshot, layout.list_height, cx))
            .child(div().h(px(DATE_UNDATED_BOTTOM_PADDING)).flex_none())
            .into_any_element()
    }

    fn render_search_header(&self, input: Entity<TextInput>) -> AnyElement {
        div()
            .h(px(DATE_UNDATED_SEARCH_HEADER_HEIGHT))
            .flex_none()
            .px(px(12.0))
            .py(px(10.0))
            .child(div().w(px(296.0)).h(px(28.0)).child(input))
            .into_any_element()
    }

    fn render_instruction(&self, instruction: &'static str) -> AnyElement {
        div()
            .h(px(DATE_UNDATED_INSTRUCTION_HEIGHT))
            .flex_none()
            .px(px(4.0))
            .pt(px(4.0))
            .child(
                div()
                    .mt(px(6.0))
                    .px(px(8.0))
                    .text_size(px(12.0))
                    .line_height(px(14.4))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_muted))
                    .child(instruction),
            )
            .into_any_element()
    }

    fn render_dialog_body(
        &self,
        snapshot: DateUndatedRenderSnapshot,
        list_height: f32,
        cx: &mut App,
    ) -> AnyElement {
        if snapshot.pending && snapshot.rows.is_empty() {
            return self.render_pending_rows(list_height);
        }
        if snapshot.loaded && snapshot.rows.is_empty() {
            return self.render_empty_results(list_height);
        }
        if snapshot.rows.is_empty() {
            return div().h(px(list_height)).into_any_element();
        }
        self.render_list(snapshot, list_height, cx)
    }

    fn render_pending_rows(&self, list_height: f32) -> AnyElement {
        div()
            .h(px(list_height))
            .px(px(12.0))
            .flex()
            .flex_col()
            .children((0..2).map(|index| {
                div()
                    .h(px(DATE_UNDATED_ROW_HEIGHT))
                    .flex_none()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(168.0 - index as f32 * 4.0))
                            .h(px(10.0))
                            .rounded(px(4.0))
                            .bg(alpha(self.theme.text_primary, 0.055)),
                    )
            }))
            .into_any_element()
    }

    fn render_empty_results(&self, list_height: f32) -> AnyElement {
        div()
            .h(px(list_height))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_muted))
            .child("No results")
            .into_any_element()
    }

    fn render_list(
        &self,
        snapshot: DateUndatedRenderSnapshot,
        list_height: f32,
        _cx: &mut App,
    ) -> AnyElement {
        let arm_actions = self.actions.clone();

        div()
            .id("notion-date-undated-menu")
            .role(Role::Menu)
            .aria_label("Pages without a date")
            .h(px(list_height))
            .px(px(4.0))
            .overflow_hidden()
            .on_scroll_wheel(move |_, window, cx| {
                arm_actions.emit(DateUndatedAction::ArmPagination, window, cx);
            })
            .child(date_undated_uniform_list(self.clone(), snapshot))
            .into_any_element()
    }
}

fn date_undated_uniform_list(
    renderer: DateUndatedRenderer,
    snapshot: DateUndatedRenderSnapshot,
) -> impl IntoElement {
    let DateUndatedRenderSnapshot {
        rows,
        assignments_in_flight,
        scroll_handle,
        pending,
        pagination_armed,
        can_load_more,
        ..
    } = snapshot;
    let row_count = rows.len();
    let virtual_row_count = row_count + usize::from(can_load_more);
    let sentinel_pending = pending || pagination_armed;
    let load_more_actions: ViewActionSink<DateUndatedAction> = renderer.actions.clone();
    uniform_list(
        "notion-date-undated-rows",
        virtual_row_count,
        move |range, window, cx| {
            if can_load_more && range.end > row_count {
                load_more_actions.emit(DateUndatedAction::LoadMore, window, cx);
            }
            range
                .map(|index| {
                    rows.get(index).map_or_else(
                        || renderer.render_pagination_sentinel(sentinel_pending),
                        |row| {
                            renderer.render_row(
                                &DateUndatedRowView {
                                    assignment_pending: assignments_in_flight
                                        .contains(&row.item.block_id),
                                    row: row.clone(),
                                },
                                cx,
                            )
                        },
                    )
                })
                .collect::<Vec<_>>()
        },
    )
    .with_sizing_behavior(ListSizingBehavior::Auto)
    .track_scroll(&scroll_handle)
    .size_full()
}
