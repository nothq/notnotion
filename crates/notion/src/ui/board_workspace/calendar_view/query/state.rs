use super::super::{
    add_days, calendar_grid_start, calendar_query_date, civil_date_month_start,
    local_today_civil_date, CivilDate, CreateCalendarPageRequest, LoadCalendarItemsRequest,
    CALENDAR_DAY_COUNT,
};
use super::types::{
    CalendarMonthLoad, CalendarMonthLoadCompletion, CalendarMonthLoadToken, CalendarPageCreation,
};
use crate::model::{BoardItem, LoadCalendarItemsResult, NotionWorkspaceResult};
use crate::ui::surface::NotionDateViewState;
use crate::ui::Arc;

impl NotionDateViewState {
    pub(super) fn prepare_page_creation(
        &mut self,
        date: CivilDate,
        view_identity: String,
    ) -> Option<CalendarPageCreation> {
        let date_key = calendar_query_date(date);
        if !self
            .calendar_items
            .creations_in_flight
            .insert(date_key.clone())
        {
            return None;
        }
        let request = CreateCalendarPageRequest::for_day(date_key.clone())
            .expect("rendered Notion Calendar days must retain exact YYYY-MM-DD dates");
        Some(CalendarPageCreation {
            request,
            date_key,
            view_identity,
            session: self.query_session.clone(),
        })
    }

    pub(super) fn complete_page_creation(
        &mut self,
        creation: &CalendarPageCreation,
        active_view_identity: Option<&str>,
    ) -> bool {
        self.calendar_items
            .creations_in_flight
            .remove(&creation.date_key);
        Arc::ptr_eq(&self.query_session, &creation.session)
            && active_view_identity == Some(creation.view_identity.as_str())
    }

    pub(super) fn ensure_calendar_items(
        &mut self,
        view_identity: &str,
        board_items: &[BoardItem],
        calendar_is_active: bool,
    ) -> Option<CivilDate> {
        if self.calendar_items.view_identity.as_deref() != Some(view_identity) {
            self.reset_calendar_items(Some(view_identity.to_string()), board_items);
        }
        (calendar_is_active && !self.calendar_items.initialized && !self.calendar_items.pending)
            .then_some(self.visible_month)
    }

    pub(super) fn begin_query_schedule(&self, has_active_date_view: bool) -> bool {
        has_active_date_view && !self.query_schedule_pending.replace(true)
    }

    pub(super) fn finish_query_schedule(&self) {
        self.query_schedule_pending.set(false);
    }

    pub(in crate::ui::board_workspace) fn prepare_month_load(
        &mut self,
        month: CivilDate,
        view_identity: Option<String>,
        board_items: &[BoardItem],
        workspace_available: bool,
    ) -> Option<CalendarMonthLoad> {
        if self.visible_month != month {
            self.calendar_items.day_focus_handles.borrow_mut().clear();
        }
        self.visible_month = month;
        let view_identity = view_identity?;
        if self.calendar_items.view_identity.as_deref() != Some(view_identity.as_str()) {
            self.reset_calendar_items(Some(view_identity.clone()), board_items);
        }
        self.calendar_items.initialized = true;
        if !workspace_available {
            self.calendar_items.loaded_month = Some(month);
            self.calendar_items.pending = false;
            self.calendar_items.requested_month = None;
            return None;
        }

        let grid_start = calendar_grid_start(month);
        let grid_end = add_days(grid_start, CALENDAR_DAY_COUNT as i64 - 1);
        let request = LoadCalendarItemsRequest::month(
            calendar_query_date(grid_start),
            calendar_query_date(grid_end),
        )
        .expect("Notion Calendar month requests must retain an inclusive 42-day grid");
        self.calendar_items.generation = self
            .calendar_items
            .generation
            .checked_add(1)
            .expect("Notion Calendar request generation exhausted");
        self.calendar_items.pending = true;
        self.calendar_items.requested_month = Some(month);
        Some(CalendarMonthLoad {
            request,
            token: CalendarMonthLoadToken {
                month,
                generation: self.calendar_items.generation,
                view_identity,
                session: self.query_session.clone(),
            },
        })
    }

    pub(super) fn complete_month_load(
        &mut self,
        token: &CalendarMonthLoadToken,
        active_view_identity: Option<&str>,
        result: NotionWorkspaceResult<LoadCalendarItemsResult>,
    ) -> CalendarMonthLoadCompletion {
        if !Arc::ptr_eq(&self.query_session, &token.session)
            || self.calendar_items.generation != token.generation
            || self.calendar_items.requested_month != Some(token.month)
            || active_view_identity != Some(token.view_identity.as_str())
        {
            return CalendarMonthLoadCompletion::Stale;
        }
        self.calendar_items.pending = false;
        self.calendar_items.requested_month = None;
        match result {
            Ok(result) => {
                self.calendar_items.items = result.items.into();
                self.calendar_items.loaded_month = Some(token.month);
                CalendarMonthLoadCompletion::Loaded
            }
            Err(error) => CalendarMonthLoadCompletion::Failed(error),
        }
    }

    pub(super) fn roll_back_failed_month_load(&mut self, month: CivilDate) {
        if self.visible_month == month {
            self.visible_month = self
                .calendar_items
                .loaded_month
                .unwrap_or(self.visible_month);
        }
    }

    pub(in crate::ui::board_workspace) fn default_assignment_date(
        &self,
        timeline_is_active: bool,
    ) -> CivilDate {
        let today = local_today_civil_date();
        if timeline_is_active {
            return today;
        }
        let visible_month = civil_date_month_start(self.visible_month);
        if today.year == visible_month.year && today.month == visible_month.month {
            today
        } else {
            calendar_grid_start(visible_month)
        }
    }
}
