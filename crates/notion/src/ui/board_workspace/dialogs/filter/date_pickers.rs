use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};

impl DatabaseFilterUiState {
    pub(super) fn render_database_filter_date_mode_picker(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let selected_mode = self
            .draft
            .as_ref()
            .expect("date mode picker requires a filter draft")
            .date_mode;
        resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(190.0, 64.0),
                DatabaseFilterDialogBehavior::new(
                    self.date_mode_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToEditor),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .children(
                [
                    DatabaseDateFilterMode::StartDate,
                    DatabaseDateFilterMode::EndDate,
                ]
                .into_iter()
                .map(|mode| {
                    self.render_database_filter_date_mode_row(resources, mode, selected_mode)
                }),
            )
            .into_any_element()
    }

    fn render_database_filter_date_mode_row(
        &self,
        resources: &DatabaseFilterRenderResources,
        mode: DatabaseDateFilterMode,
        selected_mode: DatabaseDateFilterMode,
    ) -> gpui::Stateful<Div> {
        let label = database_filter_date_mode_label(mode);
        let id = if mode == DatabaseDateFilterMode::StartDate {
            "start"
        } else {
            "end"
        };
        let action = DatabaseFilterAction::SetDateMode(mode);
        database_date_picker_row(
            resources.theme,
            format!("notion-database-filter-date-mode-{id}"),
            label,
        )
        .on_click(resources.filter_click_listener(action.clone()))
        .on_key_down(resources.filter_key_listener(action))
        .when(mode == selected_mode, |row| row.child("✓"))
    }

    pub(super) fn render_database_filter_date_value_picker(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let draft = self
            .draft
            .as_ref()
            .expect("date value picker requires a filter draft");
        let choices = database_date_value_choices(draft);
        let menu_height = choices.len() as f32 * 28.0 + 8.0;
        resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(240.0, menu_height),
                DatabaseFilterDialogBehavior::new(
                    self.date_value_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToEditor),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .children(
                choices
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, choice))| {
                        self.render_database_filter_date_value_row(resources, index, label, choice)
                    }),
            )
            .into_any_element()
    }

    fn render_database_filter_date_value_row(
        &self,
        resources: &DatabaseFilterRenderResources,
        index: usize,
        label: &'static str,
        choice: DatabaseDateValueChoice,
    ) -> gpui::Stateful<Div> {
        let selected = self
            .draft
            .as_ref()
            .is_some_and(|draft| database_date_value_choice_is_selected(draft, choice));
        let action = DatabaseFilterAction::SetDateValueChoice(choice);
        database_date_picker_row(
            resources.theme,
            format!("notion-database-filter-date-value-choice-{index}"),
            label,
        )
        .on_click(resources.filter_click_listener(action.clone()))
        .on_key_down(resources.filter_key_listener(action))
        .when(selected, |row| row.child("✓"))
    }
}

fn database_date_picker_row(theme: Theme, id: String, label: &'static str) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .role(Role::MenuItem)
        .aria_label(label)
        .focusable()
        .tab_stop(true)
        .cursor_pointer()
        .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
        .flex()
        .items_center()
        .text_size(px(14.0))
        .child(div().flex_grow(1.0).child(label))
}

fn database_date_value_choices(
    draft: &DatabaseFilterDraft,
) -> Vec<(&'static str, DatabaseDateValueChoice)> {
    if draft.operator == DatabaseTextFilterOperator::DateIsBetween {
        return database_date_range_value_choices();
    }
    database_date_point_value_choices()
}

fn database_date_range_value_choices() -> Vec<(&'static str, DatabaseDateValueChoice)> {
    vec![
        ("This week", DatabaseDateValueChoice::ThisWeek),
        (
            "The past week",
            DatabaseDateValueChoice::Past(DatabaseRelativeDateUnit::Week),
        ),
        (
            "The past month",
            DatabaseDateValueChoice::Past(DatabaseRelativeDateUnit::Month),
        ),
        (
            "The past year",
            DatabaseDateValueChoice::Past(DatabaseRelativeDateUnit::Year),
        ),
        (
            "The next week",
            DatabaseDateValueChoice::Future(DatabaseRelativeDateUnit::Week),
        ),
        (
            "The next month",
            DatabaseDateValueChoice::Future(DatabaseRelativeDateUnit::Month),
        ),
        (
            "The next year",
            DatabaseDateValueChoice::Future(DatabaseRelativeDateUnit::Year),
        ),
        ("Exact dates", DatabaseDateValueChoice::CustomRange),
    ]
}

fn database_date_point_value_choices() -> Vec<(&'static str, DatabaseDateValueChoice)> {
    vec![
        (
            "Today",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::Today),
        ),
        (
            "Tomorrow",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::Tomorrow),
        ),
        (
            "Yesterday",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::Yesterday),
        ),
        (
            "One week ago",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::OneWeekAgo),
        ),
        (
            "One week from now",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::OneWeekFromNow),
        ),
        (
            "One month ago",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::OneMonthAgo),
        ),
        (
            "One month from now",
            DatabaseDateValueChoice::Point(DatabaseRelativeDatePreset::OneMonthFromNow),
        ),
        ("Custom date", DatabaseDateValueChoice::CustomPoint),
    ]
}
