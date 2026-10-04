use super::{
    optional_record_table, space_view_space_id, unwrap_record_value, user_root_space_view_ids, Map,
    Value,
};
use crate::live::board::{record_map_table, required_string};
use crate::live::http::{
    post_private_api_for_active_user_with_session, ActiveUserNotionResponse,
    NotionPrivateApiEndpoint,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use serde_json::json;

pub(super) fn load_initial_user_context_with_session(
    session: &NotionDesktopSession,
) -> Result<ActiveUserNotionResponse, NotionLiveError> {
    post_private_api_for_active_user_with_session(
        session,
        NotionPrivateApiEndpoint::GetSpacesInitial,
        &json!({}),
    )
}

pub(super) fn hydrate_target_space_records_with_session(
    session: &NotionDesktopSession,
    response: &mut Value,
    active_user_id: &str,
    space_id: &str,
) -> Result<(), NotionLiveError> {
    let user_records = active_user_records(response, active_user_id)?;
    let space_view_id = user_root_space_view_ids(user_records, active_user_id)
        .get(space_id)
        .cloned()
        .ok_or_else(|| format!("missing Notion space_view pointer for space {space_id}"))?;
    let missing_space_view =
        !target_record_is_loaded(user_records, "space_view", &space_view_id, Some(space_id))?;
    let missing_space = !target_record_is_loaded(user_records, "space", space_id, None)?;
    let mut requests = Vec::with_capacity(2);
    if missing_space_view {
        requests.push(json!({
            "pointer": { "table": "space_view", "id": space_view_id, "spaceId": space_id },
            "version": -1,
        }));
    }
    if missing_space {
        requests.push(json!({
            "pointer": { "table": "space", "id": space_id },
            "version": -1,
        }));
    }
    if requests.is_empty() {
        return Ok(());
    }

    let hydration = post_private_api_for_active_user_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({ "requests": requests }),
    )?;
    let (hydration_user_id, hydration) = hydration.into_parts();
    if hydration_user_id != active_user_id {
        return Err(NotionLiveError::Fatal(
            "Notion Desktop active user changed while loading the workspace".to_string(),
        ));
    }

    let hydrated_space_view = missing_space_view
        .then(|| validated_target_record(&hydration, "space_view", &space_view_id, Some(space_id)))
        .transpose()?;
    let hydrated_space = missing_space
        .then(|| validated_target_record(&hydration, "space", space_id, None))
        .transpose()?;
    let user_records = active_user_records_mut(response, active_user_id)?;
    if let Some(entry) = hydrated_space_view {
        merge_hydrated_record(user_records, "space_view", &space_view_id, entry)?;
    }
    if let Some(entry) = hydrated_space {
        merge_hydrated_record(user_records, "space", space_id, entry)?;
    }
    Ok(())
}

fn active_user_records<'a>(response: &'a Value, active_user_id: &str) -> Result<&'a Value, String> {
    response
        .get("users")
        .and_then(|users| users.get(active_user_id))
        .ok_or_else(|| "Notion user context did not include the active Desktop user".to_string())
}

fn active_user_records_mut<'a>(
    response: &'a mut Value,
    active_user_id: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    response
        .get_mut("users")
        .and_then(Value::as_object_mut)
        .and_then(|users| users.get_mut(active_user_id))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "invalid records for active Notion Desktop user".to_string())
}

fn target_record_is_loaded(
    user_records: &Value,
    table: &str,
    record_id: &str,
    space_id: Option<&str>,
) -> Result<bool, String> {
    let Some(records) = optional_record_table(user_records, table)? else {
        return Ok(false);
    };
    let Some(entry) = records.get(record_id) else {
        return Ok(false);
    };
    let record_id_matches = unwrap_record_value(entry)
        .and_then(|record| record.get("id"))
        .and_then(Value::as_str)
        == Some(record_id);
    Ok(record_id_matches
        && space_id.is_none_or(|space_id| space_view_space_id(entry) == Some(space_id)))
}

fn validated_target_record(
    hydration: &Value,
    table: &str,
    record_id: &str,
    space_id: Option<&str>,
) -> Result<Value, String> {
    let entry = record_map_table(hydration, table)?
        .get(record_id)
        .ok_or_else(|| format!("missing hydrated Notion {table} record {record_id}"))?;
    let record = unwrap_record_value(entry)
        .ok_or_else(|| format!("missing hydrated Notion {table} value {record_id}"))?;
    if required_string(record, "id")? != record_id {
        return Err(format!(
            "hydrated Notion {table} record id did not match {record_id}"
        ));
    }
    if let Some(space_id) = space_id {
        if space_view_space_id(entry) != Some(space_id) {
            return Err(format!(
                "hydrated Notion {table} {record_id} did not belong to space {space_id}"
            ));
        }
    }
    Ok(entry.clone())
}

fn merge_hydrated_record(
    user_records: &mut Map<String, Value>,
    table: &str,
    record_id: &str,
    entry: Value,
) -> Result<(), String> {
    let records = user_records
        .entry(table.to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("invalid Notion {table} map"))?;
    records.insert(record_id.to_string(), entry);
    Ok(())
}
