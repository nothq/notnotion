use super::render::CalendarRenderer;
use super::{
    add_days, build_calendar_week_segments, calendar_date_range_for_item, calendar_grid_start,
    calendar_week_height, civil_date_month_start, BoardDateValue, BoardItem, CalendarDatedItem,
    CalendarDay, CalendarLayout, CalendarViewConfig, CalendarWeek, CivilDate, CALENDAR_DAY_COUNT,
    CALENDAR_WEEK_COUNT,
};
use crate::ui::surface::NotionDateViewState;

#[derive(Clone)]
pub(super) struct CalendarGridGeometry {
    grid_start: CivilDate,
    week_heights: Vec<f32>,
}

impl CalendarGridGeometry {
    pub(super) fn date_at_grid_position(
        &self,
        position: gpui::Point<gpui::Pixels>,
        grid: gpui::Bounds<gpui::Pixels>,
    ) -> CivilDate {
        let window_x = position.x.as_f32();
        let window_y = position.y.as_f32();
        let grid_left = grid.origin.x.as_f32();
        let grid_top = grid.origin.y.as_f32();
        let grid_width = grid.size.width.as_f32();
        let column = (((window_x - grid_left) / grid_width.max(1.0)) * 7.0)
            .floor()
            .clamp(0.0, 6.0) as usize;
        let grid_height = self.week_heights.iter().sum::<f32>();
        let local_y = (window_y - grid_top).clamp(0.0, (grid_height - f32::EPSILON).max(0.0));
        let mut week_top = 0.0;
        for (week_index, week_height) in self.week_heights.iter().enumerate() {
            if local_y < week_top + week_height {
                return add_days(self.grid_start, (week_index * 7 + column) as i64);
            }
            week_top += week_height;
        }
        add_days(
            self.grid_start,
            ((self.week_heights.len() - 1) * 7 + column) as i64,
        )
    }
}

impl<'a> CalendarLayout<'a> {
    pub(in crate::ui::board_workspace) fn build(
        calendar_view: &CalendarViewConfig,
        date_view: &'a NotionDateViewState,
        database_search: &crate::ui::surface::DatabaseSearchState,
    ) -> Self {
        let month_start = civil_date_month_start(date_view.visible_month);
        let grid_start = calendar_grid_start(month_start);
        let grid_end = add_days(grid_start, CALENDAR_DAY_COUNT as i64 - 1);
        let mut dated_items = date_view
            .calendar_items
            .items
            .iter()
            .filter(|item| database_search.calendar_item_matches_search(item))
            .filter_map(|item| {
                date_view.calendar_dated_item(item, calendar_view, grid_start, grid_end)
            })
            .collect::<Vec<_>>();
        if !calendar_view.has_explicit_sort {
            dated_items.sort_by_key(|dated_item| dated_item.start.ordinal());
        }
        let weeks = build_calendar_weeks(&dated_items, grid_start);
        Self { month_start, weeks }
    }

    pub(super) fn grid_geometry(&self) -> CalendarGridGeometry {
        let grid_start = self
            .weeks
            .first()
            .and_then(|week| week.days.first())
            .expect("Notion Calendar layout must retain six complete weeks")
            .date;
        CalendarGridGeometry {
            grid_start,
            week_heights: self.weeks.iter().map(|week| week.height).collect(),
        }
    }
}

impl<'a> CalendarRenderer<'a> {
    pub(super) fn current_calendar_layout(&self) -> CalendarLayout<'a> {
        CalendarLayout::build(self.calendar_view, self.date_view, self.database_search)
    }
}

impl NotionDateViewState {
    fn calendar_dated_item<'a>(
        &'a self,
        item: &'a BoardItem,
        calendar_view: &CalendarViewConfig,
        grid_start: CivilDate,
        grid_end: CivilDate,
    ) -> Option<CalendarDatedItem<'a>> {
        let date_value = calendar_date_range_for_item(item, calendar_view)?;
        let (start, end) = self
            .range_resize_preview
            .as_ref()
            .filter(|preview| preview.block_id == item.block_id)
            .map(|preview| (preview.inclusive_start, preview.inclusive_end))
            .unwrap_or_else(|| loaded_calendar_item_range(date_value));
        (start.ordinal() <= end.ordinal()
            && start.ordinal() <= grid_end.ordinal()
            && end.ordinal() >= grid_start.ordinal())
        .then_some(CalendarDatedItem { item, start, end })
    }
}

fn loaded_calendar_item_range(date_value: &BoardDateValue) -> (CivilDate, CivilDate) {
    let start = crate::ui::parse_civil_date(&date_value.start_date)
        .expect("loaded Notion Calendar dates must remain valid");
    let end = date_value
        .end_date
        .as_deref()
        .map(|end_date| {
            crate::ui::parse_civil_date(end_date)
                .expect("loaded Notion Calendar range ends must remain valid")
        })
        .unwrap_or(start);
    (start, end)
}

fn build_calendar_weeks<'a>(
    dated_items: &[CalendarDatedItem<'a>],
    grid_start: CivilDate,
) -> Vec<CalendarWeek<'a>> {
    let mut days = (0..CALENDAR_DAY_COUNT).map(|index| CalendarDay {
        date: add_days(grid_start, index as i64),
    });
    let weeks = (0..CALENDAR_WEEK_COUNT)
        .map(|week_index| {
            let days = days.by_ref().take(7).collect::<Vec<_>>();
            let week_start = add_days(grid_start, week_index as i64 * 7);
            let week_end = add_days(week_start, 6);
            let (segments, event_row_count) =
                build_calendar_week_segments(dated_items, week_start, week_end);
            CalendarWeek {
                height: calendar_week_height(event_row_count),
                days,
                segments,
            }
        })
        .collect();
    debug_assert!(days.next().is_none());
    weeks
}

impl crate::ui::surface::DatabaseSearchState {
    fn calendar_item_matches_search(&self, item: &BoardItem) -> bool {
        if !self.open {
            return true;
        }
        let query = self.query.trim().to_lowercase();
        query.is_empty() || item.title.to_lowercase().contains(&query)
    }
}
