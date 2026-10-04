use std::collections::HashSet;

use serde_json::{json, Value};

use crate::model::{
    DatabaseAdvancedFilterMutation, DatabaseAdvancedFilterState, DatabaseSimpleFilter,
    DatabaseSimpleFilterMutation, DatabaseSimpleFilterPlacement, DatabaseSimpleFilterState,
    DatabaseSimpleFiltersState, DatabaseViewFilterMutation, DatabaseViewFilterState,
};

use super::{filter_group_value, property_filter_value, DatabaseMutationContext};

pub(super) fn database_filter_operation(
    context: &DatabaseMutationContext,
    mutation: &DatabaseViewFilterMutation,
) -> Value {
    let pointer = json!({
        "table": "collection_view",
        "id": context.collection_view_id,
        "spaceId": context.favorite.space_id,
    });
    match mutation {
        DatabaseViewFilterMutation::Simple(mutation) => simple_filter_operation(pointer, mutation),
        DatabaseViewFilterMutation::Advanced(mutation) => {
            let filter = match mutation {
                DatabaseAdvancedFilterMutation::Set(filter) => filter_group_value(filter),
                DatabaseAdvancedFilterMutation::Clear => Value::Null,
            };
            json!({
                "pointer": pointer,
                "path": ["query2"],
                "command": "update",
                "args": { "filter": filter },
            })
        }
    }
}

pub(super) fn database_filter_save_operations(
    context: &DatabaseMutationContext,
    filter_state: &DatabaseViewFilterState,
) -> Result<Vec<Value>, String> {
    let pointer = json!({
        "table": "collection_view",
        "id": context.collection_view_id,
        "spaceId": context.favorite.space_id,
    });
    let mut operations = simple_filter_save_operations(
        pointer.clone(),
        context.filter_state.simple(),
        filter_state.simple(),
    )?;
    if let Some(operation) = advanced_filter_save_operation(
        pointer,
        context.filter_state.advanced(),
        filter_state.advanced(),
    )? {
        operations.push(operation);
    }
    if operations.is_empty() {
        return Err(
            "Notion database filter save has no complete representable operation".to_string(),
        );
    }
    Ok(operations)
}

fn simple_filter_save_operations(
    pointer: Value,
    current: &DatabaseSimpleFiltersState,
    next: &DatabaseSimpleFiltersState,
) -> Result<Vec<Value>, String> {
    let DatabaseSimpleFiltersState::Entries(current) = current else {
        return Err(
            "unsupported current Notion simple filter list cannot be saved safely".to_string(),
        );
    };
    let DatabaseSimpleFiltersState::Entries(next) = next else {
        return Err("unsupported Notion simple filter list cannot be saved safely".to_string());
    };
    validate_unique_simple_filter_ids(current, "current")?;
    validate_unique_simple_filter_ids(next, "saved")?;
    let current_unsupported = unsupported_simple_filter_ids(current);
    let next_unsupported = unsupported_simple_filter_ids(next);
    if current_unsupported != next_unsupported {
        return Err(
            "Notion database filter save cannot add, remove, or reorder unsupported simple filters"
                .to_string(),
        );
    }

    let mut operations = Vec::new();
    for filter in current {
        let DatabaseSimpleFilterState::Editable(filter) = filter else {
            continue;
        };
        let mutation = DatabaseSimpleFilterMutation::Remove(filter.filter_id().clone());
        operations.push(simple_filter_operation(pointer.clone(), &mutation));
    }

    let mut before_filter_id = None;
    for filter in next.iter().rev() {
        match filter {
            DatabaseSimpleFilterState::Unsupported(filter_id) => {
                before_filter_id = Some(filter_id);
            }
            DatabaseSimpleFilterState::Editable(filter) => {
                let placement = match before_filter_id {
                    Some(filter_id) => DatabaseSimpleFilterPlacement::Before(filter_id.clone()),
                    None => DatabaseSimpleFilterPlacement::End,
                };
                let mutation = DatabaseSimpleFilterMutation::Add {
                    filter: filter.clone(),
                    placement,
                };
                operations.push(simple_filter_operation(pointer.clone(), &mutation));
                before_filter_id = Some(filter.filter_id());
            }
        }
    }
    Ok(operations)
}

fn validate_unique_simple_filter_ids(
    filters: &[DatabaseSimpleFilterState],
    state_name: &str,
) -> Result<(), String> {
    let mut filter_ids = HashSet::with_capacity(filters.len());
    for filter in filters {
        if !filter_ids.insert(filter.filter_id().as_str()) {
            return Err(format!(
                "Notion database filter {state_name} state contains duplicate filter ID {}",
                filter.filter_id().as_str()
            ));
        }
    }
    Ok(())
}

fn unsupported_simple_filter_ids(filters: &[DatabaseSimpleFilterState]) -> Vec<&str> {
    filters
        .iter()
        .filter_map(|filter| match filter {
            DatabaseSimpleFilterState::Editable(_) => None,
            DatabaseSimpleFilterState::Unsupported(filter_id) => Some(filter_id.as_str()),
        })
        .collect()
}

fn advanced_filter_save_operation(
    pointer: Value,
    current: &DatabaseAdvancedFilterState,
    next: &DatabaseAdvancedFilterState,
) -> Result<Option<Value>, String> {
    let filter = match next {
        DatabaseAdvancedFilterState::None => Value::Null,
        DatabaseAdvancedFilterState::Editable(filter) => filter_group_value(filter),
        DatabaseAdvancedFilterState::Unsupported
            if current == &DatabaseAdvancedFilterState::Unsupported =>
        {
            return Ok(None);
        }
        DatabaseAdvancedFilterState::Unsupported => {
            return Err("unsupported Notion advanced filter cannot be saved safely".to_string());
        }
    };
    Ok(Some(json!({
        "pointer": pointer,
        "path": ["query2"],
        "command": "update",
        "args": { "filter": filter },
    })))
}

fn simple_filter_operation(pointer: Value, mutation: &DatabaseSimpleFilterMutation) -> Value {
    match mutation {
        DatabaseSimpleFilterMutation::Add { filter, placement } => match placement {
            DatabaseSimpleFilterPlacement::End => json!({
                "pointer": pointer,
                "path": ["format", "property_filters"],
                "command": "keyedObjectListAfter",
                "args": { "value": simple_filter_value(filter) },
            }),
            DatabaseSimpleFilterPlacement::Before(filter_id) => json!({
                "pointer": pointer,
                "path": ["format", "property_filters"],
                "command": "keyedObjectListBefore",
                "args": {
                    "before": { "id": filter_id.as_str() },
                    "value": simple_filter_value(filter),
                },
            }),
        },
        DatabaseSimpleFilterMutation::Update(filter) => json!({
            "pointer": pointer,
            "path": ["format", "property_filters"],
            "command": "keyedObjectListUpdate",
            "args": { "value": simple_filter_value(filter) },
        }),
        DatabaseSimpleFilterMutation::Remove(filter_id) => json!({
            "pointer": pointer,
            "path": ["format", "property_filters"],
            "command": "keyedObjectListRemove",
            "args": { "remove": { "id": filter_id.as_str() } },
        }),
    }
}

fn simple_filter_value(filter: &DatabaseSimpleFilter) -> Value {
    json!({
        "id": filter.filter_id().as_str(),
        "filter": property_filter_value(filter.filter()),
    })
}
