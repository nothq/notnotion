mod api;
mod calendar;
mod chat;
mod context;
mod inbox;
mod recents;
mod records;
mod search;
mod tree;

pub(super) use api::load_missing_sidebar_blocks;
use api::{
    load_meetings, load_shared_page_blocks, load_sidebar_collections,
    load_sidebar_node_collections, load_teamspaces, MeetingsData, TeamspaceData,
};
pub(super) use records::combined_block_records;
use records::is_available_sidebar_root;
pub(in crate::live::board) use tree::explicit_page_shell_icon;
pub(super) use tree::page_shell_icon;
use tree::{
    named_page_shell_icon, parse_section, ParsedSidebarNode, ParsedSidebarSection,
    SidebarSectionHeader,
};

use super::super::{
    block_value, load_complete_page_response_with_session, record_map_table, required_string,
    user::SpaceViewSidebar, BoardTarget, Map, UserContext, Value,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use crate::model::{
    LoadSidebarChildrenResult, PageShellLink, PageShellNodeIdentity, PageShellSidebarSection,
    PageShellSidebarSectionIdentity,
};

pub(crate) struct LoadedSidebar {
    pub(crate) builtin_links: Vec<PageShellLink>,
    pub(crate) sections: Vec<PageShellSidebarSection>,
}

pub(crate) use calendar::load_sidebar_calendar_for_context;
pub(crate) use chat::load_sidebar_chats;
pub(crate) use inbox::{load_sidebar_inbox, mutate_sidebar_inbox};
pub(crate) use recents::load_recent_pages;
pub(crate) use search::search_workspace;

pub(crate) fn load_sidebar(
    session: &NotionDesktopSession,
    space_id: &str,
    user_context: &UserContext,
    board_target: &BoardTarget,
    selected_page_blocks: &Map<String, Value>,
) -> Result<LoadedSidebar, NotionLiveError> {
    let sidebar = user_context.sidebar_for_space(space_id)?;
    let shared_blocks = load_shared_page_blocks(session, space_id)?;
    let teamspaces = load_teamspaces(
        session,
        space_id,
        &user_context.user_id,
        &sidebar.joined_team_ids,
    )?;
    let cached_blocks = combined_block_records([
        &user_context.initial_blocks,
        &shared_blocks,
        selected_page_blocks,
    ])?;
    let root_page_ids = visible_root_page_ids(sidebar, &teamspaces);
    let root_blocks =
        load_missing_sidebar_blocks(session, space_id, &root_page_ids, &cached_blocks)?;
    let meetings = load_meetings(session, space_id)?;
    let blocks = combined_block_records(
        [&cached_blocks, &root_blocks]
            .into_iter()
            .chain(meetings.iter().map(|meetings| &meetings.blocks)),
    )?;
    let parsed_sections =
        parsed_sidebar_sections(space_id, sidebar, &teamspaces, meetings.as_ref(), &blocks)?;
    let collections = load_sidebar_collections(session, &parsed_sections)?;
    let sections = parsed_sections
        .into_iter()
        .map(|section| section.resolve(&collections, board_target))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LoadedSidebar {
        builtin_links: builtin_links(),
        sections,
    })
}

pub(super) fn load_initial_sidebar(
    space_id: &str,
    user_context: &UserContext,
    board_target: &BoardTarget,
    selected_page_blocks: &Map<String, Value>,
) -> Result<LoadedSidebar, String> {
    let sidebar = user_context.sidebar_for_space(space_id)?;
    let blocks = combined_block_records([&user_context.initial_blocks, selected_page_blocks])?;
    let sections = parsed_sidebar_sections(space_id, sidebar, &[], None, &blocks)?
        .into_iter()
        .map(|section| section.resolve_initial(board_target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadedSidebar {
        builtin_links: builtin_links(),
        sections,
    })
}

pub(crate) fn load_sidebar_children(
    session: &NotionDesktopSession,
    current_board_url: &str,
    parent_block_id: &str,
) -> Result<LoadSidebarChildrenResult, NotionLiveError> {
    let bootstrap = load_complete_page_response_with_session(session, parent_block_id)?;
    let initial_blocks = record_map_table(bootstrap.as_value(), "block")?;
    let root = block_value(initial_blocks, parent_block_id)?;
    let space_id = required_string(root, "space_id")?;
    let board_target = BoardTarget::parse(current_board_url)?;
    let blocks = hydrate_sidebar_node_blocks(session, space_id, parent_block_id, initial_blocks)?;
    let parsed_parent = ParsedSidebarNode::parse(parent_block_id, &blocks, space_id)?;
    let collections = load_sidebar_node_collections(session, &parsed_parent)?;
    let (children, sidebar_children_resolved) =
        parsed_parent.resolve_children(&collections, &board_target)?;
    Ok(LoadSidebarChildrenResult {
        children,
        sidebar_children_resolved,
    })
}

fn hydrate_sidebar_node_blocks(
    session: &NotionDesktopSession,
    space_id: &str,
    parent_block_id: &str,
    initial_blocks: &Map<String, Value>,
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut blocks = initial_blocks.clone();
    loop {
        let parsed_parent = ParsedSidebarNode::parse(parent_block_id, &blocks, space_id)?;
        if parsed_parent.unresolved_child_block_ids().is_empty() {
            return Ok(blocks);
        }
        let loaded = load_missing_sidebar_blocks(
            session,
            space_id,
            parsed_parent.unresolved_child_block_ids(),
            &blocks,
        )?;
        if loaded.is_empty() {
            return Ok(blocks);
        }
        let merged = combined_block_records([&blocks, &loaded])?;
        if merged == blocks {
            return Ok(blocks);
        }
        blocks = merged;
    }
}

fn parsed_sidebar_sections(
    space_id: &str,
    sidebar: &SpaceViewSidebar,
    teamspaces: &[TeamspaceData],
    meetings: Option<&MeetingsData>,
    blocks: &Map<String, Value>,
) -> Result<Vec<ParsedSidebarSection>, String> {
    let mut sections = Vec::with_capacity(teamspaces.len() + 4);
    if let Some(section) = meetings_section(meetings, blocks, space_id)? {
        sections.push(section);
    }
    sections.push(parse_available_section(
        SidebarSectionHeader {
            identity: PageShellSidebarSectionIdentity::Favorites,
            title: "Favorites",
            icon_name: "favorites",
        },
        &sidebar.favorite_page_ids,
        blocks,
        space_id,
    )?);
    sections.extend(teamspace_sections(teamspaces, blocks, space_id)?);
    sections.push(parse_available_section(
        SidebarSectionHeader {
            identity: PageShellSidebarSectionIdentity::Shared,
            title: "Shared",
            icon_name: "shared",
        },
        &sidebar.shared_page_ids,
        blocks,
        space_id,
    )?);
    sections.push(parse_available_section(
        SidebarSectionHeader {
            identity: PageShellSidebarSectionIdentity::Private,
            title: "Private",
            icon_name: "private",
        },
        &sidebar.private_page_ids,
        blocks,
        space_id,
    )?);
    Ok(sections)
}

fn visible_root_page_ids(sidebar: &SpaceViewSidebar, teamspaces: &[TeamspaceData]) -> Vec<String> {
    sidebar
        .favorite_page_ids
        .iter()
        .chain(&sidebar.shared_page_ids)
        .chain(&sidebar.private_page_ids)
        .chain(
            teamspaces
                .iter()
                .flat_map(|teamspace| teamspace.page_ids.iter()),
        )
        .cloned()
        .collect()
}

fn meetings_section(
    meetings: Option<&MeetingsData>,
    blocks: &Map<String, Value>,
    space_id: &str,
) -> Result<Option<ParsedSidebarSection>, String> {
    let Some(meetings) = meetings.filter(|meetings| !meetings.page_ids.is_empty()) else {
        return Ok(None);
    };
    let available_page_ids = available_root_page_ids(&meetings.page_ids, blocks);
    if available_page_ids.is_empty() {
        return Ok(None);
    }
    parse_section(
        SidebarSectionHeader {
            identity: PageShellSidebarSectionIdentity::Meetings {
                sidebar_section_id: meetings.sidebar_section_id.clone(),
            },
            title: "Meetings",
            icon_name: "meetings",
        },
        &available_page_ids,
        blocks,
        space_id,
    )
    .map(Some)
}

fn teamspace_sections(
    teamspaces: &[TeamspaceData],
    blocks: &Map<String, Value>,
    space_id: &str,
) -> Result<Vec<ParsedSidebarSection>, String> {
    teamspaces
        .iter()
        .map(|teamspace| {
            parse_available_section(
                SidebarSectionHeader {
                    identity: PageShellSidebarSectionIdentity::Teamspace {
                        team_id: teamspace.id.clone(),
                    },
                    title: &teamspace.name,
                    icon_name: if teamspace.is_default {
                        "home"
                    } else {
                        "teamspace"
                    },
                },
                &teamspace.page_ids,
                blocks,
                space_id,
            )
        })
        .collect()
}

fn parse_available_section(
    header: SidebarSectionHeader<'_>,
    page_ids: &[String],
    blocks: &Map<String, Value>,
    space_id: &str,
) -> Result<ParsedSidebarSection, String> {
    let available_page_ids = available_root_page_ids(page_ids, blocks);
    parse_section(header, &available_page_ids, blocks, space_id)
}

fn available_root_page_ids(page_ids: &[String], blocks: &Map<String, Value>) -> Vec<String> {
    page_ids
        .iter()
        .filter(|block_id| is_available_sidebar_root(blocks, block_id))
        .cloned()
        .collect()
}

fn builtin_links() -> Vec<PageShellLink> {
    [
        (PageShellNodeIdentity::Home, "Home", "home"),
        (PageShellNodeIdentity::Chat, "Chat", "chat"),
        (PageShellNodeIdentity::Meetings, "Meetings", "meetings"),
        (PageShellNodeIdentity::Inbox, "Inbox", "inbox"),
    ]
    .into_iter()
    .map(|(identity, title, icon)| PageShellLink {
        identity: Some(identity),
        title: title.to_string(),
        icon: named_page_shell_icon(icon),
        target_board_url: None,
    })
    .collect()
}
