use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

impl DatabaseFilterUiState {
    pub(super) fn render_database_filter_more_actions(
        &self,
        resources: &DatabaseFilterRenderResources,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let advanced_rule = self.draft_advanced_path.is_some();
        resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(240.0, if advanced_rule { 37.0 } else { 65.0 }),
                DatabaseFilterDialogBehavior::new(
                    self.actions_anchor,
                    Some(DatabaseFilterOutsideAction::ReturnToEditor),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(resources.database_filter_action_row(
                "delete",
                "Delete filter",
                DatabaseFilterAction::RemoveActive(resources.host),
            ))
            .when(!advanced_rule, |menu| {
                menu.child(resources.database_filter_action_row(
                    "advanced",
                    "Add to advanced filter",
                    DatabaseFilterAction::PromoteActive,
                ))
            })
            .into_any_element()
    }
}

impl DatabaseFilterRenderResources {
    pub(super) fn database_filter_action_row(
        &self,
        id: &'static str,
        label: &'static str,
        action: DatabaseFilterAction,
    ) -> AnyElement {
        div()
            .id(format!("notion-database-filter-action-{id}"))
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::MenuItem)
            .aria_label(label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_click(self.filter_click_listener(action.clone()))
            .on_key_down(self.filter_key_listener(action))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_primary))
            .child(label)
            .into_any_element()
    }
}
