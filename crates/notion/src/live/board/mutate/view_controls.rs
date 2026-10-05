use serde_json::{json, Map, Value};

use crate::model::{
    DatabaseViewControlMutation, DatabaseViewGroup, DatabaseViewGroupKind, DatabaseViewGroupState,
    DatabaseViewSortState, NotionDatabasePropertyId, ViewTabKind,
};

use super::super::{DatabaseMutationContext, DatabaseViewControlContext};

pub(super) struct PreparedDatabaseViewControlMutation {
    pub(super) operation: Value,
    pub(super) next: DatabaseViewControlContext,
}

pub(super) fn prepare_database_view_control_mutation(
    context: &DatabaseMutationContext,
    mutation: DatabaseViewControlMutation,
) -> Result<PreparedDatabaseViewControlMutation, String> {
    let mut next = context.view_controls.clone();
    let operation = match mutation {
        DatabaseViewControlMutation::AddSort(sort) => add_sort(context, &mut next, sort)?,
        DatabaseViewControlMutation::ToggleSortDirection(property_id) => {
            toggle_sort_direction(context, &mut next, property_id)?
        }
        DatabaseViewControlMutation::RemoveSort(property_id) => {
            remove_sort(context, &mut next, property_id)?
        }
        DatabaseViewControlMutation::SetGroup(property_id) => {
            set_group(context, &mut next, property_id)?
        }
        DatabaseViewControlMutation::ClearGroup => clear_group(context, &mut next)?,
        DatabaseViewControlMutation::SetPropertyVisibility {
            property_id,
            visible,
        } => set_property_visibility(context, &mut next, property_id, visible)?,
    };
    Ok(PreparedDatabaseViewControlMutation { operation, next })
}

fn add_sort(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    sort: crate::model::DatabaseViewSort,
) -> Result<Value, String> {
    require_known_property(next, sort.property_id())?;
    next.sorts = next.sorts.adding(sort)?;
    Ok(sort_operation(context, next))
}

fn toggle_sort_direction(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    property_id: NotionDatabasePropertyId,
) -> Result<Value, String> {
    require_known_property(next, &property_id)?;
    next.sorts = next.sorts.toggling(&property_id)?;
    Ok(sort_operation(context, next))
}

fn remove_sort(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    property_id: NotionDatabasePropertyId,
) -> Result<Value, String> {
    require_known_property(next, &property_id)?;
    next.sorts = next.sorts.removing(&property_id)?;
    Ok(sort_operation(context, next))
}

fn set_group(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    property_id: NotionDatabasePropertyId,
) -> Result<Value, String> {
    require_groupable_view(next.view_kind)?;
    if matches!(next.group, DatabaseViewGroupState::Unsupported { .. }) {
        return Err("unsupported Notion database grouping cannot be changed safely".to_string());
    }
    let property_type = require_known_property(next, &property_id)?;
    let kind = DatabaseViewGroupKind::parse_schema_type(property_type).map_err(str::to_string)?;
    if next
        .group
        .supported()
        .is_some_and(|group| group.property_id() == &property_id && group.kind() == kind)
    {
        return Err(format!(
            "Notion database view already groups by property {}",
            property_id.as_str()
        ));
    }
    next.group = DatabaseViewGroupState::Supported {
        group: DatabaseViewGroup::new(property_id.clone(), kind),
    };
    group_operation(context, next, Some((property_id, kind)))
}

fn clear_group(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
) -> Result<Value, String> {
    require_groupable_view(next.view_kind)?;
    match &next.group {
        DatabaseViewGroupState::None => {
            return Err("Notion database view grouping is already cleared".to_string());
        }
        DatabaseViewGroupState::Unsupported { .. } => {
            return Err(
                "unsupported Notion database grouping cannot be cleared safely".to_string(),
            );
        }
        DatabaseViewGroupState::Supported { .. } => {}
    }
    next.group = DatabaseViewGroupState::None;
    group_operation(context, next, None)
}

fn set_property_visibility(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    property_id: NotionDatabasePropertyId,
    visible: bool,
) -> Result<Value, String> {
    require_known_property(next, &property_id)?;
    next.property_layout = next
        .property_layout
        .setting_visibility(&property_id, visible)?;
    property_visibility_operation(context, next, &property_id, visible)
}

fn require_known_property<'a>(
    context: &'a DatabaseViewControlContext,
    property_id: &NotionDatabasePropertyId,
) -> Result<&'a str, String> {
    context
        .property_types
        .get(property_id.as_str())
        .map(String::as_str)
        .ok_or_else(|| {
            format!(
                "Notion database control references unknown property {}",
                property_id.as_str()
            )
        })
}

fn require_groupable_view(view_kind: ViewTabKind) -> Result<(), String> {
    if matches!(
        view_kind,
        ViewTabKind::Table | ViewTabKind::List | ViewTabKind::Gallery
    ) {
        return Ok(());
    }
    Err("Notion grouping changes require an active Table, List, or Gallery view".to_string())
}

fn collection_view_pointer(context: &DatabaseMutationContext) -> Value {
    json!({
        "table": "collection_view",
        "id": context.collection_view_id,
        "spaceId": context.favorite.space_id,
    })
}

fn sort_operation(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
) -> Value {
    next.query2
        .insert("sort".to_string(), Value::Array(sort_values(&next.sorts)));
    json!({
        "pointer": collection_view_pointer(context),
        "path": ["query2"],
        "command": "set",
        "args": Value::Object(next.query2.clone()),
    })
}

fn sort_values(sorts: &DatabaseViewSortState) -> Vec<Value> {
    sorts
        .as_slice()
        .iter()
        .map(|sort| {
            json!({
                "property": sort.property_id().as_str(),
                "direction": sort.direction().as_notion_str(),
            })
        })
        .collect()
}

fn group_operation(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    group: Option<(NotionDatabasePropertyId, DatabaseViewGroupKind)>,
) -> Result<Value, String> {
    let group_value = match group {
        Some((property_id, kind)) => {
            let mut value = match next.format.get("collection_group_by") {
                Some(Value::Object(existing)) => existing.clone(),
                None | Some(Value::Null) => new_group_value(),
                Some(_) => {
                    return Err(
                        "Notion format.collection_group_by must be an object or null".to_string(),
                    );
                }
            };
            value.insert(
                "property".to_string(),
                Value::String(property_id.as_str().to_string()),
            );
            value.insert(
                "type".to_string(),
                Value::String(kind.as_notion_str().to_string()),
            );
            Value::Object(value)
        }
        None => Value::Null,
    };
    next.format
        .insert("collection_group_by".to_string(), group_value.clone());
    Ok(json!({
        "pointer": collection_view_pointer(context),
        "path": ["format"],
        "command": "update",
        "args": { "collection_group_by": group_value },
    }))
}

fn new_group_value() -> Map<String, Value> {
    Map::from_iter([
        ("sort".to_string(), json!({ "type": "manual" })),
        ("hideEmptyGroups".to_string(), Value::Bool(false)),
    ])
}

fn property_visibility_operation(
    context: &DatabaseMutationContext,
    next: &mut DatabaseViewControlContext,
    property_id: &NotionDatabasePropertyId,
    visible: bool,
) -> Result<Value, String> {
    let format_key = property_layout_format_key(next.view_kind)?;
    let mut updated_layout = match next.format.get(format_key) {
        Some(Value::Array(layout)) => layout.clone(),
        // A view without a stored layout shows its default one, which the
        // first change stores, as Notion does.
        None | Some(Value::Null) => next
            .property_layout
            .as_slice()
            .iter()
            .map(|entry| {
                json!({ "property": entry.property_id().as_str(), "visible": entry.is_visible() })
            })
            .collect(),
        Some(_) => {
            return Err(format!(
                "Notion active view format.{format_key} must be an array"
            ));
        }
    };
    let mut matches = 0_usize;
    for entry in &mut updated_layout {
        let Some(entry) = entry.as_object_mut() else {
            return Err(format!(
                "Notion active view format.{format_key} entries must be objects"
            ));
        };
        if entry.get("property").and_then(Value::as_str) == Some(property_id.as_str()) {
            entry.insert("visible".to_string(), Value::Bool(visible));
            matches += 1;
        }
    }
    if matches != 1 {
        return Err(format!(
            "Notion active view format.{format_key} expected one entry for property {}, found {matches}",
            property_id.as_str()
        ));
    }
    next.format
        .insert(format_key.to_string(), Value::Array(updated_layout.clone()));
    let args = Map::from_iter([(format_key.to_string(), Value::Array(updated_layout))]);
    Ok(json!({
        "pointer": collection_view_pointer(context),
        "path": ["format"],
        "command": "update",
        "args": Value::Object(args),
    }))
}

fn property_layout_format_key(view_kind: ViewTabKind) -> Result<&'static str, String> {
    match view_kind {
        ViewTabKind::Table => Ok("table_properties"),
        ViewTabKind::List => Ok("list_properties"),
        ViewTabKind::Gallery => Ok("gallery_properties"),
        ViewTabKind::Board
        | ViewTabKind::Timeline
        | ViewTabKind::Calendar
        | ViewTabKind::Unknown => Err(
            "Notion property visibility requires an active Table, List, or Gallery view"
                .to_string(),
        ),
    }
}
