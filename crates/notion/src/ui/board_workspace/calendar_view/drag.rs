use gpui::App;

use super::render::CalendarRenderer;
use super::{
    calendar_query_date, div, px, AnyElement, AppContext, CalendarAction, CalendarRangeResizeDrag,
    CalendarRangeResizeEndpoint, CalendarRangeResizePreview, CivilDate, FluentBuilder,
    InteractiveElement, IntoElement, MouseButton, SetCalendarPageDateRangeRequest,
    StatefulInteractiveElement, Styled,
};
use crate::ui::surface::NotionDateViewState;

impl CalendarRenderer<'_> {
    pub(super) fn render_calendar_range_resize_handle(
        &self,
        drag: CalendarRangeResizeDrag,
    ) -> AnyElement {
        let endpoint = drag.endpoint;
        let actions = self.actions.clone();
        div()
            .id(format!(
                "notion-calendar-resize-{}-{}",
                drag.item.block_id,
                match endpoint {
                    CalendarRangeResizeEndpoint::Start => "start",
                    CalendarRangeResizeEndpoint::End => "end",
                }
            ))
            .absolute()
            .top(px(0.0))
            .when(endpoint == CalendarRangeResizeEndpoint::Start, |handle| {
                handle.left(px(-4.0))
            })
            .when(endpoint == CalendarRangeResizeEndpoint::End, |handle| {
                handle.right(px(-4.0))
            })
            .h_full()
            .w(px(12.0))
            .cursor_col_resize()
            .block_mouse_except_scroll()
            .on_drag(
                drag,
                move |dragged: &CalendarRangeResizeDrag, _, window, cx: &mut App| {
                    cx.stop_propagation();
                    actions.emit(
                        CalendarAction::BeginRangeResize(dragged.clone()),
                        window,
                        cx,
                    );
                    cx.new(|_| dragged.clone())
                },
            )
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .occlude()
            .into_any_element()
    }
}

impl NotionDateViewState {
    pub(super) fn preview_range_resize(
        &mut self,
        drag: &CalendarRangeResizeDrag,
        target_date: CivilDate,
    ) -> bool {
        drag.target_date.set(target_date);
        let (inclusive_start, inclusive_end) = drag.resized_range(target_date);
        let next_preview = CalendarRangeResizePreview {
            block_id: drag.item.block_id.clone(),
            inclusive_start,
            inclusive_end,
        };
        if self.range_resize_preview.as_ref() == Some(&next_preview) {
            return false;
        }
        self.range_resize_preview = Some(next_preview);
        true
    }

    pub(super) fn range_resize_preview_is_stale(&self, active_drag: bool) -> bool {
        self.range_resize_preview.is_some() && !active_drag
    }

    pub(super) fn clear_range_resize_preview(&mut self) -> bool {
        self.range_resize_preview.take().is_some()
    }

    pub(super) fn begin_range_resize(
        &mut self,
        request: &SetCalendarPageDateRangeRequest,
        inclusive_start: CivilDate,
        inclusive_end: CivilDate,
        date_property_id: &str,
    ) -> Option<crate::ui::Arc<()>> {
        let block_id = request.block_id();
        if self.undated.assignments_in_flight.contains(block_id) {
            return None;
        }
        self.apply_range_resize_to_loaded_item(
            block_id,
            inclusive_start,
            inclusive_end,
            date_property_id,
        );
        self.undated
            .assignments_in_flight
            .insert(block_id.to_string());
        Some(self.query_session.clone())
    }

    fn apply_range_resize_to_loaded_item(
        &mut self,
        block_id: &str,
        inclusive_start: CivilDate,
        inclusive_end: CivilDate,
        date_property_id: &str,
    ) {
        let mut items = self.calendar_items.items.to_vec();
        let item = items
            .iter_mut()
            .find(|item| item.block_id == block_id)
            .expect("resized Notion Calendar item must remain in the loaded month");
        let date_value = item
            .properties
            .iter_mut()
            .find(|property| property.property_id == date_property_id)
            .and_then(|property| property.date.as_mut())
            .expect("resized Notion Calendar item must retain its configured date property");
        date_value.start_date = calendar_query_date(inclusive_start);
        date_value.end_date = Some(calendar_query_date(inclusive_end));
        self.calendar_items.items = items.into();
    }

    pub(super) fn complete_range_resize(
        &mut self,
        block_id: &str,
        session: &crate::ui::Arc<()>,
    ) -> bool {
        if !crate::ui::Arc::ptr_eq(&self.query_session, session) {
            return false;
        }
        self.undated.assignments_in_flight.remove(block_id);
        true
    }
}
