use serde_json::{Map, Value};

use super::super::super::super::{required_string, unwrap_record_value};
use crate::model::PageShellInboxActor;

pub(super) struct InboxRecords {
    tables: Map<String, Value>,
}

type RecordTable = Map<String, Value>;

pub(super) struct RecordPointer<'a> {
    pub(super) table: &'a str,
    pub(super) id: &'a str,
}

impl InboxRecords {
    pub(super) fn new(tables: Map<String, Value>) -> Self {
        Self { tables }
    }

    pub(super) fn table(&self, table: &str) -> Result<Option<&RecordTable>, String> {
        self.tables
            .get(table)
            .map(|records| {
                records
                    .as_object()
                    .ok_or_else(|| format!("invalid Notion inbox recordMap.{table}"))
            })
            .transpose()
    }

    pub(super) fn record(&self, table: &str, record_id: &str) -> Result<Option<&Value>, String> {
        Ok(self
            .table(table)?
            .and_then(|records| records.get(record_id))
            .and_then(unwrap_record_value))
    }

    pub(super) fn required_record(&self, table: &str, record_id: &str) -> Result<&Value, String> {
        self.record(table, record_id)?
            .ok_or_else(|| format!("missing Notion inbox {table} record {record_id}"))
    }

    pub(super) fn insert_hydrated_actor(
        &mut self,
        table: String,
        id: String,
        entry: Value,
    ) -> Result<(), String> {
        self.tables
            .entry(table)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| "invalid Notion activity actor table".to_string())?
            .insert(id, entry);
        Ok(())
    }
}

pub(super) fn validate_space(record: &Value, space_id: &str, table: &str) -> Result<(), String> {
    if required_string(record, "space_id")? != space_id {
        return Err(format!(
            "Notion inbox {table} record belonged to a different space"
        ));
    }
    Ok(())
}

pub(super) fn resolve_actor(
    activity: Option<&Value>,
    comment: Option<&Value>,
    records: &InboxRecords,
) -> Result<Option<PageShellInboxActor>, String> {
    let pointer = comment
        .map(|comment| record_pointer(comment, "created_by_table", "created_by_id"))
        .transpose()?
        .flatten()
        .or(activity.map(newest_activity_author).transpose()?.flatten());
    let Some(pointer) = pointer else {
        return Ok(None);
    };
    let Some(actor) = records.record(pointer.table, pointer.id)? else {
        return Ok(None);
    };
    let name = actor_name(pointer.table, actor)?;
    let Some(name) = name else {
        return Ok(None);
    };
    let avatar_key = match pointer.table {
        "notion_user" => "profile_photo",
        "bot" => "icon",
        _ => "profile_photo",
    };
    let avatar_url = optional_string(actor, avatar_key)?.map(str::to_string);
    Ok(Some(PageShellInboxActor {
        actor_id: pointer.id.to_string(),
        name,
        avatar_url,
    }))
}

pub(super) fn newest_activity_author(
    activity: &Value,
) -> Result<Option<RecordPointer<'_>>, String> {
    let Some(edits) = activity.get("edits") else {
        return Ok(None);
    };
    let edits = edits
        .as_array()
        .ok_or_else(|| "invalid Notion inbox activity edits".to_string())?;
    let mut latest = None;
    for edit in edits {
        let Some(authors) = edit.get("authors") else {
            continue;
        };
        let authors = authors
            .as_array()
            .ok_or_else(|| "invalid Notion inbox activity authors".to_string())?;
        let Some(author) = authors.first() else {
            continue;
        };
        let Some(pointer) = record_pointer(author, "table", "id")? else {
            continue;
        };
        let timestamp = optional_timestamp(edit, "timestamp")?.unwrap_or_default();
        if latest
            .as_ref()
            .is_none_or(|(latest_timestamp, _)| timestamp >= *latest_timestamp)
        {
            latest = Some((timestamp, pointer));
        }
    }
    Ok(latest.map(|(_, pointer)| pointer))
}

fn record_pointer<'a>(
    value: &'a Value,
    table_key: &str,
    id_key: &str,
) -> Result<Option<RecordPointer<'a>>, String> {
    let table = optional_string(value, table_key)?;
    let id = optional_string(value, id_key)?;
    match (table, id) {
        (Some(table), Some(id)) => Ok(Some(RecordPointer { table, id })),
        (None, None) => Ok(None),
        _ => Err(format!(
            "incomplete Notion inbox record pointer {table_key}/{id_key}"
        )),
    }
}

fn actor_name(table: &str, actor: &Value) -> Result<Option<String>, String> {
    if let Some(name) = optional_string(actor, "name")? {
        return Ok(non_empty_text(name.to_string()));
    }
    if table == "notion_user" {
        let given_name = optional_string(actor, "given_name")?.unwrap_or_default();
        let family_name = optional_string(actor, "family_name")?.unwrap_or_default();
        return Ok(non_empty_text(format!("{given_name} {family_name}")));
    }
    if table == "bot" {
        return Ok(Some("Unnamed bot".to_string()));
    }
    Ok(None)
}

pub(super) fn event_time_ms(
    notification: &Value,
    activity: Option<&Value>,
    comment: Option<&Value>,
) -> Result<Option<u64>, String> {
    for (record, key) in [
        (Some(notification), "end_time"),
        (activity, "end_time"),
        (comment, "created_time"),
        (activity, "start_time"),
    ] {
        if let Some(timestamp) = record
            .map(|record| optional_timestamp(record, key))
            .transpose()?
            .flatten()
        {
            return Ok(Some(timestamp));
        }
    }
    Ok(None)
}

pub(super) fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    value
        .get(key)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("invalid string field {key}"))
        })
        .transpose()
}

pub(super) fn optional_timestamp(value: &Value, key: &str) -> Result<Option<u64>, String> {
    value
        .get(key)
        .map(|value| match value {
            Value::Number(number) => number
                .as_u64()
                .ok_or_else(|| format!("invalid timestamp field {key}")),
            Value::String(timestamp) => timestamp
                .parse::<u64>()
                .map_err(|error| format!("invalid timestamp field {key}: {error}")),
            _ => Err(format!("invalid timestamp field {key}")),
        })
        .transpose()
}

pub(super) fn non_empty_text(text: String) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
