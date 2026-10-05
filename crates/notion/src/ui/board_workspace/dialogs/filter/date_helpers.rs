use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

pub(super) fn database_date_point_label(point: &DatabaseDatePoint) -> String {
    match point {
        DatabaseDatePoint::Exact(date) => date.format("%b %-d").to_string(),
        DatabaseDatePoint::Relative(preset) => match preset {
            DatabaseRelativeDatePreset::Today => "Today",
            DatabaseRelativeDatePreset::Tomorrow => "Tomorrow",
            DatabaseRelativeDatePreset::Yesterday => "Yesterday",
            DatabaseRelativeDatePreset::OneWeekAgo => "One week ago",
            DatabaseRelativeDatePreset::OneWeekFromNow => "One week from now",
            DatabaseRelativeDatePreset::OneMonthAgo => "One month ago",
            DatabaseRelativeDatePreset::OneMonthFromNow => "One month from now",
        }
        .to_string(),
    }
}

pub(super) const fn database_filter_date_mode_label(mode: DatabaseDateFilterMode) -> &'static str {
    match mode {
        DatabaseDateFilterMode::StartDate => "Start date",
        DatabaseDateFilterMode::EndDate => "End date",
    }
}

pub(super) fn database_date_value_choice_is_selected(
    draft: &DatabaseFilterDraft,
    choice: DatabaseDateValueChoice,
) -> bool {
    match choice {
        DatabaseDateValueChoice::Point(choice) => matches!(
            &draft.date_point,
            DatabaseDatePoint::Relative(selected) if *selected == choice
        ),
        DatabaseDateValueChoice::CustomPoint => {
            matches!(&draft.date_point, DatabaseDatePoint::Exact(_))
        }
        DatabaseDateValueChoice::ThisWeek => matches!(
            &draft.date_range,
            DatabaseDateRange::Surrounding {
                unit: DatabaseRelativeDateUnit::Week
            }
        ),
        DatabaseDateValueChoice::Past(choice_unit) => matches!(
            &draft.date_range,
            DatabaseDateRange::Relative {
                direction: DatabaseRelativeDateDirection::Past,
                count: 1,
                unit,
            } if *unit == choice_unit
        ),
        DatabaseDateValueChoice::Future(choice_unit) => matches!(
            &draft.date_range,
            DatabaseDateRange::Relative {
                direction: DatabaseRelativeDateDirection::Future,
                count: 1,
                unit,
            } if *unit == choice_unit
        ),
        DatabaseDateValueChoice::CustomRange => {
            matches!(&draft.date_range, DatabaseDateRange::Exact { .. })
        }
    }
}

pub(super) fn database_date_range_label(range: &DatabaseDateRange) -> String {
    match range {
        DatabaseDateRange::Exact {
            start_date,
            end_date,
        } => match (start_date, end_date) {
            (Some(start), Some(end)) => {
                format!("{} – {}", start.format("%b %-d"), end.format("%b %-d"))
            }
            (Some(start), None) => start.format("%b %-d").to_string(),
            (None, Some(end)) => end.format("%b %-d").to_string(),
            (None, None) => "Date range".to_string(),
        },
        DatabaseDateRange::Relative {
            direction,
            count,
            unit,
        } => format!(
            "{} {count} {}",
            match direction {
                DatabaseRelativeDateDirection::Past => "Past",
                DatabaseRelativeDateDirection::Future => "Next",
            },
            relative_date_unit_label_plural(*unit, *count)
        ),
        DatabaseDateRange::Surrounding { unit } => {
            format!("This {}", relative_date_unit_label(*unit))
        }
    }
}

pub(super) const fn relative_date_unit_label(unit: DatabaseRelativeDateUnit) -> &'static str {
    match unit {
        DatabaseRelativeDateUnit::Day => "day",
        DatabaseRelativeDateUnit::Week => "week",
        DatabaseRelativeDateUnit::Month => "month",
        DatabaseRelativeDateUnit::Year => "year",
    }
}

pub(super) fn relative_date_unit_label_plural(
    unit: DatabaseRelativeDateUnit,
    count: u32,
) -> &'static str {
    if count == 1 {
        return relative_date_unit_label(unit);
    }
    match unit {
        DatabaseRelativeDateUnit::Day => "days",
        DatabaseRelativeDateUnit::Week => "weeks",
        DatabaseRelativeDateUnit::Month => "months",
        DatabaseRelativeDateUnit::Year => "years",
    }
}

pub(super) fn relative_date_range_unit(range: &DatabaseDateRange) -> DatabaseRelativeDateUnit {
    match range {
        DatabaseDateRange::Relative { unit, .. } | DatabaseDateRange::Surrounding { unit } => *unit,
        DatabaseDateRange::Exact { .. } => DatabaseRelativeDateUnit::Week,
    }
}
