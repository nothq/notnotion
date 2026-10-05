use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};

impl DatabaseFilterRenderer<'_> {
    pub(super) fn render_database_advanced_filter_editor(
        &self,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let group = match self.state.effective_state(self.board).advanced() {
            DatabaseAdvancedFilterState::Editable(group) => group.clone(),
            DatabaseAdvancedFilterState::None | DatabaseAdvancedFilterState::Unsupported => {
                return div().into_any_element();
            }
        };
        let rows = database_advanced_filter_rows(&group);
        let nested = rows.iter().any(|row| row.path.len() > 1);
        let width = if nested { 682.71875 } else { 556.71875 };
        let height = 77.0 + rows.len().max(1) as f32 * 40.0;
        let dialog = self
            .resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(width, height),
                DatabaseFilterDialogBehavior::new(
                    self.state.editor_anchor,
                    (matches!(placement, DatabaseFilterDialogPlacement::FullPage)
                        && self.state.stage == DatabaseFilterDialogStage::AdvancedEditor)
                        .then_some(DatabaseFilterOutsideAction::CloseToolbar),
                ),
                cx,
            )
            .p(px(4.0))
            .flex()
            .flex_col()
            .child(self.render_database_advanced_filter_header(group.operator(), rows.len() > 1))
            .children(rows.into_iter().map(|row| {
                self.render_database_advanced_filter_row(&self.board.database_properties, row)
            }))
            .child(self.render_database_advanced_add_rule_button())
            .child(self.render_database_advanced_delete_all_button());
        self.render_database_advanced_filter_layers(dialog, placement, height, cx)
    }

    fn render_database_advanced_filter_layers(
        &self,
        dialog: gpui::Stateful<Div>,
        placement: DatabaseFilterDialogPlacement,
        height: f32,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .absolute()
            .inset_0()
            .child(dialog)
            .when(
                self.state.stage == DatabaseFilterDialogStage::AdvancedAddMenu,
                |root| {
                    root.child(self.state.render_database_advanced_add_menu(
                        &self.resources,
                        placement,
                        height,
                        cx,
                    ))
                },
            )
            .when(
                self.state.stage == DatabaseFilterDialogStage::AdvancedCombinerPicker,
                |root| {
                    root.child(self.state.render_database_advanced_combiner_menu(
                        &self.resources,
                        placement,
                        cx,
                    ))
                },
            )
            .into_any_element()
    }

    fn render_database_advanced_filter_header(
        &self,
        combiner: DatabaseFilterGroupOperator,
        show_combiner: bool,
    ) -> Div {
        div()
            .h(px(32.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .child("Where")
            .when(show_combiner, |header| {
                header.child(self.render_database_advanced_combiner_button(combiner))
            })
    }

    fn render_database_advanced_combiner_button(
        &self,
        combiner: DatabaseFilterGroupOperator,
    ) -> gpui::Stateful<Div> {
        let label = database_filter_group_operator_label(combiner);
        let action =
            DatabaseFilterAction::SetStage(DatabaseFilterDialogStage::AdvancedCombinerPicker);
        div()
            .id("notion-database-advanced-combiner")
            .ml(px(8.0))
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .child(label)
            .child("⌄")
    }

    fn render_database_advanced_add_rule_button(&self) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::SetAdvancedAddPath(Vec::new());
        div()
            .id("notion-database-advanced-add-rule")
            .h(px(32.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Add filter rule")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .child("+  Add filter rule")
    }

    fn render_database_advanced_delete_all_button(&self) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::ClearAdvanced(self.resources.host);
        div()
            .id("notion-database-advanced-delete")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Delete filter")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .child("Delete filter")
    }
}
