use super::presence::page_presence_profile;
use super::{
    normalize_uuid, required_string, unwrap_record_value, HashMap, HashSet, Map,
    NotionSpaceShortId, PagePresenceProfile, Url, Value,
};
use crate::live::http::ActiveUserNotionResponse;
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use chrono::{Offset, Utc};

mod bookmarks;
mod hydration;
mod sidebar;

use sidebar::{optional_record_table, sidebar_bookmarks, space_view_sidebars};

pub(super) struct BoardTarget {
    pub(super) collection_view_block_id: String,
    pub(super) collection_view_id: Option<String>,
    source_url: Url,
}

impl BoardTarget {
    pub(super) fn parse(board_url: &str) -> Result<Self, String> {
        let source_url = Url::parse(board_url)
            .map_err(|error| format!("invalid Notion workspace URL: {error}"))?;
        let collection_view_block_id = source_url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .ok_or_else(|| "missing Notion root block id".to_string())?;
        let collection_view_id = source_url
            .query_pairs()
            .find_map(|(name, value)| (name == "v").then(|| value.into_owned()));

        Ok(Self {
            collection_view_block_id: normalize_uuid(collection_view_block_id),
            collection_view_id: collection_view_id.map(|value| normalize_uuid(&value)),
            source_url,
        })
    }

    pub(super) fn child_url(&self, child_block_id: &str) -> String {
        let mut child_url = self.source_url.clone();
        let parent_path = child_url
            .path()
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or_default()
            .to_string();
        child_url.set_path(&format!(
            "{parent_path}/{}",
            child_block_id.replace('-', "")
        ));
        child_url.set_query(None);
        child_url.set_fragment(None);
        child_url.to_string()
    }

    pub(super) fn search_result_url(&self, block_id: &str) -> String {
        if normalize_uuid(block_id) == self.collection_view_block_id {
            return self.source_url.to_string();
        }
        self.child_url(block_id)
    }
}

#[derive(Clone, Debug)]
pub(super) struct UserContext {
    pub(super) user_id: String,
    pub(super) profile: PagePresenceProfile,
    pub(super) time_zone: String,
    pub(super) utc_offset_seconds: i32,
    pub(super) space_short_id: NotionSpaceShortId,
    pub(super) space_view_ids_by_space_id: HashMap<String, String>,
    pub(super) bookmarked_pages_by_space_view_id: HashMap<String, HashSet<String>>,
    pub(super) initial_blocks: Map<String, Value>,
    pub(super) can_create_custom_emoji: bool,
    sidebar_by_space_id: HashMap<String, SpaceViewSidebar>,
    space_names_by_space_id: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub(super) struct SpaceViewSidebar {
    pub(super) favorite_page_ids: Vec<String>,
    pub(super) joined_team_ids: Vec<String>,
    pub(super) pinned_chat_ids: Vec<String>,
    pub(super) shared_page_ids: Vec<String>,
    pub(super) private_page_ids: Vec<String>,
    pub(super) selected_calendar_bot_id: Option<String>,
}

impl UserContext {
    pub(super) fn space_view_id_for_space(&self, space_id: &str) -> Option<&str> {
        self.space_view_ids_by_space_id
            .get(space_id)
            .map(String::as_str)
    }

    pub(super) fn is_page_bookmarked(&self, space_view_id: &str, block_id: &str) -> bool {
        self.bookmarked_pages_by_space_view_id
            .get(space_view_id)
            .is_some_and(|bookmarked_pages| bookmarked_pages.contains(block_id))
    }

    pub(super) fn space_name(&self, space_id: &str) -> Result<&str, String> {
        self.space_names_by_space_id
            .get(space_id)
            .map(String::as_str)
            .ok_or_else(|| format!("missing Notion space {space_id}"))
    }

    pub(super) fn sidebar_for_space(&self, space_id: &str) -> Result<&SpaceViewSidebar, String> {
        self.sidebar_by_space_id
            .get(space_id)
            .ok_or_else(|| "missing Notion sidebar for current space".to_string())
    }
}

pub(super) fn load_user_context_with_session(
    session: &NotionDesktopSession,
    space_id: &str,
) -> Result<UserContext, NotionLiveError> {
    complete_user_context_with_session(
        session,
        request_initial_user_context_with_session(session)?,
        space_id,
    )
}

pub(super) fn request_initial_user_context_with_session(
    session: &NotionDesktopSession,
) -> Result<ActiveUserNotionResponse, NotionLiveError> {
    hydration::load_initial_user_context_with_session(session)
}

pub(super) fn complete_user_context_with_session(
    session: &NotionDesktopSession,
    response: ActiveUserNotionResponse,
    space_id: &str,
) -> Result<UserContext, NotionLiveError> {
    let (active_user_id, mut response) = response.into_parts();
    hydration::hydrate_target_space_records_with_session(
        session,
        &mut response,
        &active_user_id,
        space_id,
    )?;
    parse_user_context(&response, &active_user_id, space_id).map_err(NotionLiveError::Fatal)
}

pub(super) fn parse_user_context(
    response: &Value,
    active_user_id: &str,
    space_id: &str,
) -> Result<UserContext, String> {
    let users = response
        .get("users")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing users map".to_string())?;
    let user_records = users
        .get(active_user_id)
        .ok_or_else(|| "Notion user context did not include the active Desktop user".to_string())?;
    let (time_zone, utc_offset_seconds) = user_time_zone(user_records, active_user_id)?;
    let profile = current_user_profile(user_records, active_user_id)?;
    let space_short_id = current_space_short_id(user_records, space_id)?;
    let mut space_view_ids_by_space_id = user_root_space_view_ids(user_records, active_user_id);
    let space_views = optional_record_table(user_records, "space_view")?;
    if let Some(space_views) = space_views {
        merge_space_view_ids(space_views, &mut space_view_ids_by_space_id);
    }
    let sidebar_by_space_id = space_view_sidebars(space_views, &space_view_ids_by_space_id)?;
    let bookmarked_pages_by_space_view_id =
        sidebar_bookmarks(&sidebar_by_space_id, &space_view_ids_by_space_id);
    let initial_blocks = optional_record_table(user_records, "block")?
        .cloned()
        .unwrap_or_default();
    let space_names_by_space_id = named_records(user_records, "space")?;
    let can_create_custom_emoji = can_create_custom_emoji(user_records, space_id)?;

    Ok(UserContext {
        user_id: active_user_id.to_string(),
        profile,
        time_zone,
        utc_offset_seconds,
        space_short_id,
        space_view_ids_by_space_id,
        bookmarked_pages_by_space_view_id,
        initial_blocks,
        can_create_custom_emoji,
        sidebar_by_space_id,
        space_names_by_space_id,
    })
}

fn can_create_custom_emoji(user_records: &Value, space_id: &str) -> Result<bool, String> {
    let space_entry = user_records
        .get("space")
        .and_then(Value::as_object)
        .and_then(|spaces| spaces.get(space_id))
        .ok_or_else(|| format!("missing hydrated Notion space {space_id}"))?;
    let role = space_entry
        .get("value")
        .and_then(|wrapper| wrapper.get("role"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing Notion space role for space {space_id}"))?;
    let can_admin = role == "editor";
    let can_read = matches!(
        role,
        "reader"
            | "comment_only"
            | "content_only_editor"
            | "read_and_write"
            | "membership_admin"
            | "editor"
    );
    let space = unwrap_record_value(space_entry)
        .ok_or_else(|| format!("missing hydrated Notion space {space_id}"))?;
    let creation_only_admin = match space
        .get("settings")
        .and_then(|settings| settings.get("custom_emoji_creation_only_admin"))
    {
        Some(setting) => setting
            .as_bool()
            .ok_or_else(|| "invalid Notion custom-emoji creation setting".to_string())?,
        None => false,
    };
    Ok(can_admin || (can_read && !creation_only_admin))
}

fn current_space_short_id(
    user_records: &Value,
    space_id: &str,
) -> Result<NotionSpaceShortId, String> {
    let space = user_records
        .get("space")
        .and_then(Value::as_object)
        .and_then(|spaces| spaces.get(space_id))
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing hydrated Notion space {space_id}"))?;
    NotionSpaceShortId::try_from(required_string(space, "short_id_str")?)
}

fn current_user_profile(
    user_records: &Value,
    active_user_id: &str,
) -> Result<PagePresenceProfile, String> {
    let users = user_records
        .get("notion_user")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing authenticated notion_user map".to_string())?;
    let entry = users
        .get(active_user_id)
        .and_then(unwrap_record_value)
        .ok_or_else(|| "missing authenticated notion_user profile".to_string())?;
    page_presence_profile(entry)
}

fn named_records(user_records: &Value, table: &str) -> Result<HashMap<String, String>, String> {
    let Some(records) = user_records.get(table).and_then(Value::as_object) else {
        return Ok(HashMap::new());
    };
    records
        .iter()
        .map(|(id, entry)| {
            let record = unwrap_record_value(entry)
                .ok_or_else(|| format!("missing Notion {table} record {id}"))?;
            Ok((id.clone(), required_string(record, "name")?.to_string()))
        })
        .collect()
}

pub(super) fn space_view_space_id(entry: &Value) -> Option<&str> {
    unwrap_record_value(entry)
        .and_then(|value| value.get("space_id"))
        .and_then(Value::as_str)
        .or_else(|| entry.get("spaceId").and_then(Value::as_str))
}

fn user_time_zone(user_records: &Value, user_id: &str) -> Result<(String, i32), String> {
    let time_zone = user_records
        .get("user_settings")
        .and_then(|settings| settings.get(user_id))
        .and_then(unwrap_record_value)
        .and_then(|value| value.get("settings"))
        .and_then(|settings| settings.get("time_zone"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Notion user settings require an IANA time zone".to_string())?;
    let parsed_time_zone = time_zone
        .parse::<chrono_tz::Tz>()
        .map_err(|error| format!("Notion user time zone {time_zone:?} is invalid: {error}"))?;
    let utc_offset_seconds = Utc::now()
        .with_timezone(&parsed_time_zone)
        .offset()
        .fix()
        .local_minus_utc();
    Ok((time_zone.to_string(), utc_offset_seconds))
}

fn user_root_space_view_ids(user_records: &Value, user_id: &str) -> HashMap<String, String> {
    user_records
        .get("user_root")
        .and_then(|roots| roots.get(user_id))
        .and_then(unwrap_record_value)
        .and_then(|value| value.get("space_view_pointers"))
        .and_then(Value::as_array)
        .map(|pointers| {
            pointers
                .iter()
                .filter_map(space_view_pointer)
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default()
}

fn space_view_pointer(pointer: &Value) -> Option<(String, String)> {
    Some((
        required_string(pointer, "spaceId").ok()?.to_string(),
        required_string(pointer, "id").ok()?.to_string(),
    ))
}

fn merge_space_view_ids(
    space_views: &Map<String, Value>,
    space_view_ids_by_space_id: &mut HashMap<String, String>,
) {
    for (space_view_id, entry) in space_views {
        if let Some(space_id) = space_view_space_id(entry) {
            space_view_ids_by_space_id
                .entry(space_id.to_string())
                .or_insert_with(|| space_view_id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BoardTarget;

    #[gpui::test]
    fn parse_board_target_without_view_id() {
        let target = BoardTarget::parse(
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23",
        )
        .expect("board target should parse");

        assert_eq!(
            target.collection_view_block_id,
            "d4e5f6a7-b8c9-4a4b-b526-d7e8f90a1b23"
        );
        assert_eq!(target.collection_view_id, None);
    }

    #[gpui::test]
    fn parse_board_target_with_view_id() {
        let target = BoardTarget::parse(
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23?v=1a2b3c4d5e6f7890abcdeffedcba0123",
        )
        .expect("board target should parse");

        assert_eq!(
            target.collection_view_id.as_deref(),
            Some("1a2b3c4d-5e6f-7890-abcd-effedcba0123")
        );
    }
}
