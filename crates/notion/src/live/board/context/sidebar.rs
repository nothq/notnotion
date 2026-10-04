use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::model::{
    PageShellNodeIdentity, PageShellSearchBadge, PageShellSearchResult, PageShellSidebarItem,
    PageShellSidebarSection, PageShellSidebarSectionIdentity, PageShellSnapshot,
};

use super::super::{normalize_uuid, LiveWorkspaceCache, PendingFavoriteMutation};

pub(super) fn collect_local_search_page(
    item: &PageShellSidebarItem,
    breadcrumbs: &HashMap<String, String>,
    seen: &mut HashSet<String>,
    pages: &mut Vec<PageShellSearchResult>,
) {
    if let (Some(identity), Some(target_board_url)) =
        (item.identity.as_ref(), item.target_board_url.as_ref())
    {
        if let Some(block_id) = identity.block_id() {
            let normalized_block_id = normalize_uuid(block_id);
            if !item.title.trim().is_empty() && seen.insert(normalized_block_id.clone()) {
                let badges = matches!(identity, PageShellNodeIdentity::Database { .. })
                    .then_some(PageShellSearchBadge::Database)
                    .into_iter()
                    .collect();
                pages.push(PageShellSearchResult {
                    block_id: normalized_block_id.clone(),
                    title: item.title.clone(),
                    icon: item.icon.clone(),
                    target_board_url: target_board_url.clone(),
                    highlight: breadcrumbs.get(&normalized_block_id).cloned(),
                    match_snippet: None,
                    editor_display_name: None,
                    edited_label: None,
                    edited_at: None,
                    badges,
                });
            }
        }
    }
    for child in &item.children {
        collect_local_search_page(child, breadcrumbs, seen, pages);
    }
}

pub(in super::super) fn sidebar_breadcrumbs(
    page_shell: &PageShellSnapshot,
) -> HashMap<String, String> {
    let mut breadcrumbs = HashMap::new();
    for item in page_shell
        .sidebar_sections
        .iter()
        .flat_map(|section| &section.items)
    {
        collect_sidebar_breadcrumbs(item, &[], &mut breadcrumbs);
    }
    breadcrumbs
}

fn collect_sidebar_breadcrumbs(
    item: &PageShellSidebarItem,
    ancestors: &[String],
    breadcrumbs: &mut HashMap<String, String>,
) {
    if let (Some(block_id), Some(breadcrumb)) = (
        item.identity
            .as_ref()
            .and_then(PageShellNodeIdentity::block_id),
        collapsed_sidebar_breadcrumb(ancestors),
    ) {
        breadcrumbs.insert(normalize_uuid(block_id), breadcrumb);
    }
    let mut child_ancestors = ancestors.to_vec();
    let title = item.title.trim();
    if !title.is_empty() {
        child_ancestors.push(title.to_string());
    }
    for child in &item.children {
        collect_sidebar_breadcrumbs(child, &child_ancestors, breadcrumbs);
    }
}

fn collapsed_sidebar_breadcrumb(ancestors: &[String]) -> Option<String> {
    match ancestors {
        [] => None,
        [only] => Some(only.clone()),
        [first, second] => Some(format!("{first} / {second}")),
        [first, .., last] => Some(format!("{first} / … / {last}")),
    }
}

pub(super) fn effective_favorite_state(
    cache: &LiveWorkspaceCache,
    pending: &PendingFavoriteMutation,
) -> (bool, PageShellSidebarItem) {
    cache
        .pending_favorite_mutations
        .iter()
        .rev()
        .find(|queued| queued.page_block_id == pending.page_block_id)
        .map(|queued| (queued.is_favorited, queued.page_sidebar_item.clone()))
        .unwrap_or_else(|| {
            (
                cache
                    .authoritative_favorites_by_page
                    .get(&pending.page_block_id)
                    .copied()
                    .expect("favorite mutation must retain authoritative state"),
                pending.page_sidebar_item.clone(),
            )
        })
}

pub(super) fn synchronize_sidebar_item_active_state(
    item: &mut PageShellSidebarItem,
    page_block_id: &str,
    board_url: &str,
) {
    item.active = match item
        .identity
        .as_ref()
        .and_then(PageShellNodeIdentity::block_id)
    {
        Some(block_id) => normalize_uuid(block_id) == page_block_id,
        None => item.target_board_url.as_deref() == Some(board_url),
    };
    for child in &mut item.children {
        synchronize_sidebar_item_active_state(child, page_block_id, board_url);
    }
}

pub(super) fn cached_page_is_favorited(cache: &LiveWorkspaceCache, page_block_id: &str) -> bool {
    cache
        .user_context
        .space_view_id_for_space(&cache.space_id)
        .is_some_and(|space_view_id| {
            cache
                .user_context
                .is_page_bookmarked(space_view_id, page_block_id)
        })
}

pub(super) fn set_cached_page_favorited(
    cache: &mut LiveWorkspaceCache,
    page_block_id: &str,
    is_favorited: bool,
    page_sidebar_item: &PageShellSidebarItem,
) -> Result<(), String> {
    Arc::make_mut(&mut cache.user_context).set_page_bookmarked(
        &cache.space_id,
        page_block_id,
        is_favorited,
    )?;
    let favorites = cache
        .page_shell
        .sidebar_sections
        .iter_mut()
        .find(|section| {
            section.identity.as_ref() == Some(&PageShellSidebarSectionIdentity::Favorites)
        })
        .ok_or_else(|| "Notion page shell did not include its Favorites section".to_string())?;
    favorites.items.retain(|item| {
        item.identity
            .as_ref()
            .and_then(PageShellNodeIdentity::block_id)
            != Some(page_block_id)
    });
    if is_favorited {
        favorites.items.insert(0, page_sidebar_item.clone());
    }
    Ok(())
}

pub(in super::super) fn find_sidebar_item<'a>(
    sections: &'a [PageShellSidebarSection],
    block_id: &str,
) -> Option<&'a PageShellSidebarItem> {
    sections
        .iter()
        .flat_map(|section| &section.items)
        .find_map(|item| find_sidebar_item_in_tree(item, block_id))
}

fn find_sidebar_item_in_tree<'a>(
    item: &'a PageShellSidebarItem,
    block_id: &str,
) -> Option<&'a PageShellSidebarItem> {
    if item
        .identity
        .as_ref()
        .and_then(PageShellNodeIdentity::block_id)
        == Some(block_id)
    {
        return Some(item);
    }
    item.children
        .iter()
        .find_map(|child| find_sidebar_item_in_tree(child, block_id))
}
