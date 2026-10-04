use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};
use crate::ui::surface::DatabaseFilterUiState;

impl DatabaseFilterUiState {
    pub(super) fn render_database_picker_filter_header(
        &self,
        resources: &DatabaseFilterRenderResources,
        draft: &DatabaseFilterDraft,
    ) -> AnyElement {
        let is_date = draft.property.filter_type() == "date";
        div()
            .h(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .child(database_picker_property_label(
                resources.theme,
                draft,
                is_date,
            ))
            .when(is_date, |header| {
                header.child(self.render_database_picker_date_mode(resources, draft.date_mode))
            })
            .child(self.render_database_picker_operator(resources, draft.operator))
            .child(div().flex_grow(1.0))
            .when(draft.present_in_effective_state, |header| {
                header.child(self.render_database_filter_more_actions_button(resources))
            })
            .into_any_element()
    }

    fn render_database_picker_date_mode(
        &self,
        resources: &DatabaseFilterRenderResources,
        mode: DatabaseDateFilterMode,
    ) -> Div {
        let label = database_filter_date_mode_label(mode);
        let action = DatabaseFilterAction::SetStage(DatabaseFilterDialogStage::DateModePicker);
        let button = database_picker_header_button(resources.theme, "date-mode", label, label)
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action));
        database_filter_anchor(
            self.date_mode_anchor,
            resources,
            DatabaseFilterAnchorKind::DateMode,
            button,
        )
    }

    fn render_database_picker_operator(
        &self,
        resources: &DatabaseFilterRenderResources,
        operator: DatabaseTextFilterOperator,
    ) -> Div {
        let action = DatabaseFilterAction::OpenOperatorPicker;
        let button = database_picker_header_button(
            resources.theme,
            "operator",
            operator.label(),
            operator.header_label(),
        )
        .on_click(resources.filter_click_listener(action.clone()))
        .on_key_down(resources.filter_key_listener(action));
        database_filter_anchor(
            self.operator_anchor,
            resources,
            DatabaseFilterAnchorKind::Operator,
            button,
        )
    }

    pub(super) fn render_database_filter_more_actions_button(
        &self,
        resources: &DatabaseFilterRenderResources,
    ) -> AnyElement {
        let action = DatabaseFilterAction::SetStage(DatabaseFilterDialogStage::MoreActions);
        let button = database_more_actions_button(resources.theme)
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action));
        database_filter_anchor(
            self.actions_anchor,
            resources,
            DatabaseFilterAnchorKind::Actions,
            button,
        )
        .into_any_element()
    }

    pub(super) fn picker_editor_height(&self, draft: &DatabaseFilterDraft) -> f32 {
        match draft.property.filter_type() {
            "person" | "relation" => FILTER_PICKER_EDITOR_MAX_HEIGHT,
            "date" => 366.5,
            "select" | "multi_select" => {
                80.0 + draft.property.options.len().max(1) as f32 * FILTER_PROPERTY_ROW_HEIGHT
            }
            "status" => {
                let rows = draft.property.status_groups.len() + draft.property.options.len();
                68.0 + rows.max(1) as f32 * FILTER_PROPERTY_ROW_HEIGHT
            }
            "checkbox" => 68.0 + 2.0 * FILTER_PROPERTY_ROW_HEIGHT,
            _ => FILTER_EDITOR_HEIGHT,
        }
    }
}

fn database_picker_property_label(theme: Theme, draft: &DatabaseFilterDraft, is_date: bool) -> Div {
    div()
        .h_full()
        .max_w(px(if is_date { 74.0 } else { 116.0 }))
        .pl(px(4.0))
        .pr(px(2.0))
        .rounded(px(4.0))
        .flex()
        .items_center()
        .text_size(px(12.0))
        .text_color(rgb(theme.text_secondary))
        .truncate()
        .child(draft.property.label.clone())
}

fn database_picker_header_button(
    theme: Theme,
    id: &'static str,
    aria_label: &'static str,
    label: &'static str,
) -> gpui::Stateful<Div> {
    div()
        .id(format!("notion-database-filter-{id}"))
        .h_full()
        .px(px(2.0))
        .rounded(px(4.0))
        .role(Role::Button)
        .aria_label(aria_label)
        .focusable()
        .tab_stop(true)
        .cursor_pointer()
        .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
        .flex()
        .items_center()
        .gap(px(3.0))
        .text_size(px(12.0))
        .text_color(rgb(theme.text_secondary))
        .child(label)
        .child("⌄")
}

fn database_more_actions_button(theme: Theme) -> gpui::Stateful<Div> {
    div()
        .id("notion-database-filter-more-actions")
        .size(px(20.0))
        .rounded(px(4.0))
        .role(Role::Button)
        .aria_label("More actions")
        .focusable()
        .tab_stop(true)
        .cursor_pointer()
        .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(rgb(theme.text_secondary))
        .child("•••")
}

fn database_filter_anchor(
    current: Option<Bounds<Pixels>>,
    resources: &DatabaseFilterRenderResources,
    kind: DatabaseFilterAnchorKind,
    child: gpui::Stateful<Div>,
) -> Div {
    let actions = resources.actions.clone();
    div()
        .on_children_prepainted(move |bounds, window, cx| {
            let Some(bounds) = bounds.first().copied() else {
                return;
            };
            if current == Some(bounds) {
                return;
            }
            actions.emit(
                DatabaseFilterAction::MeasureAnchor { kind, bounds },
                window,
                cx,
            );
            window.refresh();
        })
        .h(px(20.0))
        .flex_none()
        .child(child)
}
