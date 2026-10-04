use std::sync::Arc;

use gpui::SharedString;

use crate::model::{
    BoardSnapshot, DatabaseViewGroupKind, DatabaseViewSortDirection, PageShellIcon,
};

#[derive(Clone)]
pub(crate) struct DatabaseViewControlPropertyRow {
    pub(crate) property_id: SharedString,
    pub(crate) label: SharedString,
    pub(crate) property_type: SharedString,
    pub(crate) visibility: Option<bool>,
    pub(crate) sort_direction: Option<DatabaseViewSortDirection>,
    pub(crate) group_kind: Option<DatabaseViewGroupKind>,
    pub(crate) grouped: bool,
}

#[derive(Clone)]
pub(crate) struct DatabaseViewPropertyRow {
    pub(crate) label: SharedString,
    pub(crate) property_type: SharedString,
    pub(crate) value: SharedString,
}

#[derive(Clone)]
pub(crate) struct DatabaseViewRow {
    pub(crate) block_id: SharedString,
    pub(crate) title: SharedString,
    pub(crate) icon: Option<PageShellIcon>,
    pub(crate) properties: Arc<[DatabaseViewPropertyRow]>,
}

pub(crate) fn database_view_rows(board: &BoardSnapshot) -> Arc<[DatabaseViewRow]> {
    board
        .items
        .iter()
        .map(|item| {
            let properties = board
                .active_view_property_layout
                .as_slice()
                .iter()
                .filter(|entry| entry.is_visible())
                .filter_map(|entry| {
                    let schema = board
                        .database_properties
                        .iter()
                        .find(|property| property.property_id == entry.property_id().as_str())
                        .expect("parsed database view property must exist in the schema");
                    if schema.property_type == "title" {
                        return None;
                    }
                    let value = item
                        .properties
                        .iter()
                        .find(|property| property.property_id == entry.property_id().as_str())
                        .map(|property| property.value.clone())
                        .unwrap_or_default();
                    Some(DatabaseViewPropertyRow {
                        label: schema.label.clone().into(),
                        property_type: schema.property_type.clone().into(),
                        value: value.into(),
                    })
                })
                .collect::<Vec<_>>()
                .into();
            DatabaseViewRow {
                block_id: item.block_id.clone().into(),
                title: item.title.clone().into(),
                icon: item.icon.clone(),
                properties,
            }
        })
        .collect::<Vec<_>>()
        .into()
}

pub(crate) fn database_view_control_property_rows(
    board: &BoardSnapshot,
) -> Arc<[DatabaseViewControlPropertyRow]> {
    let grouped_property_id = board
        .active_view_group
        .supported()
        .map(|group| group.property_id().as_str());
    board
        .database_properties
        .iter()
        .map(|property| DatabaseViewControlPropertyRow {
            property_id: property.property_id.clone().into(),
            label: property.label.clone().into(),
            property_type: property.property_type.clone().into(),
            visibility: board
                .active_view_property_layout
                .visibility_for(&property.property_id),
            sort_direction: board
                .active_view_sorts
                .as_slice()
                .iter()
                .find(|sort| sort.property_id().as_str() == property.property_id)
                .map(|sort| sort.direction()),
            group_kind: DatabaseViewGroupKind::parse_schema_type(&property.property_type).ok(),
            grouped: grouped_property_id == Some(property.property_id.as_str()),
        })
        .collect::<Vec<_>>()
        .into()
}

#[derive(Default)]
pub(crate) struct DatabaseViewControlsState {
    pub(crate) property_rows: Arc<[DatabaseViewControlPropertyRow]>,
    pub(crate) mutation_in_flight: bool,
}
