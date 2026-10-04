use std::sync::Arc;

use crate::model::{
    LoadSidebarChildrenRequest, PageShellNodeIdentity, PageShellSidebarItem, PageShellSnapshot,
};
use crate::ui::{BoardSnapshot, NotionSidebarNodeKey, NotionSidebarSectionKey, NotionWorkspaceApi};

use super::resource_job::{
    SidebarChildrenCompletion, SidebarResourceContext, SidebarResourceEffect,
    SidebarResourceFailure, SidebarResourceFailureRecovery, SidebarResourceJob,
    SidebarWorkspaceCompletion,
};
use super::{NotionSidebarState, NotionSidebarTreeState};

pub(crate) enum SidebarWorkspacePreparation {
    None,
    PageHydrationOnly,
    Start {
        job: SidebarResourceJob,
        board_url: String,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct SidebarWorkspaceReadiness {
    pub(crate) surface_active: bool,
    pub(crate) startup_ready: bool,
    pub(crate) has_page_shell: bool,
}

impl NotionSidebarState {
    pub(crate) fn prepare_workspace_resource(
        &mut self,
        readiness: SidebarWorkspaceReadiness,
        context: &SidebarResourceContext,
    ) -> SidebarWorkspacePreparation {
        if self.tree != NotionSidebarTreeState::Idle
            || !readiness.startup_ready
            || !readiness.has_page_shell
        {
            return SidebarWorkspacePreparation::PageHydrationOnly;
        }
        if !readiness.surface_active {
            return SidebarWorkspacePreparation::None;
        }
        let (Some(workspace_api), Some(board_url)) = (&context.workspace_api, &context.board_url)
        else {
            self.tree = NotionSidebarTreeState::Failed;
            return SidebarWorkspacePreparation::None;
        };
        self.tree = NotionSidebarTreeState::Loading;
        SidebarWorkspacePreparation::Start {
            job: SidebarResourceJob::workspace(board_url.clone(), workspace_api.clone()),
            board_url: board_url.clone(),
            workspace_api: workspace_api.clone(),
        }
    }

    pub(crate) fn prepare_expanded_children(
        &mut self,
        page_shell: Option<&PageShellSnapshot>,
        context: &SidebarResourceContext,
    ) -> Vec<SidebarResourceJob> {
        self.expanded_nodes
            .clone()
            .into_iter()
            .filter_map(|key| self.prepare_child_resource(key, page_shell, context))
            .collect()
    }

    pub(crate) fn prepare_child_resource(
        &mut self,
        key: NotionSidebarNodeKey,
        page_shell: Option<&PageShellSnapshot>,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJob> {
        if !self.loading_nodes.insert(key.clone()) {
            return None;
        }
        let Some(current_board_url) = context.board_url.clone() else {
            self.loading_nodes.remove(&key);
            return None;
        };
        let Some(request) = sidebar_children_request(page_shell, &key, current_board_url) else {
            self.loading_nodes.remove(&key);
            return None;
        };
        let Some(workspace_api) = context.workspace_api.clone() else {
            self.loading_nodes.remove(&key);
            return None;
        };
        Some(SidebarResourceJob::children(
            request,
            workspace_api,
            context.navigation_request_id,
            key,
        ))
    }

    pub(super) fn complete_workspace_resource(
        &mut self,
        completion: SidebarWorkspaceCompletion,
        board: &mut BoardSnapshot,
        context: &SidebarResourceContext,
    ) -> SidebarResourceEffect {
        if self.tree != NotionSidebarTreeState::Loading
            || context.board_url.as_deref() != Some(completion.board_url.as_str())
            || context
                .workspace_api
                .as_ref()
                .is_none_or(|current| !Arc::ptr_eq(current, &completion.workspace_api))
        {
            return SidebarResourceEffect::None;
        }
        let result = completion.result.and_then(|()| {
            completion
                .workspace_api
                .synchronize_workspace_snapshot(board)
        });
        if let Err(error) = result {
            return SidebarResourceEffect::Failure(Box::new(SidebarResourceFailure {
                operation: "sidebar hydration failed",
                error,
                recovery: SidebarResourceFailureRecovery::Workspace,
            }));
        }
        self.tree = NotionSidebarTreeState::Loaded;
        self.reconcile_node_state(board.page_shell.as_ref());
        let jobs = self.prepare_expanded_children(board.page_shell.as_ref(), context);
        self.rebuild_rows(board.page_shell.as_ref());
        SidebarResourceEffect::SpawnAndNotify(jobs)
    }

    pub(super) fn complete_children_resource(
        &mut self,
        completion: SidebarChildrenCompletion,
        board: &mut BoardSnapshot,
        context: &SidebarResourceContext,
    ) -> SidebarResourceEffect {
        if context.navigation_request_id != completion.navigation_request_id
            || context.board_url.as_deref() != Some(completion.board_url.as_str())
            || !self.loading_nodes.remove(&completion.node_key)
        {
            return SidebarResourceEffect::None;
        }
        match completion.result {
            Ok(result) => {
                let Some(page_shell) = board.page_shell.as_mut() else {
                    return SidebarResourceEffect::None;
                };
                let Some(item) = sidebar_item_mut(page_shell, &completion.node_key) else {
                    return SidebarResourceEffect::None;
                };
                item.children = result.children;
                item.sidebar_children_resolved = result.sidebar_children_resolved;
                if result.sidebar_children_resolved {
                    item.unresolved_child_block_ids.clear();
                }
            }
            Err(error) => {
                return SidebarResourceEffect::Failure(Box::new(SidebarResourceFailure {
                    operation: "sidebar child loading failed",
                    error,
                    recovery: SidebarResourceFailureRecovery::Children,
                }));
            }
        }
        self.reconcile_node_state(board.page_shell.as_ref());
        self.rebuild_rows(board.page_shell.as_ref());
        SidebarResourceEffect::Notify
    }

    pub(crate) fn reconcile_node_state(&mut self, page_shell: Option<&PageShellSnapshot>) {
        let Some(page_shell) = page_shell else {
            self.expanded_nodes.clear();
            self.loading_nodes.clear();
            return;
        };
        self.loading_nodes.retain(|key| {
            sidebar_item(page_shell, key).is_some_and(|item| !item.sidebar_children_resolved)
        });
        let loading_nodes = self.loading_nodes.clone();
        self.expanded_nodes.retain(|key| {
            sidebar_item(page_shell, key).is_some_and(|item| {
                loading_nodes.contains(key)
                    || !item.sidebar_children_resolved
                    || !item.children.is_empty()
            })
        });
    }
}

fn sidebar_children_request(
    page_shell: Option<&PageShellSnapshot>,
    key: &NotionSidebarNodeKey,
    current_board_url: String,
) -> Option<LoadSidebarChildrenRequest> {
    let item = sidebar_item(page_shell?, key)?;
    if item.sidebar_children_resolved {
        return None;
    }
    let parent_block_id = match item.identity.as_ref()? {
        PageShellNodeIdentity::Page { block_id } | PageShellNodeIdentity::Database { block_id } => {
            block_id.clone()
        }
        PageShellNodeIdentity::Home
        | PageShellNodeIdentity::Chat
        | PageShellNodeIdentity::Meetings
        | PageShellNodeIdentity::Inbox => return None,
    };
    Some(LoadSidebarChildrenRequest {
        parent_block_id,
        current_board_url,
    })
}

fn sidebar_item<'a>(
    page_shell: &'a PageShellSnapshot,
    key: &NotionSidebarNodeKey,
) -> Option<&'a PageShellSidebarItem> {
    let section_key = sidebar_node_section_key(key);
    let section = page_shell
        .sidebar_sections
        .iter()
        .enumerate()
        .find(|(index, section)| {
            NotionSidebarSectionKey::new(section.identity.as_ref(), *index, &section.title)
                == *section_key
        })?
        .1;
    match key {
        NotionSidebarNodeKey::Identity { node, .. } => section
            .items
            .iter()
            .find_map(|item| sidebar_item_by_identity(item, node)),
        NotionSidebarNodeKey::Legacy { path, .. } => sidebar_item_by_path(&section.items, path),
    }
}

fn sidebar_item_mut<'a>(
    page_shell: &'a mut PageShellSnapshot,
    key: &NotionSidebarNodeKey,
) -> Option<&'a mut PageShellSidebarItem> {
    let section_key = sidebar_node_section_key(key);
    let section = page_shell
        .sidebar_sections
        .iter_mut()
        .enumerate()
        .find(|(index, section)| {
            NotionSidebarSectionKey::new(section.identity.as_ref(), *index, &section.title)
                == *section_key
        })?
        .1;
    match key {
        NotionSidebarNodeKey::Identity { node, .. } => section
            .items
            .iter_mut()
            .find_map(|item| sidebar_item_by_identity_mut(item, node)),
        NotionSidebarNodeKey::Legacy { path, .. } => {
            sidebar_item_by_path_mut(&mut section.items, path)
        }
    }
}

fn sidebar_node_section_key(key: &NotionSidebarNodeKey) -> &NotionSidebarSectionKey {
    match key {
        NotionSidebarNodeKey::Identity { section, .. }
        | NotionSidebarNodeKey::Legacy { section, .. } => section,
    }
}

fn sidebar_item_by_identity<'a>(
    item: &'a PageShellSidebarItem,
    identity: &PageShellNodeIdentity,
) -> Option<&'a PageShellSidebarItem> {
    if item.identity.as_ref() == Some(identity) {
        return Some(item);
    }
    item.children
        .iter()
        .find_map(|child| sidebar_item_by_identity(child, identity))
}

fn sidebar_item_by_identity_mut<'a>(
    item: &'a mut PageShellSidebarItem,
    identity: &PageShellNodeIdentity,
) -> Option<&'a mut PageShellSidebarItem> {
    if item.identity.as_ref() == Some(identity) {
        return Some(item);
    }
    item.children
        .iter_mut()
        .find_map(|child| sidebar_item_by_identity_mut(child, identity))
}

fn sidebar_item_by_path<'a>(
    items: &'a [PageShellSidebarItem],
    path: &[usize],
) -> Option<&'a PageShellSidebarItem> {
    let mut path = path.iter();
    let mut item = items.get(*path.next()?)?;
    for child_index in path {
        item = item.children.get(*child_index)?;
    }
    Some(item)
}

fn sidebar_item_by_path_mut<'a>(
    items: &'a mut [PageShellSidebarItem],
    path: &[usize],
) -> Option<&'a mut PageShellSidebarItem> {
    let mut path = path.iter();
    let mut item = items.get_mut(*path.next()?)?;
    for child_index in path {
        item = item.children.get_mut(*child_index)?;
    }
    Some(item)
}
