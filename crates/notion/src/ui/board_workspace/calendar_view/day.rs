use gpui::App;

use super::render::CalendarRenderer;
use super::{
    alpha, calendar_control_key, calendar_day_is_weekend, calendar_query_date, div, img,
    local_today_civil_date, month_name, point, px, rgb, short_month_name, AnyElement, BoxShadow,
    CalendarAction, CalendarDay, CalendarLayout, CalendarPageDrag, CivilDate, ClickEvent, Div,
    FluentBuilder, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    SharedString, StatefulInteractiveElement, Styled, TIMELINE_TODAY_COLOR,
};

struct CalendarDayRenderState {
    date: CivilDate,
    current_month: bool,
    is_today: bool,
    day_label: String,
    date_key: String,
    date_mutation_available: bool,
    creation_available: bool,
    creation_pending: bool,
    day_group: SharedString,
    day_focus_handle: Option<gpui::FocusHandle>,
}

impl CalendarRenderer<'_> {
    pub(super) fn ensure_calendar_day_focus_handles(
        &self,
        layout: &CalendarLayout<'_>,
        cx: &mut App,
    ) {
        if !self.creation_available {
            return;
        }
        let mut focus_handles = self.date_view.calendar_items.day_focus_handles.borrow_mut();
        for day in layout.weeks.iter().flat_map(|week| &week.days) {
            let date_key = calendar_query_date(day.date);
            focus_handles
                .entry(date_key)
                .or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    pub(super) fn render_calendar_day(
        &self,
        day: &CalendarDay,
        month_start: CivilDate,
        week_height: f32,
        cx: &mut App,
    ) -> AnyElement {
        let state = self.calendar_day_render_state(day.date, month_start);
        let day_cell = self.render_calendar_day_cell(&state, week_height);
        day_cell
            .when(
                state.creation_available && !state.creation_pending,
                |cell| cell.child(self.render_calendar_day_add_affordance(&state, cx)),
            )
            .child(self.render_calendar_day_label(&state))
            .into_any_element()
    }

    fn calendar_day_render_state(
        &self,
        date: CivilDate,
        month_start: CivilDate,
    ) -> CalendarDayRenderState {
        let is_today = date == local_today_civil_date();
        let day_label = if is_today {
            date.day.to_string()
        } else if date.day == 1 {
            format!("{} {}", short_month_name(date.month), date.day)
        } else {
            date.day.to_string()
        };
        let date_key = calendar_query_date(date);
        let day_focus_handle = self.creation_available.then(|| {
            self.date_view
                .calendar_items
                .day_focus_handles
                .borrow()
                .get(&date_key)
                .expect("editable Calendar day must retain its prepared focus handle")
                .clone()
        });
        CalendarDayRenderState {
            date,
            current_month: date.year == month_start.year && date.month == month_start.month,
            is_today,
            day_label,
            creation_pending: self
                .date_view
                .calendar_items
                .creations_in_flight
                .contains(&date_key),
            day_group: format!("notion-calendar-day-group-{date_key}").into(),
            date_key,
            date_mutation_available: self.mutation_available,
            creation_available: self.creation_available,
            day_focus_handle,
        }
    }

    fn render_calendar_day_cell(
        &self,
        state: &CalendarDayRenderState,
        week_height: f32,
    ) -> gpui::Stateful<Div> {
        let drop_date = state.date;
        let focus_handle = state.day_focus_handle.clone();
        let date_mutation_available = state.date_mutation_available;
        let drop_actions = self.actions.clone();
        div()
            .id(format!(
                "notion-calendar-day-{:04}-{:02}-{:02}",
                state.date.year, state.date.month, state.date.day
            ))
            .role(Role::GridCell)
            .aria_label(format!(
                "{} {}, {}",
                month_name(state.date.month),
                state.date.day,
                state.date.year
            ))
            .relative()
            .h(px(week_height))
            .min_w(px(0.0))
            .border_r_1()
            .border_b_1()
            .border_color(self.calendar_grid_color())
            .bg(rgb(self.calendar_day_background(state.date)))
            .when(state.creation_available, |cell| {
                self.configure_calendar_day_creation(
                    cell,
                    state.day_group.clone(),
                    focus_handle,
                    state.date,
                )
            })
            .can_drop(move |dragged, _, _| {
                date_mutation_available && dragged.downcast_ref::<CalendarPageDrag>().is_some()
            })
            .drag_over::<CalendarPageDrag>(|style, _, _, _| {
                style.border_color(rgb(0x2383e2)).bg(alpha(0x2383e2, 0.1))
            })
            .on_drop(move |dragged: &CalendarPageDrag, window, cx| {
                cx.stop_propagation();
                drop_actions.emit(
                    CalendarAction::AssignPageDate {
                        item: dragged.item.clone(),
                        date: drop_date,
                    },
                    window,
                    cx,
                );
            })
            .overflow_hidden()
    }

    fn configure_calendar_day_creation(
        &self,
        cell: gpui::Stateful<Div>,
        day_group: SharedString,
        focus_handle: Option<gpui::FocusHandle>,
        date: CivilDate,
    ) -> gpui::Stateful<Div> {
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        cell.group(day_group)
            .track_focus(
                focus_handle
                    .as_ref()
                    .expect("editable Calendar day must retain its focus handle"),
            )
            .tab_stop(true)
            .on_click(move |event: &ClickEvent, window, cx| {
                if event.click_count() != 2 || cx.has_active_drag() {
                    return;
                }
                cx.stop_propagation();
                click_actions.emit(CalendarAction::CreatePage(date), window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !calendar_control_key(event) {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(CalendarAction::CreatePage(date), window, cx);
            })
    }

    fn render_calendar_day_add_affordance(
        &self,
        state: &CalendarDayRenderState,
        cx: &mut App,
    ) -> AnyElement {
        let plus_date = state.date;
        let focus_handle = state
            .day_focus_handle
            .as_ref()
            .expect("editable Calendar add affordance must retain its day focus handle");
        let actions = self.actions.clone();
        div()
            .absolute()
            .top(px(6.0))
            .left(px(6.0))
            .size(px(24.0))
            .p(px(2.0))
            .rounded(px(8.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(calendar_day_add_shadow(self.theme.text_primary))
            .opacity(0.0)
            .group_hover(state.day_group.clone(), |mut style| {
                style.opacity = Some(1.0);
                style
            })
            .track_focus(focus_handle)
            .tab_stop(false)
            .focus_visible(|style| style.opacity(1.0))
            .child(
                div()
                    .id(format!("notion-calendar-day-add-{}", state.date_key))
                    .role(Role::Button)
                    .aria_label("Add an item")
                    .size(px(20.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        if !cx.has_active_drag() {
                            actions.emit(CalendarAction::CreatePage(plus_date), window, cx);
                        }
                    })
                    .child(img(self.icons.header_plus.render(cx)).size(px(20.0))),
            )
            .into_any_element()
    }

    fn render_calendar_day_label(&self, state: &CalendarDayRenderState) -> AnyElement {
        div()
            .absolute()
            .top(px(4.0))
            .when(state.is_today, |label| {
                label
                    .right(px(5.0))
                    .size(px(24.0))
                    .rounded_full()
                    .bg(rgb(TIMELINE_TODAY_COLOR))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x000000))
            })
            .when(!state.is_today, |label| {
                label
                    .right(px(10.0))
                    .text_color(rgb(if state.current_month {
                        self.theme.text_primary
                    } else {
                        self.theme.calendar_muted_text
                    }))
            })
            .text_size(px(14.0))
            .line_height(px(24.0))
            .child(state.day_label.clone())
            .into_any_element()
    }

    fn calendar_day_background(&self, date: CivilDate) -> u32 {
        if calendar_day_is_weekend(date) {
            self.theme.calendar_weekend_bg
        } else {
            self.theme.app_bg
        }
    }
}

fn calendar_day_add_shadow(text_primary: u32) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(text_primary, 0.14),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 0.18),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
