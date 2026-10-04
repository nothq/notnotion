use std::sync::Arc;

use gpui::{
    div, px, uniform_list, AnyElement, InteractiveElement, IntoElement, ListSizingBehavior,
    MouseButton, MouseDownEvent, ParentElement, SharedString, Styled,
};

use crate::model::{DatabaseViewControlMutationRequest, DatabaseViewGroupState, ViewTabKind};
use crate::ui::surface::DatabaseViewControlPropertyRow;
use crate::ui::{alpha, rgb, FluentBuilder};

use super::renderer::DatabaseViewControlsRenderer;
use super::{
    DatabaseViewControlAction, DatabaseViewControlPlacement, DatabaseViewControlsResources,
    CONTROL_DIALOG_ROW_HEIGHT,
};

#[derive(Clone)]
enum PropertiesControlRow {
    Section(SharedString),
    NoGroup,
    UnsupportedGroup,
    GroupProperty(usize),
    PropertyVisibility(usize),
}

struct DatabaseControlChoiceRow {
    id: String,
    label: SharedString,
    detail: SharedString,
    selected: bool,
    request: Option<DatabaseViewControlMutationRequest>,
}

impl DatabaseViewControlsRenderer {
    pub(in crate::ui::board_workspace) fn render_database_properties_controls(
        &self,
        placement: DatabaseViewControlPlacement,
    ) -> AnyElement {
        let properties = self.snapshot.property_rows.clone();
        let rows = properties_control_rows(
            self.snapshot.active_view_kind,
            &self.snapshot.active_view_group,
            &properties,
        );
        let row_count = rows.len();
        let resources = self.resources(placement);
        let list = self.render_database_properties_control_list(properties, rows, resources);
        self.dialog(placement, "Properties", row_count)
            .child(self.header("View settings", "Live view"))
            .child(database_control_list_container(list))
            .into_any_element()
    }

    fn render_database_properties_control_list(
        &self,
        properties: Arc<[DatabaseViewControlPropertyRow]>,
        rows: Arc<[PropertiesControlRow]>,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let renderer = self.clone();
        let row_count = rows.len();
        uniform_list(
            match resources.host {
                super::DatabaseViewControlHost::Inline => "notion-inline-database-properties-rows",
                super::DatabaseViewControlHost::FullPage => "notion-database-properties-rows",
            },
            row_count,
            move |range, _window, _cx| {
                range
                    .filter_map(|index| {
                        rows.get(index).map(|row| {
                            renderer.render_database_properties_control_row(
                                row,
                                &properties,
                                resources,
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .w_full()
        .h_full()
        .into_any_element()
    }

    fn render_database_properties_control_row(
        &self,
        control_row: &PropertiesControlRow,
        properties: &[DatabaseViewControlPropertyRow],
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        match control_row {
            PropertiesControlRow::Section(label) => {
                database_control_section(label.clone(), resources.theme.text_muted)
            }
            PropertiesControlRow::NoGroup => self.render_database_group_none_row(resources),
            PropertiesControlRow::UnsupportedGroup => {
                unsupported_group_row(resources.theme.text_muted)
            }
            PropertiesControlRow::GroupProperty(index) => properties
                .get(*index)
                .map(|property| self.render_database_group_property_row(property, resources))
                .unwrap_or_else(|| div().into_any_element()),
            PropertiesControlRow::PropertyVisibility(index) => properties
                .get(*index)
                .map(|property| self.render_database_visibility_row(property, resources))
                .unwrap_or_else(|| div().into_any_element()),
        }
    }

    fn render_database_group_none_row(
        &self,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let selected = matches!(
            &self.snapshot.active_view_group,
            DatabaseViewGroupState::None
        );
        let request = self
            .snapshot
            .active_view_id
            .clone()
            .map(DatabaseViewControlMutationRequest::clear_group);
        self.database_control_choice_row(
            DatabaseControlChoiceRow {
                id: "notion-database-group-none".to_string(),
                label: "No grouping".into(),
                detail: "None".into(),
                selected,
                request: request.filter(|_| !selected),
            },
            resources,
        )
    }

    fn render_database_group_property_row(
        &self,
        property: &DatabaseViewControlPropertyRow,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let request = self.snapshot.active_view_id.clone().and_then(|view_id| {
            DatabaseViewControlMutationRequest::set_group(view_id, property.property_id.to_string())
                .ok()
        });
        self.database_control_choice_row(
            DatabaseControlChoiceRow {
                id: format!("notion-database-group-property-{}", property.property_id),
                label: property.label.clone(),
                detail: property.property_type.clone(),
                selected: property.grouped,
                request: request.filter(|_| !property.grouped),
            },
            resources,
        )
    }

    fn render_database_visibility_row(
        &self,
        property: &DatabaseViewControlPropertyRow,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let visible = property
            .visibility
            .expect("visibility rows require a typed layout entry");
        let request = self.snapshot.active_view_id.clone().and_then(|view_id| {
            DatabaseViewControlMutationRequest::set_property_visibility(
                view_id,
                property.property_id.to_string(),
                !visible,
            )
            .ok()
        });
        self.database_control_choice_row(
            DatabaseControlChoiceRow {
                id: format!(
                    "notion-database-property-visibility-{}",
                    property.property_id
                ),
                label: property.label.clone(),
                detail: property.property_type.clone(),
                selected: visible,
                request,
            },
            resources,
        )
    }

    fn database_control_choice_row(
        &self,
        choice: DatabaseControlChoiceRow,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let enabled = self.snapshot.can_mutate && choice.request.is_some();
        div()
            .id(choice.id)
            .h(px(CONTROL_DIALOG_ROW_HEIGHT))
            .flex_none()
            .min_w(px(0.0))
            .rounded(px(6.0))
            .px(px(8.0))
            .when(enabled, |row| {
                let request = choice
                    .request
                    .expect("enabled database choice requires a request");
                row.cursor_pointer()
                    .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            DatabaseViewControlAction::Submit {
                                request: request.clone(),
                                host: resources.host,
                            }
                        }),
                    )
            })
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(choice_indicator(
                choice.selected,
                resources.theme.text_muted,
            ))
            .child(choice_label(choice.label, resources.theme.text_primary))
            .child(choice_detail(choice.detail, resources.theme.text_muted))
            .into_any_element()
    }
}

fn properties_control_rows(
    view_kind: ViewTabKind,
    group: &DatabaseViewGroupState,
    properties: &[DatabaseViewControlPropertyRow],
) -> Arc<[PropertiesControlRow]> {
    let mut rows = Vec::new();
    if matches!(
        view_kind,
        ViewTabKind::Table | ViewTabKind::List | ViewTabKind::Gallery
    ) {
        rows.push(PropertiesControlRow::Section("Group".into()));
        match group {
            DatabaseViewGroupState::Unsupported { .. } => {
                rows.push(PropertiesControlRow::UnsupportedGroup);
            }
            DatabaseViewGroupState::None | DatabaseViewGroupState::Supported { .. } => {
                rows.push(PropertiesControlRow::NoGroup);
                rows.extend(
                    properties
                        .iter()
                        .enumerate()
                        .filter_map(|(index, property)| {
                            property
                                .group_kind
                                .is_some()
                                .then_some(PropertiesControlRow::GroupProperty(index))
                        }),
                );
            }
        }
    }
    rows.push(PropertiesControlRow::Section("Property visibility".into()));
    rows.extend(
        properties
            .iter()
            .enumerate()
            .filter_map(|(index, property)| {
                property
                    .visibility
                    .is_some()
                    .then_some(PropertiesControlRow::PropertyVisibility(index))
            }),
    );
    rows.into()
}

fn database_control_list_container(list: AnyElement) -> gpui::Div {
    div()
        .flex_grow(1.0)
        .min_h(px(0.0))
        .overflow_hidden()
        .px(px(4.0))
        .pb(px(6.0))
        .child(list)
}

fn database_control_section(label: SharedString, color: u32) -> AnyElement {
    div()
        .h(px(CONTROL_DIALOG_ROW_HEIGHT))
        .px(px(8.0))
        .flex()
        .items_end()
        .pb(px(5.0))
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .child(label)
        .into_any_element()
}

fn unsupported_group_row(color: u32) -> AnyElement {
    div()
        .id("notion-database-unsupported-group")
        .h(px(CONTROL_DIALOG_ROW_HEIGHT))
        .px(px(8.0))
        .flex()
        .items_center()
        .text_size(px(13.0))
        .text_color(rgb(color))
        .child("Current grouping is not editable in notnotion")
        .into_any_element()
}

fn choice_indicator(selected: bool, color: u32) -> gpui::Div {
    div()
        .w(px(20.0))
        .flex_none()
        .text_size(px(12.0))
        .text_color(rgb(color))
        .child(if selected { "✓" } else { "" })
}

fn choice_label(label: SharedString, color: u32) -> gpui::Div {
    div()
        .flex_grow(1.0)
        .min_w(px(0.0))
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(13.0))
        .text_color(rgb(color))
        .child(label)
}

fn choice_detail(detail: SharedString, color: u32) -> gpui::Div {
    div()
        .flex_none()
        .text_size(px(11.0))
        .text_color(rgb(color))
        .child(detail)
}
