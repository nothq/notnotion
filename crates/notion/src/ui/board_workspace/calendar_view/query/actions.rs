use super::super::{
    calendar_query_date, civil_date_month_start, local_today_civil_date, previous_month,
    CalendarAction, CalendarRangeResizeDrag, CivilDate, Context, SetCalendarPageDateRangeRequest,
    SurfaceState, Window,
};
use super::types::{CalendarRangeResizeJob, DateViewContext, DateViewEffect, DateViewRequest};
use crate::model::SetCalendarPageDateRequest;
use crate::ui::surface::NotionDateViewState;

pub(in super::super) fn handle_calendar_action(
    surface: &mut SurfaceState,
    action: CalendarAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let workspace_available = surface.notion_startup.workspace_api().is_some();
    let context = DateViewContext::from_board(&surface.board, workspace_available);
    let effects = surface.date_view.reduce_calendar_action(action, &context);
    surface.execute_date_view_effects(effects, cx);
}

impl NotionDateViewState {
    fn reduce_calendar_action(
        &mut self,
        action: CalendarAction,
        context: &DateViewContext,
    ) -> Vec<DateViewEffect> {
        match action {
            CalendarAction::OpenExternalCalendar => vec![DateViewEffect::OpenExternalCalendar],
            CalendarAction::ShowPreviousMonth => {
                self.month_effect(previous_month(self.visible_month))
            }
            CalendarAction::ShowToday => {
                self.month_effect(civil_date_month_start(local_today_civil_date()))
            }
            CalendarAction::ShowNextMonth => {
                self.month_effect(crate::ui::next_month(self.visible_month))
            }
            CalendarAction::CreatePage(date) => self.create_page_effects(date, context),
            CalendarAction::AssignPageDate { item, date } => {
                let request = SetCalendarPageDateRequest::from_calendar_day_drop(
                    item.block_id,
                    calendar_query_date(date),
                )
                .expect("Notion Calendar day drop must retain a valid block id and date");
                self.assignment_effects(request, context)
            }
            CalendarAction::OpenItem(item) => {
                vec![DateViewEffect::OpenBoardItem(Box::new(item))]
            }
            CalendarAction::DragStarted => vec![DateViewEffect::Notify],
            CalendarAction::BeginRangeResize(drag) => {
                self.preview_range_resize(&drag, drag.target_date.get());
                vec![DateViewEffect::Notify]
            }
            CalendarAction::PreviewRangeResize { drag, target_date } => self
                .preview_range_resize(&drag, target_date)
                .then_some(DateViewEffect::Notify)
                .into_iter()
                .collect(),
            CalendarAction::FinishRangeResize(drag) => self.finish_resize_effects(drag, context),
        }
    }

    fn month_effect(&self, month: CivilDate) -> Vec<DateViewEffect> {
        vec![DateViewEffect::Request(Box::new(
            DateViewRequest::CalendarMonth(month),
        ))]
    }

    fn create_page_effects(
        &mut self,
        date: CivilDate,
        context: &DateViewContext,
    ) -> Vec<DateViewEffect> {
        if context.board_locked || !context.calendar_is_active() || !context.workspace_available {
            return Vec::new();
        }
        let Some(view_identity) = context.active_view_identity.clone() else {
            return Vec::new();
        };
        let Some(creation) = self.prepare_page_creation(date, view_identity) else {
            return Vec::new();
        };
        vec![
            DateViewEffect::Request(Box::new(DateViewRequest::CreateCalendarPage(creation))),
            DateViewEffect::Notify,
        ]
    }

    pub(in crate::ui::board_workspace) fn assignment_effects(
        &mut self,
        request: crate::model::SetCalendarPageDateRequest,
        context: &DateViewContext,
    ) -> Vec<DateViewEffect> {
        let session = self.query_session.clone();
        let Some(effect) = self
            .undated
            .prepare_assignment_effect(request, context, session)
        else {
            return Vec::new();
        };
        vec![effect, DateViewEffect::Notify]
    }

    fn finish_resize_effects(
        &mut self,
        drag: CalendarRangeResizeDrag,
        context: &DateViewContext,
    ) -> Vec<DateViewEffect> {
        let (inclusive_start, inclusive_end) = drag.resized_range(drag.target_date.get());
        self.clear_range_resize_preview();
        if inclusive_start == drag.inclusive_start && inclusive_end == drag.inclusive_end {
            return vec![DateViewEffect::Notify];
        }
        let request = SetCalendarPageDateRangeRequest::from_calendar_endpoint_resize(
            drag.item.block_id,
            calendar_query_date(inclusive_start),
            calendar_query_date(inclusive_end),
        )
        .expect("Notion Calendar endpoint resize must retain an ordered inclusive date range");
        if context.active_identity().is_none() || !context.workspace_available {
            return Vec::new();
        }
        let date_property_id = context
            .calendar_date_property_id
            .as_deref()
            .expect("active Calendar resize must retain Calendar configuration");
        let block_id = request.block_id().to_string();
        let Some(session) =
            self.begin_range_resize(&request, inclusive_start, inclusive_end, date_property_id)
        else {
            return Vec::new();
        };
        vec![
            DateViewEffect::Request(Box::new(DateViewRequest::ResizeCalendarRange(
                CalendarRangeResizeJob {
                    request,
                    block_id,
                    session,
                },
            ))),
            DateViewEffect::Notify,
        ]
    }
}
