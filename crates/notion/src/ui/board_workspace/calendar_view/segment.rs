use gpui::App;

use super::render::CalendarRenderer;
use super::{
    alpha, calendar_control_key, div, img, point, px, rgb, rgba, AnyElement, AppContext, BoardItem,
    BoxShadow, CalendarAction, CalendarPageDrag, CalendarRangeResizeDrag,
    CalendarRangeResizeEndpoint, CalendarWeekSegment, CivilDate, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role, StatefulInteractiveElement,
    Styled, CALENDAR_EVENT_ROW_PITCH,
};

/// Whether a segment's date assignment is in flight, and whether it can be
/// dragged.
#[derive(Clone, Copy)]
struct CalendarSegmentInteraction {
    assignment_pending: bool,
    drag_enabled: bool,
}

impl CalendarRenderer<'_> {
    pub(super) fn render_calendar_week_segment(
        &self,
        segment: &CalendarWeekSegment<'_>,
        week_start: CivilDate,
        cx: &mut App,
    ) -> AnyElement {
        let assignment_pending = self
            .date_view
            .undated
            .assignments_in_flight
            .contains(&segment.item.block_id);
        let drag_enabled = !assignment_pending && self.mutation_available;
        let record_icon = self.calendar_segment_record_icon(segment.item, cx);
        let mut card = self.calendar_segment_card(
            segment,
            week_start,
            CalendarSegmentInteraction {
                assignment_pending,
                drag_enabled,
            },
            record_icon,
        );
        if drag_enabled {
            let drag = CalendarPageDrag::new(segment.item, self.theme);
            let actions = self.actions.clone();
            card.interactivity().on_drag(
                drag,
                move |dragged: &CalendarPageDrag, _, window, cx: &mut App| {
                    cx.stop_propagation();
                    actions.emit(CalendarAction::DragStarted, window, cx);
                    cx.new(|_| dragged.clone())
                },
            );
        }
        card.into_any_element()
    }

    fn calendar_segment_record_icon(&self, item: &BoardItem, cx: &mut App) -> Option<AnyElement> {
        if !self.calendar_view.show_page_icon {
            return None;
        }
        if let Some(icon) = item.icon.as_ref() {
            return Some(
                div()
                    .size(px(14.0))
                    .mr(px(3.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.page_icons.render(icon, 14.0, cx))
                    .into_any_element(),
            );
        }
        (!item.title.is_empty()).then(|| {
            div()
                .size(px(14.0))
                .mr(px(3.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(img(self.icons.page.render(cx)).size(px(12.6)))
                .into_any_element()
        })
    }

    fn calendar_segment_card(
        &self,
        segment: &CalendarWeekSegment<'_>,
        week_start: CivilDate,
        interaction: CalendarSegmentInteraction,
        record_icon: Option<AnyElement>,
    ) -> gpui::Stateful<Div> {
        let CalendarSegmentInteraction {
            assignment_pending,
            drag_enabled,
        } = interaction;
        let mouse_item = segment.item.clone();
        let key_item = mouse_item.clone();
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id(format!(
                "notion-calendar-item-{}-{:04}-{:02}-{:02}",
                segment.item.block_id, week_start.year, week_start.month, week_start.day
            ))
            .role(Role::Button)
            .aria_label(segment.item.title.clone())
            .focusable()
            .tab_stop(true)
            .h(px(CALENDAR_EVENT_ROW_PITCH))
            .flex_none()
            .min_w(px(0.0))
            .col_span(segment.column_count as u16)
            .col_start(segment.start_column as i16 + 1)
            .row_start(segment.lane as i16 + 1)
            .py(px(3.0))
            .pl(px(calendar_segment_left_padding(segment)))
            .pr(px(calendar_segment_right_padding(segment)))
            .when(segment.continues_into_next_week, |card| card.mr(px(1.0)))
            .when(!assignment_pending, |card| card.cursor_pointer())
            .opacity(if assignment_pending { 0.45 } else { 1.0 })
            .flex()
            .items_center()
            .on_click(move |_, window, cx| {
                if cx.has_active_drag() {
                    return;
                }
                cx.stop_propagation();
                mouse_actions.emit(CalendarAction::OpenItem(mouse_item.clone()), window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !calendar_control_key(event) {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(CalendarAction::OpenItem(key_item.clone()), window, cx);
            })
            .child(self.render_calendar_segment_link(
                segment,
                week_start,
                drag_enabled,
                record_icon,
            ))
    }

    fn render_calendar_segment_link(
        &self,
        segment: &CalendarWeekSegment<'_>,
        week_start: CivilDate,
        drag_enabled: bool,
        record_icon: Option<AnyElement>,
    ) -> AnyElement {
        div()
            .id(format!(
                "notion-calendar-item-link-{}-{:04}-{:02}-{:02}",
                segment.item.block_id, week_start.year, week_start.month, week_start.day
            ))
            .h(px(28.0))
            .w_full()
            .min_w(px(0.0))
            .relative()
            .py(px(2.0))
            .when(!segment.continues_from_previous_week, |link| {
                link.rounded_tl(px(6.0)).rounded_bl(px(6.0))
            })
            .when(!segment.continues_into_next_week, |link| {
                link.rounded_tr(px(6.0)).rounded_br(px(6.0))
            })
            .bg(rgb(self.theme.app_bg))
            .shadow(vec![BoxShadow {
                color: rgba(self.theme.surface_border),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(1.0),
                inset: false,
            }])
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.075)))
            .flex()
            .items_center()
            .child(self.render_calendar_segment_content(segment, record_icon))
            .when(
                drag_enabled && !segment.continues_from_previous_week,
                |link| {
                    link.child(self.render_calendar_range_resize_handle(
                        CalendarRangeResizeDrag::new(segment, CalendarRangeResizeEndpoint::Start),
                    ))
                },
            )
            .when(drag_enabled && !segment.continues_into_next_week, |link| {
                link.child(
                    self.render_calendar_range_resize_handle(CalendarRangeResizeDrag::new(
                        segment,
                        CalendarRangeResizeEndpoint::End,
                    )),
                )
            })
            .into_any_element()
    }

    fn render_calendar_segment_content(
        &self,
        segment: &CalendarWeekSegment<'_>,
        record_icon: Option<AnyElement>,
    ) -> AnyElement {
        div()
            .h(px(24.0))
            .w_full()
            .min_w(px(0.0))
            .pl(px(if segment.continues_from_previous_week {
                12.0
            } else {
                6.0
            }))
            .pr(px(if segment.continues_into_next_week {
                11.0
            } else {
                6.0
            }))
            .overflow_hidden()
            .flex()
            .items_center()
            .when_some(record_icon, |content, icon| content.child(icon))
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child(segment.item.title.clone()),
            )
            .into_any_element()
    }
}

fn calendar_segment_left_padding(segment: &CalendarWeekSegment<'_>) -> f32 {
    if segment.continues_from_previous_week {
        0.0
    } else {
        6.0
    }
}

fn calendar_segment_right_padding(segment: &CalendarWeekSegment<'_>) -> f32 {
    if segment.continues_into_next_week {
        0.0
    } else {
        6.0
    }
}
