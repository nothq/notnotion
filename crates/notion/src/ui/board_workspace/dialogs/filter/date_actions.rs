use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};
use crate::ui::surface::DatabaseFilterUiState;

fn apply_database_date_value_choice(
    draft: &mut DatabaseFilterDraft,
    choice: DatabaseDateValueChoice,
    today: NaiveDate,
) {
    match choice {
        DatabaseDateValueChoice::Point(preset) => {
            draft.date_point = DatabaseDatePoint::Relative(preset);
        }
        DatabaseDateValueChoice::CustomPoint => {
            if !matches!(draft.date_point, DatabaseDatePoint::Exact(_)) {
                draft.date_point = DatabaseDatePoint::Exact(today);
            }
        }
        DatabaseDateValueChoice::ThisWeek => {
            draft.date_range = DatabaseDateRange::Surrounding {
                unit: DatabaseRelativeDateUnit::Week,
            };
        }
        DatabaseDateValueChoice::Past(unit) | DatabaseDateValueChoice::Future(unit) => {
            let direction = match choice {
                DatabaseDateValueChoice::Past(_) => DatabaseRelativeDateDirection::Past,
                DatabaseDateValueChoice::Future(_) => DatabaseRelativeDateDirection::Future,
                _ => unreachable!("relative date choice retains its direction"),
            };
            draft.date_range = DatabaseDateRange::Relative {
                direction,
                count: 1,
                unit,
            };
        }
        DatabaseDateValueChoice::CustomRange => {
            if !matches!(draft.date_range, DatabaseDateRange::Exact { .. }) {
                draft.date_range = DatabaseDateRange::Exact {
                    start_date: Some(today),
                    end_date: None,
                };
            }
        }
    }
}

fn database_date_value_choice_visible_date(
    draft: &DatabaseFilterDraft,
    choice: DatabaseDateValueChoice,
    today: NaiveDate,
) -> NaiveDate {
    match choice {
        DatabaseDateValueChoice::CustomPoint => match &draft.date_point {
            DatabaseDatePoint::Exact(date) => *date,
            DatabaseDatePoint::Relative(_) => today,
        },
        DatabaseDateValueChoice::CustomRange => match &draft.date_range {
            DatabaseDateRange::Exact {
                start_date,
                end_date,
            } => (*start_date).or(*end_date).unwrap_or(today),
            DatabaseDateRange::Relative { .. } | DatabaseDateRange::Surrounding { .. } => today,
        },
        DatabaseDateValueChoice::Point(_)
        | DatabaseDateValueChoice::ThisWeek
        | DatabaseDateValueChoice::Past(_)
        | DatabaseDateValueChoice::Future(_) => today,
    }
}

impl DatabaseFilterUiState {
    pub(super) fn set_date(&mut self, date: NaiveDate) {
        let draft = self
            .draft
            .as_mut()
            .expect("selecting a filter date requires a draft");
        if draft.operator == DatabaseTextFilterOperator::DateIsBetween {
            draft.date_range = match &draft.date_range {
                DatabaseDateRange::Exact {
                    start_date: Some(start_date),
                    end_date: None,
                } if *start_date <= date => DatabaseDateRange::Exact {
                    start_date: Some(*start_date),
                    end_date: Some(date),
                },
                DatabaseDateRange::Exact {
                    start_date: Some(start_date),
                    end_date: None,
                } => DatabaseDateRange::Exact {
                    start_date: Some(date),
                    end_date: Some(*start_date),
                },
                _ => DatabaseDateRange::Exact {
                    start_date: Some(date),
                    end_date: None,
                },
            };
        } else {
            draft.date_point = DatabaseDatePoint::Exact(date);
        }
    }

    pub(super) fn set_date_mode(&mut self, mode: DatabaseDateFilterMode) {
        self.draft
            .as_mut()
            .expect("changing a date mode requires a filter draft")
            .date_mode = mode;
        self.stage = DatabaseFilterDialogStage::Editor;
    }

    pub(super) fn set_date_value_choice(&mut self, choice: DatabaseDateValueChoice) {
        let today = chrono::Local::now().date_naive();
        let draft = self
            .draft
            .as_mut()
            .expect("changing a date value requires a filter draft");
        apply_database_date_value_choice(draft, choice, today);
        let visible_date = database_date_value_choice_visible_date(draft, choice, today);
        self.visible_date_month = visible_date
            .with_day(1)
            .expect("the first day exists in every Gregorian month");
        self.stage = DatabaseFilterDialogStage::Editor;
    }

    pub(super) fn move_calendar_month(&mut self, direction: i32) {
        let month = self.visible_date_month;
        self.visible_date_month = if direction < 0 {
            (month - Duration::days(1))
                .with_day(1)
                .expect("the first day exists in every Gregorian month")
        } else {
            (month + Duration::days(32))
                .with_day(1)
                .expect("the first day exists in every Gregorian month")
        };
    }

    pub(super) fn cycle_relative_direction(&mut self) {
        let draft = self
            .draft
            .as_mut()
            .expect("changing a relative date requires a filter draft");
        let unit = relative_date_range_unit(&draft.date_range);
        draft.date_range = match draft.date_range {
            DatabaseDateRange::Surrounding { .. } | DatabaseDateRange::Exact { .. } => {
                DatabaseDateRange::Relative {
                    direction: DatabaseRelativeDateDirection::Past,
                    count: 1,
                    unit,
                }
            }
            DatabaseDateRange::Relative {
                direction: DatabaseRelativeDateDirection::Past,
                count,
                ..
            } => DatabaseDateRange::Relative {
                direction: DatabaseRelativeDateDirection::Future,
                count,
                unit,
            },
            DatabaseDateRange::Relative {
                direction: DatabaseRelativeDateDirection::Future,
                ..
            } => DatabaseDateRange::Surrounding { unit },
        };
    }

    pub(super) fn cycle_relative_unit(&mut self) {
        let draft = self
            .draft
            .as_mut()
            .expect("changing a relative date requires a filter draft");
        let next_unit = match relative_date_range_unit(&draft.date_range) {
            DatabaseRelativeDateUnit::Day => DatabaseRelativeDateUnit::Week,
            DatabaseRelativeDateUnit::Week => DatabaseRelativeDateUnit::Month,
            DatabaseRelativeDateUnit::Month => DatabaseRelativeDateUnit::Year,
            DatabaseRelativeDateUnit::Year => DatabaseRelativeDateUnit::Day,
        };
        draft.date_range = match draft.date_range {
            DatabaseDateRange::Relative {
                direction, count, ..
            } => DatabaseDateRange::Relative {
                direction,
                count,
                unit: next_unit,
            },
            DatabaseDateRange::Surrounding { .. } | DatabaseDateRange::Exact { .. } => {
                DatabaseDateRange::Surrounding { unit: next_unit }
            }
        };
    }

    pub(super) fn adjust_relative_count(&mut self, direction: i32) {
        let draft = self
            .draft
            .as_mut()
            .expect("changing a relative date requires a filter draft");
        let DatabaseDateRange::Relative {
            direction: relative_direction,
            count,
            unit,
        } = draft.date_range
        else {
            return;
        };
        let count = if direction < 0 {
            count.saturating_sub(1).max(1)
        } else {
            count.saturating_add(1)
        };
        draft.date_range = DatabaseDateRange::Relative {
            direction: relative_direction,
            count,
            unit,
        };
    }
}
