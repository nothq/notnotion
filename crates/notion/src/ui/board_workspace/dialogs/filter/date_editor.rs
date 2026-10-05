use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};

impl DatabaseFilterUiState {
    pub(super) fn render_database_date_filter_editor(
        &self,
        resources: &DatabaseFilterRenderResources,
        draft: &DatabaseFilterDraft,
    ) -> AnyElement {
        if draft.operator == DatabaseTextFilterOperator::DateIsRelativeToToday {
            return self.render_database_relative_date_editor(resources, draft);
        }
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(self.render_database_filter_date_value_button(resources, draft))
            .child(self.render_database_filter_calendar(resources, draft))
            .into_any_element()
    }

    fn render_database_relative_date_editor(
        &self,
        resources: &DatabaseFilterRenderResources,
        draft: &DatabaseFilterDraft,
    ) -> AnyElement {
        let (direction, count, unit) = database_relative_date_editor_state(&draft.date_range);
        let row = div()
            .h(px(32.0))
            .flex_none()
            .flex()
            .gap(px(8.0))
            .child(self.render_database_relative_date_direction(resources, direction))
            .when_some(count, |row, count| {
                row.child(self.render_database_relative_date_count(resources, count))
            })
            .child(
                self.render_database_relative_date_unit(resources, relative_date_unit_label(unit)),
            );
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(row)
            .child(self.render_database_filter_calendar(resources, draft))
            .child(database_relative_date_footer(resources.theme))
            .into_any_element()
    }

    fn render_database_relative_date_direction(
        &self,
        resources: &DatabaseFilterRenderResources,
        direction: &'static str,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::CycleRelativeDirection;
        database_relative_date_button(resources.theme, "direction", direction)
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
    }

    fn render_database_relative_date_unit(
        &self,
        resources: &DatabaseFilterRenderResources,
        unit: &'static str,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::CycleRelativeUnit;
        database_relative_date_button(resources.theme, "unit", unit)
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
    }

    pub(super) fn render_database_filter_date_value_button(
        &self,
        resources: &DatabaseFilterRenderResources,
        draft: &DatabaseFilterDraft,
    ) -> AnyElement {
        let label = if draft.operator == DatabaseTextFilterOperator::DateIsBetween {
            database_date_range_label(&draft.date_range)
        } else {
            database_date_point_label(&draft.date_point)
        };
        let current_anchor = self.date_value_anchor;
        let actions = resources.actions.clone();
        div()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first().copied() else {
                    return;
                };
                if current_anchor == Some(bounds) {
                    return;
                }
                actions.emit(
                    DatabaseFilterAction::MeasureAnchor {
                        kind: DatabaseFilterAnchorKind::DateValue,
                        bounds,
                    },
                    window,
                    cx,
                );
                window.refresh();
            })
            .h(px(32.0))
            .flex_none()
            .child(self.render_database_filter_date_value_control(resources, label))
            .into_any_element()
    }

    fn render_database_filter_date_value_control(
        &self,
        resources: &DatabaseFilterRenderResources,
        label: String,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::SetStage(DatabaseFilterDialogStage::DateValuePicker);
        div()
            .id("notion-database-filter-date-value")
            .h(px(28.0))
            .w_full()
            .px(px(10.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(resources.theme.text_primary, 0.12))
            .role(Role::Button)
            .aria_label(label.clone())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .text_color(rgb(resources.theme.text_primary))
            .child(div().min_w(px(0.0)).flex_grow(1.0).truncate().child(label))
            .child("⌄")
    }

    pub(super) fn render_database_relative_date_count(
        &self,
        resources: &DatabaseFilterRenderResources,
        count: u32,
    ) -> AnyElement {
        let decrement = self.render_database_relative_date_count_button(
            resources,
            RelativeDateCountButton {
                id: "decrement",
                label: "Decrease relative date count",
                glyph: "−",
                direction: -1,
            },
        );
        let increment = self.render_database_relative_date_count_button(
            resources,
            RelativeDateCountButton {
                id: "increment",
                label: "Increase relative date count",
                glyph: "+",
                direction: 1,
            },
        );
        div()
            .h(px(28.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(resources.theme.text_primary, 0.12))
            .flex()
            .items_center()
            .child(decrement)
            .child(
                div()
                    .w(px(24.0))
                    .text_align(gpui::TextAlign::Center)
                    .text_size(px(13.0))
                    .child(count.to_string()),
            )
            .child(increment)
            .into_any_element()
    }

    fn render_database_relative_date_count_button(
        &self,
        resources: &DatabaseFilterRenderResources,
        button: RelativeDateCountButton,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::AdjustRelativeCount(button.direction);
        div()
            .id(format!("notion-database-filter-date-count-{}", button.id))
            .size(px(26.0))
            .rounded(px(5.0))
            .role(Role::Button)
            .aria_label(button.label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .items_center()
            .justify_center()
            .child(button.glyph)
    }
}

#[derive(Clone, Copy)]
struct RelativeDateCountButton {
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    direction: i32,
}

fn database_relative_date_editor_state(
    range: &DatabaseDateRange,
) -> (&'static str, Option<u32>, DatabaseRelativeDateUnit) {
    match range {
        DatabaseDateRange::Surrounding { unit } => ("This", None, *unit),
        DatabaseDateRange::Relative {
            direction,
            count,
            unit,
        } => (
            match direction {
                DatabaseRelativeDateDirection::Past => "Past",
                DatabaseRelativeDateDirection::Future => "Next",
            },
            Some(*count),
            *unit,
        ),
        DatabaseDateRange::Exact { .. } => ("This", None, DatabaseRelativeDateUnit::Week),
    }
}

fn database_relative_date_button(
    theme: Theme,
    id: &'static str,
    label: &'static str,
) -> gpui::Stateful<Div> {
    div()
        .id(format!("notion-database-filter-date-{id}"))
        .h(px(28.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(alpha(theme.text_primary, 0.12))
        .role(Role::Button)
        .aria_label(label)
        .focusable()
        .tab_stop(true)
        .cursor_pointer()
        .flex()
        .items_center()
        .child(label)
}

fn database_relative_date_footer(theme: Theme) -> Div {
    div()
        .h(px(28.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(rgb(theme.text_hint))
        .child("Filter will update with the current date")
}
