use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};
use gpui::ElementId;
use gpui_components::backdrop::{blocking_backdrop, ClickAwayBoundary};

impl DatabaseFilterUiState {
    pub(super) fn render_database_advanced_add_menu(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        editor_height: f32,
        cx: &mut App,
    ) -> AnyElement {
        let menu = resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(206.875, 82.0),
                DatabaseFilterDialogBehavior::new(
                    self.editor_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToAdvancedEditor),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .child(resources.database_filter_action_row(
                "add-advanced-rule",
                "Add filter rule",
                DatabaseFilterAction::AddAdvancedRule,
            ))
            .child(self.render_database_advanced_add_group_row(resources));
        if matches!(placement, DatabaseFilterDialogPlacement::FullPage) {
            return menu
                .top(px(168.0 + editor_height - 40.0))
                .right(px(377.0))
                .into_any_element();
        }
        menu.into_any_element()
    }

    fn render_database_advanced_add_group_row(
        &self,
        resources: &DatabaseFilterRenderResources,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::AddAdvancedGroup;
        div()
            .id("notion-database-filter-action-add-advanced-group")
            .h(px(45.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::MenuItem)
            .aria_label("Add filter group")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .flex_col()
            .justify_center()
            .child("Add filter group")
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(resources.theme.text_hint))
                    .child("A group to nest more filters"),
            )
    }

    pub(super) fn render_database_advanced_combiner_menu(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(190.1875, 99.0),
                DatabaseFilterDialogBehavior::new(
                    self.editor_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToAdvancedEditor),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .children(
                [
                    (DatabaseFilterGroupOperator::And, "All filters must match"),
                    (
                        DatabaseFilterGroupOperator::Or,
                        "At least one filter must match",
                    ),
                ]
                .into_iter()
                .map(|(operator, description)| {
                    self.render_database_advanced_combiner_row(resources, operator, description)
                }),
            )
            .into_any_element()
    }

    fn render_database_advanced_combiner_row(
        &self,
        resources: &DatabaseFilterRenderResources,
        operator: DatabaseFilterGroupOperator,
        description: &'static str,
    ) -> gpui::Stateful<Div> {
        let label = database_filter_group_operator_label(operator);
        let action = DatabaseFilterAction::SetAdvancedOperator(operator);
        div()
            .id(format!(
                "notion-database-advanced-combiner-{}",
                label.to_lowercase()
            ))
            .h(px(45.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::MenuItem)
            .aria_label(label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .flex_col()
            .justify_center()
            .child(label)
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(resources.theme.text_hint))
                    .child(description),
            )
    }

    pub(super) fn render_database_filter_operator_picker(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let draft = self
            .draft
            .as_ref()
            .expect("operator picker requires a filter draft");
        let operators = DatabaseTextFilterOperator::for_property(draft.property.filter_type());
        let operator_count = operators.len() as f32;
        resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(
                    FILTER_OPERATOR_WIDTH,
                    FILTER_OPERATOR_ROW_HEIGHT * operator_count
                        + FILTER_OPERATOR_ROW_GAP * (operator_count - 1.0)
                        + 8.0,
                ),
                DatabaseFilterDialogBehavior::new(
                    self.operator_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToEditorAndFocusValue),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap(px(FILTER_OPERATOR_ROW_GAP))
            .children(
                operators
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, operator)| {
                        self.render_database_filter_operator_row(resources, index, operator)
                    }),
            )
            .into_any_element()
    }

    fn render_database_filter_operator_row(
        &self,
        resources: &DatabaseFilterRenderResources,
        index: usize,
        operator: DatabaseTextFilterOperator,
    ) -> gpui::Stateful<Div> {
        let selected = self.operator_highlighted_index == index;
        let action = DatabaseFilterAction::SetTextOperator(operator);
        div()
            .id(format!("notion-database-filter-operator-{index}"))
            .h(px(FILTER_OPERATOR_ROW_HEIGHT))
            .flex_none()
            .rounded(px(6.0))
            .px(px(8.0))
            .role(Role::MenuItem)
            .aria_label(operator.label())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .when(selected, |row| {
                row.bg(alpha(resources.theme.text_primary, 0.08))
            })
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(resources.theme.text_primary))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .child(operator.label())
    }
}

impl DatabaseFilterRenderResources {
    #[track_caller]
    pub(super) fn database_filter_dialog_surface(
        &self,
        placement: DatabaseFilterDialogPlacement,
        size: DatabaseFilterDialogSize,
        behavior: DatabaseFilterDialogBehavior,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let DatabaseFilterDialogBehavior {
            preferred_anchor,
            outside_action,
        } = behavior;
        let DatabaseFilterDialogSize { width, height } = size;
        let surface = div()
            .absolute()
            .w(px(width))
            .h(px(height))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(database_filter_shadow());
        let surface =
            position_database_filter_surface(surface, placement, preferred_anchor, width, height);
        let surface = match outside_action {
            Some(action) => {
                let actions = self.actions.clone();
                let host = self.host;
                ClickAwayBoundary::new().dismissible_with_handler(
                    surface,
                    move |_, window, cx| {
                        actions.emit(DatabaseFilterAction::Outside { action, host }, window, cx);
                    },
                    cx,
                )
            }
            None => blocking_backdrop(surface, cx)
                .id(ElementId::CodeLocation(*core::panic::Location::caller())),
        };
        surface.role(Role::Dialog)
    }
}

fn position_database_filter_surface(
    surface: Div,
    placement: DatabaseFilterDialogPlacement,
    preferred_anchor: Option<Bounds<Pixels>>,
    width: f32,
    height: f32,
) -> Div {
    match placement {
        DatabaseFilterDialogPlacement::FullPage => match preferred_anchor {
            Some(anchor) => surface
                .left(px(anchor.left().as_f32()))
                .top(px(anchor.bottom().as_f32() + 4.0)),
            None => surface.top(px(168.0)).right(px(27.0)),
        },
        DatabaseFilterDialogPlacement::Inline {
            toolbar_anchor,
            viewport_width,
            viewport_height,
        } => {
            let anchor = preferred_anchor.unwrap_or(toolbar_anchor);
            let left = anchor.left().as_f32().clamp(
                FILTER_DIALOG_MARGIN,
                (viewport_width - width - FILTER_DIALOG_MARGIN).max(FILTER_DIALOG_MARGIN),
            );
            let below = anchor.bottom().as_f32() + 4.0;
            let top = if below + height <= viewport_height - FILTER_DIALOG_MARGIN {
                below
            } else {
                (anchor.top().as_f32() - height - 4.0).max(FILTER_DIALOG_MARGIN)
            };
            surface.left(px(left)).top(px(top))
        }
    }
}
