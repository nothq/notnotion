use super::{HashMap, HashSet, Map, SpaceViewSidebar, Value};
use crate::live::board::{optional_string_array, unwrap_record_value};

pub(super) fn space_view_sidebars(
    space_views: Option<&Map<String, Value>>,
    space_view_ids_by_space_id: &HashMap<String, String>,
) -> Result<HashMap<String, SpaceViewSidebar>, String> {
    let Some(space_views) = space_views else {
        return Ok(HashMap::new());
    };
    space_view_ids_by_space_id
        .iter()
        .filter_map(|(space_id, space_view_id)| {
            space_views
                .get(space_view_id)
                .map(|entry| (space_id, entry))
        })
        .map(|(space_id, entry)| {
            let value = unwrap_record_value(entry)
                .ok_or_else(|| "missing Notion space_view record value".to_string())?;
            Ok((
                space_id.clone(),
                SpaceViewSidebar {
                    favorite_page_ids: optional_string_array(value, "bookmarked_pages")?,
                    joined_team_ids: optional_string_array(value, "joined_teams")?,
                    pinned_chat_ids: pinned_chat_ids(value)?,
                    shared_page_ids: optional_string_array(value, "shared_pages")?,
                    private_page_ids: optional_string_array(value, "private_pages")?,
                    selected_calendar_bot_id: selected_calendar_bot_id(value)?,
                },
            ))
        })
        .collect()
}

fn pinned_chat_ids(space_view: &Value) -> Result<Vec<String>, String> {
    let Some(settings) = space_view.get("settings") else {
        return Ok(Vec::new());
    };
    let settings = settings
        .as_object()
        .ok_or_else(|| "invalid Notion space view settings".to_string())?;
    match settings.get("pinned_chat_ids") {
        Some(value) => value
            .as_array()
            .ok_or_else(|| "invalid Notion pinned chat ids".to_string())?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("invalid Notion pinned chat id at index {index}"))
            })
            .collect(),
        None => Ok(Vec::new()),
    }
}

fn selected_calendar_bot_id(space_view: &Value) -> Result<Option<String>, String> {
    let Some(calendar) = space_view
        .get("settings")
        .and_then(|settings| settings.get("personal_home"))
        .and_then(|personal_home| personal_home.get("calendar"))
    else {
        return Ok(None);
    };
    let Some(bot_id) = calendar.get("selected_calendar_bot_id") else {
        return Ok(None);
    };
    bot_id
        .as_str()
        .map(str::to_string)
        .map(Some)
        .ok_or_else(|| "invalid Notion selected calendar bot id".to_string())
}

pub(super) fn sidebar_bookmarks(
    sidebar_by_space_id: &HashMap<String, SpaceViewSidebar>,
    space_view_ids_by_space_id: &HashMap<String, String>,
) -> HashMap<String, HashSet<String>> {
    sidebar_by_space_id
        .iter()
        .filter_map(|(space_id, sidebar)| {
            space_view_ids_by_space_id
                .get(space_id)
                .map(|space_view_id| {
                    (
                        space_view_id.clone(),
                        sidebar.favorite_page_ids.iter().cloned().collect(),
                    )
                })
        })
        .collect()
}

type RecordTable = Map<String, Value>;

pub(super) fn optional_record_table<'a>(
    user_records: &'a Value,
    table: &str,
) -> Result<Option<&'a RecordTable>, String> {
    match user_records.get(table) {
        Some(records) => records
            .as_object()
            .map(Some)
            .ok_or_else(|| format!("invalid Notion {table} map")),
        None => Ok(None),
    }
}
