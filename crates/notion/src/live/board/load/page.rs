use super::super::{
    block_value, collection_database_name, collection_entry, collection_view_collection_id,
    format_edited_label, load_page_presence, loaded_record_value, optional_record_map_table,
    record_map_table, required_array, required_string, required_u64, title_property,
    unwrap_record_value, BoardTarget, CompletePageResponse, HashSet, LoadedBoardSnapshot, Map,
    UserContext, Value,
};
use super::sidebar::{load_initial_sidebar, page_shell_icon};
use super::snapshot::{build_loaded_page_snapshot, LoadedPageSnapshotInput};
use super::BootstrapBoardData;
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use crate::model::{PagePresenceSnapshot, PageShellLink, PageShellNodeIdentity, PageShellSnapshot};

use super::super::page::card_page_snapshot_from_response;

pub(super) struct LivePageLoadRequest<'a> {
    pub(super) session: &'a NotionDesktopSession,
    pub(super) board_target: BoardTarget,
    pub(super) user_context: &'a UserContext,
    pub(super) bootstrap: &'a CompletePageResponse,
    pub(super) current_page_shell: Option<&'a PageShellSnapshot>,
    pub(super) load_presence: bool,
}

pub(super) fn build_page_snapshot(
    request: LivePageLoadRequest<'_>,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let LivePageLoadRequest {
        session,
        board_target,
        user_context,
        bootstrap,
        current_page_shell,
        load_presence,
    } = request;
    let bootstrap_value = &bootstrap.value()?;
    let blocks = record_map_table(bootstrap_value, "block")?;
    let root = block_value(blocks, &board_target.collection_view_block_id)?.clone();
    let page_snapshot = card_page_snapshot_from_response(
        &board_target.collection_view_block_id,
        bootstrap.clone(),
    )?;
    let space_id = required_string(&root, "space_id")?.to_string();
    let page_shell = page_shell_snapshot(PageShellSnapshotRequest {
        space_id: &space_id,
        root: &root,
        blocks,
        bootstrap: bootstrap_value,
        opaque_unavailable_blocks: bootstrap.opaque_unavailable_blocks(),
        board_target: &board_target,
        user_context,
        current_page_shell,
    })?;
    let presence = if load_presence {
        load_optional_page_presence(session, &board_target, &space_id, user_context)?
    } else {
        None
    };
    let edited_label = format_edited_label(
        required_u64(&root, "last_edited_time")?,
        &user_context.time_zone,
    );
    Ok(build_loaded_page_snapshot(LoadedPageSnapshotInput {
        board_target,
        user_context,
        root,
        page: page_snapshot.page,
        page_mutation_state: page_snapshot.mutation_state,
        page_response: page_snapshot.response,
        page_shell,
        presence,
        space_id,
        edited_label,
    }))
}

/// Loads page presence, keeping the page usable when presence is unavailable.
pub(super) fn load_optional_page_presence(
    session: &NotionDesktopSession,
    board_target: &BoardTarget,
    space_id: &str,
    user_context: &UserContext,
) -> Result<Option<PagePresenceSnapshot>, NotionLiveError> {
    match load_page_presence(
        session,
        &board_target.collection_view_block_id,
        space_id,
        user_context,
    ) {
        Ok(presence) => Ok(Some(presence)),
        Err(NotionLiveError::Session(error)) => Err(NotionLiveError::Session(error)),
        Err(NotionLiveError::Unavailable(_) | NotionLiveError::Fatal(_)) => Ok(None),
    }
}

struct PageShellSnapshotRequest<'a> {
    space_id: &'a str,
    root: &'a Value,
    blocks: &'a Map<String, Value>,
    bootstrap: &'a Value,
    opaque_unavailable_blocks: &'a super::super::ProvenOpaqueUnavailableBlocks,
    board_target: &'a BoardTarget,
    user_context: &'a UserContext,
    current_page_shell: Option<&'a PageShellSnapshot>,
}

fn page_shell_snapshot(request: PageShellSnapshotRequest<'_>) -> Result<PageShellSnapshot, String> {
    let links = page_shell_links(&request)?;
    let PageShellSnapshotRequest {
        space_id,
        root,
        blocks,
        bootstrap,
        board_target,
        user_context,
        current_page_shell,
        ..
    } = request;
    build_page_shell_snapshot(PageShellSnapshotInput {
        space_id,
        root,
        blocks,
        bootstrap,
        board_target,
        user_context,
        fallback_icon: "page",
        links,
        current_page_shell,
    })
}

fn page_shell_links(request: &PageShellSnapshotRequest<'_>) -> Result<Vec<PageShellLink>, String> {
    let PageShellSnapshotRequest {
        space_id,
        root,
        blocks,
        bootstrap,
        opaque_unavailable_blocks,
        board_target,
        ..
    } = *request;
    let content = required_array(root, "content")?;
    let mut links = Vec::new();
    for child_id in content {
        let block_id = child_id
            .as_str()
            .ok_or_else(|| "Notion page root contains a non-string child block id".to_string())?;
        let block = match blocks.get(block_id).and_then(loaded_record_value) {
            Some(block) => block,
            None if opaque_unavailable_blocks
                .get(block_id)
                .is_some_and(|proof| {
                    proof.validates_content_edge(
                        block_id,
                        &board_target.collection_view_block_id,
                        space_id,
                    )
                }) =>
            {
                continue;
            }
            None => {
                return Err(format!(
                    "Notion page shell child {block_id} was not hydrated"
                ));
            }
        };
        if let Some(link) = page_shell_link(block_id, block, bootstrap, board_target)? {
            links.push(link);
        }
    }
    Ok(links)
}

fn page_shell_link(
    block_id: &str,
    block: &Value,
    bootstrap: &Value,
    board_target: &BoardTarget,
) -> Result<Option<PageShellLink>, String> {
    let block_type = required_string(block, "type")
        .map_err(|error| format!("Notion page shell child {block_id}: {error}"))?;
    let header = match block_type {
        "page" => Some({
            let title = title_property(block)?.unwrap_or_else(|| "Untitled".to_string());
            (
                title,
                "page",
                PageShellNodeIdentity::Page {
                    block_id: block_id.to_string(),
                },
            )
        }),
        "collection_view_page" => {
            let collection_id = collection_view_collection_id(
                block,
                optional_record_map_table(bootstrap, "collection_view"),
                None,
            )?;
            let collection = optional_record_map_table(bootstrap, "collection")
                .ok_or_else(|| "missing recordMap.collection for page shell child".to_string())
                .and_then(|records| collection_entry(records, collection_id))?;
            Some((
                collection_database_name(collection)?,
                "database",
                PageShellNodeIdentity::Database {
                    block_id: block_id.to_string(),
                },
            ))
        }
        _ => None,
    };
    Ok(
        header.map(|(title, fallback_icon, identity)| PageShellLink {
            identity: Some(identity),
            title,
            icon: page_shell_icon(block, fallback_icon),
            target_board_url: Some(board_target.child_url(block_id)),
        }),
    )
}

pub(super) fn database_page_shell_snapshot(
    bootstrap_data: &BootstrapBoardData<'_>,
    bootstrap: &Value,
    board_target: &BoardTarget,
    user_context: &UserContext,
    current_page_shell: Option<&PageShellSnapshot>,
) -> Result<PageShellSnapshot, String> {
    build_page_shell_snapshot(PageShellSnapshotInput {
        space_id: &bootstrap_data.space_id,
        root: &bootstrap_data.collection_view_block,
        blocks: bootstrap_data.blocks,
        bootstrap,
        board_target,
        user_context,
        fallback_icon: "database",
        links: Vec::new(),
        current_page_shell,
    })
}

struct PageShellSnapshotInput<'a> {
    space_id: &'a str,
    root: &'a Value,
    blocks: &'a Map<String, Value>,
    bootstrap: &'a Value,
    board_target: &'a BoardTarget,
    user_context: &'a UserContext,
    fallback_icon: &'a str,
    links: Vec<PageShellLink>,
    current_page_shell: Option<&'a PageShellSnapshot>,
}

fn build_page_shell_snapshot(
    input: PageShellSnapshotInput<'_>,
) -> Result<PageShellSnapshot, String> {
    let PageShellSnapshotInput {
        space_id,
        root,
        blocks,
        bootstrap,
        board_target,
        user_context,
        fallback_icon,
        links,
        current_page_shell,
    } = input;
    let workspace_name = page_parent_name(root, blocks, bootstrap, user_context)?;
    let (builtin_links, sidebar_sections) = match current_page_shell {
        Some(current_page_shell) => (
            current_page_shell.builtin_links.clone(),
            current_page_shell.sidebar_sections.clone(),
        ),
        None => {
            let sidebar = load_initial_sidebar(space_id, user_context, board_target, blocks)?;
            (sidebar.builtin_links, sidebar.sections)
        }
    };
    Ok(PageShellSnapshot {
        workspace_name,
        page_icon: page_shell_icon(root, fallback_icon),
        page_icon_is_explicit: root
            .get("format")
            .and_then(|format| format.get("page_icon"))
            .and_then(Value::as_str)
            .is_some(),
        builtin_links,
        sidebar_sections,
        links,
    })
}

fn page_parent_name(
    root: &Value,
    blocks: &Map<String, Value>,
    bootstrap: &Value,
    user_context: &UserContext,
) -> Result<String, String> {
    let mut parent_id = required_string(root, "parent_id")?.to_string();
    let mut parent_table = required_string(root, "parent_table")?.to_string();
    let mut visited_block_ids = HashSet::new();
    loop {
        match parent_table.as_str() {
            "block" => {
                if !visited_block_ids.insert(parent_id.clone()) {
                    return Err(format!("cyclic Notion page parent chain at {parent_id}"));
                }
                let parent = block_value(blocks, &parent_id)?;
                if let Some(title) = page_parent_block_name(parent, bootstrap)? {
                    return Ok(title);
                }
                parent_id = required_string(parent, "parent_id")?.to_string();
                parent_table = required_string(parent, "parent_table")?.to_string();
            }
            "collection" => return collection_parent_name(bootstrap, &parent_id),
            "team" => return record_map_parent_name(bootstrap, "team", &parent_id),
            "space" => return user_context.space_name(&parent_id).map(str::to_string),
            _ => {
                return Err(format!(
                    "unsupported Notion page parent table {parent_table}"
                ));
            }
        }
    }
}

fn page_parent_block_name(parent: &Value, bootstrap: &Value) -> Result<Option<String>, String> {
    if let Some(title) = title_property(parent)? {
        return Ok(Some(title));
    }
    if !matches!(
        required_string(parent, "type")?,
        "collection_view" | "collection_view_page"
    ) {
        return Ok(None);
    }
    let collection_id = collection_view_collection_id(
        parent,
        optional_record_map_table(bootstrap, "collection_view"),
        None,
    )?;
    collection_parent_name(bootstrap, collection_id).map(Some)
}

fn collection_parent_name(bootstrap: &Value, collection_id: &str) -> Result<String, String> {
    let collection = collection_entry(record_map_table(bootstrap, "collection")?, collection_id)?;
    collection_database_name(collection)
}

fn record_map_parent_name(
    bootstrap: &Value,
    table: &str,
    parent_id: &str,
) -> Result<String, String> {
    let parent = record_map_table(bootstrap, table)?
        .get(parent_id)
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing Notion {table} parent {parent_id}"))?;
    Ok(required_string(parent, "name")?.to_string())
}
