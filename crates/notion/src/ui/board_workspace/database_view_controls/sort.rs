use std::sync::Arc;

use gpui::{
    div, px, uniform_list, AnyElement, InteractiveElement, IntoElement, ListSizingBehavior,
    MouseButton, MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled,
};

use crate::model::{DatabaseViewControlMutationRequest, DatabaseViewSortDirection};
use crate::ui::surface::DatabaseViewControlPropertyRow;
use crate::ui::{alpha, rgb, FluentBuilder};

use super::renderer::DatabaseViewControlsRenderer;
use super::{
    DatabaseViewControlAction, DatabaseViewControlPlacement, DatabaseViewControlsResources,
    CONTROL_DIALOG_ROW_HEIGHT,
};

struct DatabaseSortRowAction {
    request: Option<DatabaseViewControlMutationRequest>,
    remove_request: Option<DatabaseViewControlMutationRequest>,
    detail: SharedString,
    glyph: &'static str,
}

struct DatabaseSortRowInteraction {
    id: String,
    request: Option<DatabaseViewControlMutationRequest>,
    resources: DatabaseViewControlsResources,
}

impl DatabaseViewControlsRenderer {
    pub(in crate::ui::board_workspace) fn render_database_sort_controls(
        &self,
        placement: DatabaseViewControlPlacement,
    ) -> AnyElement {
        let rows = self.snapshot.property_rows.clone();
        let order = self.database_sort_control_order(&rows);
        let row_count = order.len();
        let resources = self.resources(placement);
        let list = self.render_database_sort_control_list(rows, order, resources);
        self.dialog(placement, "Sort", row_count)
            .child(self.header("Sort", "Add, reverse, or remove"))
            .child(database_control_list_container(list))
            .into_any_element()
    }

    fn database_sort_control_order(&self, rows: &[DatabaseViewControlPropertyRow]) -> Arc<[usize]> {
        let mut order = self
            .snapshot
            .sort_order
            .iter()
            .filter_map(|property_id| rows.iter().position(|row| &row.property_id == property_id))
            .collect::<Vec<_>>();
        order.extend((0..rows.len()).filter(|index| rows[*index].sort_direction.is_none()));
        order.into()
    }

    fn render_database_sort_control_list(
        &self,
        rows: Arc<[DatabaseViewControlPropertyRow]>,
        order: Arc<[usize]>,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let renderer = self.clone();
        let row_count = order.len();
        uniform_list(
            match resources.host {
                super::DatabaseViewControlHost::Inline => "notion-inline-database-sort-rows",
                super::DatabaseViewControlHost::FullPage => "notion-database-sort-rows",
            },
            row_count,
            move |range, _window, _cx| {
                range
                    .filter_map(|visible_index| {
                        let index = *order.get(visible_index)?;
                        let row = rows.get(index)?;
                        Some(renderer.render_database_sort_control_row(row, resources))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .w_full()
        .h_full()
        .into_any_element()
    }

    fn render_database_sort_control_row(
        &self,
        row: &DatabaseViewControlPropertyRow,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        let action = self.database_sort_row_action(row);
        let interaction = DatabaseSortRowInteraction {
            id: format!("notion-database-sort-property-{}", row.property_id),
            request: action.request,
            resources,
        };
        self.database_sort_row_shell(interaction)
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(sort_direction_glyph(
                action.glyph,
                resources.theme.text_muted,
            ))
            .child(sort_property_label(
                row.label.clone(),
                resources.theme.text_primary,
            ))
            .child(sort_property_detail(
                action.detail,
                resources.theme.text_muted,
            ))
            .when_some(
                action.remove_request.filter(|_| self.snapshot.can_mutate),
                |control, request| {
                    control.child(self.render_database_sort_remove_button(row, request, resources))
                },
            )
            .into_any_element()
    }

    fn database_sort_row_action(
        &self,
        row: &DatabaseViewControlPropertyRow,
    ) -> DatabaseSortRowAction {
        let property_id = row.property_id.clone();
        let request =
            self.snapshot
                .active_view_id
                .clone()
                .and_then(|view_id| match row.sort_direction {
                    Some(_) => DatabaseViewControlMutationRequest::toggle_sort_direction(
                        view_id,
                        property_id.to_string(),
                    )
                    .ok(),
                    None => DatabaseViewControlMutationRequest::add_sort(
                        view_id,
                        property_id.to_string(),
                        DatabaseViewSortDirection::Ascending,
                    )
                    .ok(),
                });
        let remove_request = row.sort_direction.and_then(|_| {
            self.snapshot.active_view_id.clone().and_then(|view_id| {
                DatabaseViewControlMutationRequest::remove_sort(view_id, property_id.to_string())
                    .ok()
            })
        });
        let (detail, glyph) = match row.sort_direction {
            Some(DatabaseViewSortDirection::Ascending) => ("Ascending".into(), "↑"),
            Some(DatabaseViewSortDirection::Descending) => ("Descending".into(), "↓"),
            None => (row.property_type.clone(), "+"),
        };
        DatabaseSortRowAction {
            request,
            remove_request,
            detail,
            glyph,
        }
    }

    fn database_sort_row_shell(
        &self,
        interaction: DatabaseSortRowInteraction,
    ) -> gpui::Stateful<gpui::Div> {
        let DatabaseSortRowInteraction {
            id,
            request,
            resources,
        } = interaction;
        let enabled = self.snapshot.can_mutate && request.is_some();
        div()
            .id(id)
            .h(px(CONTROL_DIALOG_ROW_HEIGHT))
            .flex_none()
            .min_w(px(0.0))
            .rounded(px(6.0))
            .px(px(8.0))
            .when(enabled, |control| {
                let request = request.expect("sort request must exist when enabled");
                control
                    .cursor_pointer()
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
    }

    fn render_database_sort_remove_button(
        &self,
        row: &DatabaseViewControlPropertyRow,
        request: DatabaseViewControlMutationRequest,
        resources: DatabaseViewControlsResources,
    ) -> AnyElement {
        div()
            .id(format!("notion-database-remove-sort-{}", row.property_id))
            .size(px(24.0))
            .flex_none()
            .rounded(px(5.0))
            .role(gpui::Role::Button)
            .aria_label(format!("Remove {} sort", row.label))
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
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
            .child("×")
            .into_any_element()
    }
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

fn sort_direction_glyph(glyph: &'static str, color: u32) -> gpui::Div {
    div()
        .w(px(24.0))
        .flex_none()
        .text_size(px(12.0))
        .text_color(rgb(color))
        .child(glyph)
}

fn sort_property_label(label: SharedString, color: u32) -> gpui::Div {
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

fn sort_property_detail(detail: SharedString, color: u32) -> gpui::Div {
    div()
        .flex_none()
        .text_size(px(11.0))
        .text_color(rgb(color))
        .child(detail)
}
