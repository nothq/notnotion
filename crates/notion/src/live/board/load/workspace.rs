use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    thread,
};

use super::super::{
    block_value, collection_entry, collection_view_collection_id, collection_view_entry,
    complete_user_context_with_session, find_sidebar_item,
    load_complete_page_response_with_session, load_user_context_with_session,
    navigation_cache::{
        NavigationBootstrap, NavigationBootstrapCache, NavigationBootstrapKey,
        NavigationBootstrapPriority, NavigationBootstrapResponse,
    },
    record_map_table, request_initial_user_context_with_session, required_string,
    resolve_active_view_id, BoardTarget, CompletePageResponse, LiveWorkspaceCache,
    LiveWorkspaceContext, LoadedBoardSnapshot, PageShellNodeIdentity, PageShellSidebarItem,
    PageShellSnapshot, UserContext, Value,
};
use super::{
    load_live_database_snapshot,
    page::{build_page_snapshot, LivePageLoadRequest},
    DatabaseFilterQueryInput, DatabasePresentation, LiveDatabaseLoadRequest,
};
use crate::live::http::ActiveUserNotionResponse;
use crate::live::{
    credentials::{current_notion_desktop_session, NotionDesktopSession},
    NotionLiveError,
};

mod context;
mod inline_database;

use context::LiveWorkspaceContextInput;
use inline_database::hydrate_inline_database_views;

pub fn load_board_snapshot(board_url: &str) -> Result<LoadedBoardSnapshot, String> {
    let session = current_notion_desktop_session().map_err(|error| error.to_string())?;
    load_board_snapshot_for_bootstrap(&session, board_url).map_err(|error| error.to_string())
}

pub(crate) fn load_board_snapshot_for_bootstrap(
    session: &NotionDesktopSession,
    board_url: &str,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    load_live_board_snapshot(session, board_url, None).map_err(with_board_load_context)
}

pub(crate) fn load_board_snapshot_for_navigation(
    session: &NotionDesktopSession,
    board_url: &str,
    current_context: &LiveWorkspaceContext,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    load_live_board_snapshot(session, board_url, Some(current_context))
        .map_err(with_board_load_context)
}

pub(crate) fn load_database_filter_snapshot_for_query(
    session: &NotionDesktopSession,
    current_context: &LiveWorkspaceContext,
    query: DatabaseFilterQueryInput,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let board_url = current_context.board_url.clone();
    let board_target = BoardTarget::parse(&board_url)?;
    let (initial_bootstrap, _) =
        load_initial_board_data(session, &board_target, Some(current_context))?;
    let cache = current_context.workspace_cache()?;
    if initial_bootstrap.active_user_id != cache.user_context.user_id {
        return Err(NotionLiveError::Fatal(
            "Notion page and user context resolved different active users".to_string(),
        ));
    }
    let bootstrap = initial_bootstrap.response.database_value()?;
    let mut loaded = load_live_database_snapshot(LiveDatabaseLoadRequest {
        session,
        board_target,
        user_context: &cache.user_context,
        bootstrap,
        current_page_shell: Some(&cache.page_shell),
        load_presence: false,
        presentation: DatabasePresentation::FullPage,
        query: Some(query),
    })?;
    loaded.workspace_context = Some(current_context.clone());
    Ok(loaded)
}

pub(crate) fn load_inline_database_snapshot_from_bootstrap(
    session: &NotionDesktopSession,
    board_url: &str,
    mut bootstrap: Value,
    current_context: &LiveWorkspaceContext,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let board_target = BoardTarget::parse(board_url)?;
    let page_block_id = board_target.collection_view_block_id.clone();
    let cache = current_context.workspace_cache()?;
    hydrate_inline_database_views(session, &board_target, &mut bootstrap)?;
    let (space_id, root_type) = inline_database_root(&bootstrap, &board_target)?;
    if space_id != cache.space_id {
        return Err(NotionLiveError::Fatal(format!(
            "Notion inline database {} belongs to space {space_id}, expected {}",
            board_target.collection_view_block_id, cache.space_id
        )));
    }
    let navigation_key =
        navigation_bootstrap_key(&cache.user_context.user_id, &cache.space_id, &board_target);
    let initial_bootstrap = NavigationBootstrap {
        active_user_id: cache.user_context.user_id.clone(),
        space_id: space_id.clone(),
        root_type: root_type.clone(),
        response: NavigationBootstrapResponse::inline_database(bootstrap.clone()),
    };
    let mut loaded = load_live_database_snapshot(LiveDatabaseLoadRequest {
        session,
        board_target,
        user_context: &cache.user_context,
        bootstrap,
        current_page_shell: Some(&cache.page_shell),
        load_presence: false,
        presentation: DatabasePresentation::Inline,
        query: None,
    })?;
    let page_shell =
        loaded.snapshot.page_shell.clone().ok_or_else(|| {
            "live Notion inline database did not include its page shell".to_string()
        })?;
    let page_title = loaded.snapshot.page_title.clone();
    loaded.workspace_context = Some(LiveWorkspaceContext::new(LiveWorkspaceContextInput {
        space_id,
        page_block_id,
        root_type: &root_type,
        board_url,
        user_context: cache.user_context,
        page_shell,
        page_title: &page_title,
        workspace_cache: Some(cache.shared),
        navigation_key,
        initial_bootstrap,
    })?);
    Ok(loaded)
}

fn inline_database_root(
    bootstrap: &Value,
    board_target: &BoardTarget,
) -> Result<(String, String), String> {
    let root = block_value(
        record_map_table(bootstrap, "block")?,
        &board_target.collection_view_block_id,
    )?;
    let root_type = required_string(root, "type")?.to_string();
    if root_type != "collection_view" {
        return Err(format!(
            "Notion inline database block {} has unsupported type {root_type}",
            board_target.collection_view_block_id
        ));
    }
    let collection_views = record_map_table(bootstrap, "collection_view")?;
    let active_view_id = resolve_active_view_id(
        root,
        collection_views,
        board_target.collection_view_id.as_deref(),
    )?;
    collection_view_entry(collection_views, &active_view_id)?;
    let collection_id =
        collection_view_collection_id(root, Some(collection_views), Some(&active_view_id))?;
    collection_entry(record_map_table(bootstrap, "collection")?, collection_id)?;
    Ok((required_string(root, "space_id")?.to_string(), root_type))
}

fn load_live_board_snapshot(
    session: &NotionDesktopSession,
    board_url: &str,
    current_context: Option<&LiveWorkspaceContext>,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let board_target = BoardTarget::parse(board_url)?;
    let page_block_id = board_target.collection_view_block_id.clone();
    let (initial_bootstrap, initial_user_context) =
        load_initial_board_data(session, &board_target, current_context)?;
    let space_id = initial_bootstrap.space_id.clone();
    let root_type = initial_bootstrap.root_type.clone();
    let current_cache = current_context
        .map(LiveWorkspaceContext::workspace_cache)
        .transpose()?
        .filter(|cache| cache.space_id == space_id);
    let user_context = match current_cache.as_ref() {
        Some(cache) => cache.user_context.clone(),
        None => Arc::new(match initial_user_context {
            Some(initial_user_context) => {
                complete_user_context_with_session(session, initial_user_context, &space_id)?
            }
            None => load_user_context_with_session(session, &space_id)?,
        }),
    };
    if user_context.user_id != initial_bootstrap.active_user_id {
        return Err(NotionLiveError::Fatal(
            "Notion page and user context resolved different active users".to_string(),
        ));
    }
    let current_page_shell = current_cache.as_ref().map(|cache| &cache.page_shell);
    let workspace_cache = current_cache.as_ref().map(|cache| cache.shared.clone());
    let navigation_key = navigation_bootstrap_key(&user_context.user_id, &space_id, &board_target);
    let mut loaded = build_loaded_board_snapshot(
        session,
        board_target,
        &user_context,
        &initial_bootstrap,
        current_page_shell,
    )?;
    let page_shell = loaded
        .snapshot
        .page_shell
        .clone()
        .ok_or_else(|| "live Notion workspace did not include its page shell".to_string())?;
    loaded.workspace_context = Some(LiveWorkspaceContext::new(LiveWorkspaceContextInput {
        space_id,
        page_block_id,
        root_type: &root_type,
        board_url,
        user_context,
        page_shell,
        page_title: &loaded.snapshot.page_title,
        workspace_cache,
        navigation_key,
        initial_bootstrap,
    })?);
    Ok(loaded)
}

/// The page bootstrap and, on a first load, the user context requested alongside it.
type InitialBoardData = (NavigationBootstrap, Option<ActiveUserNotionResponse>);

fn load_initial_board_data(
    session: &NotionDesktopSession,
    board_target: &BoardTarget,
    current_context: Option<&LiveWorkspaceContext>,
) -> Result<InitialBoardData, NotionLiveError> {
    let Some(current_context) = current_context else {
        let (bootstrap, user_context) = thread::scope(|scope| {
            let bootstrap = scope.spawn(|| request_navigation_bootstrap(session, board_target));
            let user_context = scope.spawn(|| request_initial_user_context_with_session(session));
            let bootstrap = bootstrap
                .join()
                .map_err(|_| "Notion page bootstrap thread panicked".to_string())??;
            let user_context = user_context
                .join()
                .map_err(|_| "Notion user context thread panicked".to_string())??;
            Ok::<_, NotionLiveError>((bootstrap, user_context))
        })?;
        return Ok((bootstrap, Some(user_context)));
    };
    let bootstrap = current_context
        .load_navigation_bootstrap(session, board_target, NavigationBootstrapPriority::Demand)?
        .expect("demand Notion navigation bootstrap must not be skipped");
    Ok((bootstrap, None))
}

fn build_loaded_board_snapshot(
    session: &NotionDesktopSession,
    board_target: BoardTarget,
    user_context: &UserContext,
    initial_bootstrap: &NavigationBootstrap,
    current_page_shell: Option<&PageShellSnapshot>,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let initial_response = &initial_bootstrap.response;
    let load_presence = false;
    match initial_bootstrap.root_type.as_str() {
        "page" | "transcription" => build_page_snapshot(LivePageLoadRequest {
            session,
            board_target,
            user_context,
            bootstrap: initial_response.completed_page()?,
            current_page_shell,
            load_presence,
        }),
        "collection_view" | "collection_view_page" => {
            let bootstrap = initial_response.database_value()?;
            load_live_database_snapshot(LiveDatabaseLoadRequest {
                session,
                board_target,
                user_context,
                bootstrap,
                current_page_shell,
                load_presence,
                presentation: DatabasePresentation::FullPage,
                query: None,
            })
        }
        root_type => Err(NotionLiveError::Fatal(format!(
            "unsupported Notion root block type {root_type}"
        ))),
    }
}

fn request_navigation_bootstrap(
    session: &NotionDesktopSession,
    board_target: &BoardTarget,
) -> Result<NavigationBootstrap, NotionLiveError> {
    let response =
        load_complete_page_response_with_session(session, &board_target.collection_view_block_id)?;
    let (space_id, root_type) = workspace_root(&response, board_target)?;
    Ok(NavigationBootstrap {
        active_user_id: session.user_id().to_string(),
        space_id,
        root_type,
        response: NavigationBootstrapResponse::complete(response),
    })
}

fn navigation_bootstrap_key(
    active_user_id: &str,
    space_id: &str,
    board_target: &BoardTarget,
) -> NavigationBootstrapKey {
    NavigationBootstrapKey {
        active_user_id: active_user_id.to_string(),
        space_id: space_id.to_string(),
        page_block_id: board_target.collection_view_block_id.clone(),
        collection_view_id: board_target.collection_view_id.clone(),
    }
}

fn workspace_root(
    bootstrap: &CompletePageResponse,
    board_target: &BoardTarget,
) -> Result<(String, String), NotionLiveError> {
    let bootstrap = &bootstrap.value()?;
    let blocks = record_map_table(bootstrap, "block")?;
    let block_id = &board_target.collection_view_block_id;
    let root = block_value(blocks, block_id).map_err(NotionLiveError::Fatal)?;
    Ok((
        required_string(root, "space_id")?.to_string(),
        required_string(root, "type")?.to_string(),
    ))
}

fn with_board_load_context(error: NotionLiveError) -> NotionLiveError {
    match error {
        NotionLiveError::Fatal(message) => {
            NotionLiveError::Fatal(format!("failed to load live Notion board: {message}"))
        }
        error => error,
    }
}
