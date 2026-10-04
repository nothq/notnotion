use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};

impl DatabaseFilterRenderer<'_> {
    pub(super) fn render_database_advanced_filter_row(
        &self,
        database_properties: &[DatabaseProperty],
        row: DatabaseAdvancedFilterRow,
    ) -> AnyElement {
        match row.node {
            DatabaseFilterNode::Property(filter) => {
                self.render_database_advanced_property_row(database_properties, row.path, filter)
            }
            DatabaseFilterNode::Group(group) => {
                self.render_database_advanced_group_row(row.path, group)
            }
        }
    }

    fn render_database_advanced_property_row(
        &self,
        database_properties: &[DatabaseProperty],
        path: Vec<usize>,
        filter: DatabasePropertyFilter,
    ) -> AnyElement {
        let depth = path.len() - 1;
        let property_label = database_properties
            .iter()
            .find(|property| property.property_id == filter.property_id().as_str())
            .map_or("Property", |property| property.label.as_str())
            .to_string();
        let condition_label = database_filter_chip_label(&property_label, filter.condition());
        let operator = DatabaseTextFilterOperator::from_condition(filter.condition());
        let rule = div()
            .id(format!(
                "notion-database-advanced-rule-{}",
                advanced_filter_path_id(&path)
            ))
            .h(px(32.0))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(self.resources.theme.text_primary, 0.10))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .child(self.render_database_advanced_property_button(&path, property_label))
            .child(self.render_database_advanced_condition_button(
                &path,
                operator,
                condition_label,
            ));
        div()
            .h(px(40.0))
            .pl(px(depth as f32 * 77.0))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(rule)
            .child(self.render_database_advanced_delete_button(path, "rule", "Delete filter rule"))
            .into_any_element()
    }

    fn render_database_advanced_group_row(
        &self,
        path: Vec<usize>,
        group: DatabaseFilterGroup,
    ) -> AnyElement {
        let depth = path.len() - 1;
        div()
            .h(px(40.0))
            .pl(px(depth as f32 * 77.0))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.render_database_advanced_group_toggle(&path, &group))
            .child(div().flex_grow(1.0))
            .child(self.render_database_advanced_group_add(&path))
            .child(self.render_database_advanced_delete_button(
                path,
                "group",
                "Delete filter group",
            ))
            .into_any_element()
    }

    fn render_database_advanced_property_button(
        &self,
        path: &[usize],
        property_label: String,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::OpenAdvancedPropertyPicker(path.to_vec());
        div()
            .id(format!(
                "notion-database-advanced-rule-property-{}",
                advanced_filter_path_id(path)
            ))
            .w(px(137.0))
            .h_full()
            .px(px(10.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(format!("Change property {property_label}"))
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .truncate()
            .child(property_label)
    }

    fn render_database_advanced_condition_button(
        &self,
        path: &[usize],
        operator: DatabaseTextFilterOperator,
        condition_label: String,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::OpenAdvancedRule(path.to_vec());
        div()
            .id(format!(
                "notion-database-advanced-rule-condition-{}",
                advanced_filter_path_id(path)
            ))
            .h_full()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .px(px(10.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(condition_label.clone())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .w(px(85.0))
                    .truncate()
                    .text_color(rgb(self.resources.theme.text_secondary))
                    .child(operator.header_label()),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .truncate()
                    .child(condition_label),
            )
    }

    fn render_database_advanced_group_toggle(
        &self,
        path: &[usize],
        group: &DatabaseFilterGroup,
    ) -> gpui::Stateful<Div> {
        let operator_label = database_filter_group_operator_label(group.operator());
        let action = DatabaseFilterAction::ToggleAdvancedGroup(path.to_vec());
        div()
            .id(format!(
                "notion-database-advanced-group-{}",
                advanced_filter_path_id(path)
            ))
            .h(px(32.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(self.resources.theme.text_primary, 0.10))
            .role(Role::Button)
            .aria_label(operator_label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .child(format!(
                "{operator_label} group · {} rules",
                database_filter_rule_count(group)
            ))
    }

    fn render_database_advanced_group_add(&self, path: &[usize]) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::SetAdvancedAddPath(path.to_vec());
        div()
            .id(format!(
                "notion-database-advanced-group-add-{}",
                advanced_filter_path_id(path)
            ))
            .size(px(32.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Add nested filter")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .justify_center()
            .child("+")
    }

    fn render_database_advanced_delete_button(
        &self,
        path: Vec<usize>,
        kind: &'static str,
        label: &'static str,
    ) -> gpui::Stateful<Div> {
        let id_path = advanced_filter_path_id(&path);
        let action = DatabaseFilterAction::RemoveAdvancedNode {
            path,
            host: self.resources.host,
        };
        div()
            .id(format!("notion-database-advanced-{kind}-delete-{id_path}"))
            .size(px(32.0))
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
            .justify_center()
            .child("•••")
    }
}
