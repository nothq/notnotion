use super::{
    load, load_page_presence, Arc, BoardSnapshot, BoardTarget, FavoriteMutationContext,
    LiveBoardMutator, LiveWorkspaceCacheSnapshot, LiveWorkspaceContext, Map, PagePresenceSnapshot,
    PageShellSearchResult, PendingFavoriteMutation,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

mod custom_emoji;
mod sidebar;
#[cfg(test)]
mod tests;

use sidebar::{
    cached_page_is_favorited, collect_local_search_page, effective_favorite_state,
    set_cached_page_favorited, synchronize_sidebar_item_active_state,
};
pub(super) use sidebar::{find_sidebar_item, sidebar_breadcrumbs};

impl LiveWorkspaceContext {
    pub(crate) fn child_board_url(&self, child_block_id: &str) -> Result<String, String> {
        Ok(BoardTarget::parse(&self.board_url)?.child_url(child_block_id))
    }

    pub(crate) fn cached_space_id(&self) -> Result<String, String> {
        self.workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())
            .map(|cache| cache.space_id.clone())
    }

    pub(crate) fn cached_search_context(
        &self,
    ) -> Result<super::LiveWorkspaceSearchContext, String> {
        let cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        Ok(super::LiveWorkspaceSearchContext {
            space_id: cache.space_id.clone(),
            active_user_id: cache.user_context.user_id.clone(),
            active_user_profile: cache.user_context.profile.clone(),
            time_zone: cache.user_context.time_zone.clone(),
            sidebar_breadcrumbs_by_block_id: cache.quick_find_breadcrumbs_by_block_id.clone(),
            quick_find_records: self.quick_find_records.clone(),
        })
    }

    pub(crate) fn load_quick_find_preview(
        &self,
        session: &NotionDesktopSession,
        block_id: &str,
    ) -> Result<crate::model::CardPage, NotionLiveError> {
        self.quick_find_records.load_preview(session, block_id)
    }

    pub(crate) fn invalidate_quick_find_record_root(&self, block_id: &str) -> Result<(), String> {
        self.quick_find_records.invalidate_root(block_id)
    }

    pub(crate) fn cached_local_search_pages(&self) -> Result<Vec<PageShellSearchResult>, String> {
        let cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        let mut pages = Vec::new();
        let mut seen = std::collections::HashSet::new();
        collect_local_search_page(
            &self.page_sidebar_item,
            &cache.quick_find_breadcrumbs_by_block_id,
            &mut seen,
            &mut pages,
        );
        for item in cache
            .page_shell
            .sidebar_sections
            .iter()
            .flat_map(|section| &section.items)
        {
            collect_local_search_page(
                item,
                &cache.quick_find_breadcrumbs_by_block_id,
                &mut seen,
                &mut pages,
            );
        }
        Ok(pages)
    }

    pub(crate) fn hydrate_sidebar(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<(), NotionLiveError> {
        let snapshot = self.workspace_cache()?;
        if snapshot.sidebar_hydrated {
            return Ok(());
        }
        let board_target = BoardTarget::parse(&self.board_url)?;
        let sidebar = load::load_sidebar(
            session,
            &snapshot.space_id,
            &snapshot.user_context,
            &board_target,
            &Map::new(),
        )?;
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        cache.page_shell.builtin_links = sidebar.builtin_links;
        cache.page_shell.sidebar_sections = sidebar.sections;
        cache.quick_find_breadcrumbs_by_block_id = Arc::new(sidebar_breadcrumbs(&cache.page_shell));
        cache.sidebar_hydrated = true;
        Ok(())
    }

    pub(crate) fn load_page_presence(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<PagePresenceSnapshot, NotionLiveError> {
        let cache = self.workspace_cache()?;
        load_page_presence(
            session,
            &self.page_block_id,
            &cache.space_id,
            &cache.user_context,
        )
    }

    pub(crate) fn load_sidebar_calendar(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<crate::model::LoadSidebarCalendarResult, NotionLiveError> {
        let cache = self.workspace_cache()?;
        load::load_sidebar_calendar_for_context(session, &cache.space_id, &cache.user_context)
    }

    pub(crate) fn cache_page_favorited(
        &self,
        is_favorited: bool,
        mutation_context: FavoriteMutationContext,
    ) -> Result<bool, String> {
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        let authoritative_favorite = cached_page_is_favorited(&cache, &self.page_block_id);
        cache
            .authoritative_favorites_by_page
            .entry(self.page_block_id.clone())
            .or_insert(authoritative_favorite);
        set_cached_page_favorited(
            &mut cache,
            &self.page_block_id,
            is_favorited,
            &self.page_sidebar_item,
        )?;
        cache
            .pending_favorite_mutations
            .push_back(PendingFavoriteMutation {
                page_block_id: self.page_block_id.clone(),
                is_favorited,
                page_sidebar_item: self.page_sidebar_item.clone(),
                mutation_context,
            });
        let should_start_worker = !cache.favorite_worker_running;
        cache.favorite_worker_running = true;
        Ok(should_start_worker)
    }

    pub(crate) fn flush_favorite_mutations(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<Vec<String>, NotionLiveError> {
        let mut errors = Vec::new();
        while let Some(pending) = self.take_pending_favorite_mutation()? {
            self.flush_favorite_mutation(session, pending, &mut errors)?;
        }
        Ok(errors)
    }

    fn take_pending_favorite_mutation(&self) -> Result<Option<PendingFavoriteMutation>, String> {
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        let pending = cache.pending_favorite_mutations.pop_front();
        if pending.is_none() {
            cache.favorite_worker_running = false;
        }
        Ok(pending)
    }

    fn flush_favorite_mutation(
        &self,
        session: &NotionDesktopSession,
        pending: PendingFavoriteMutation,
        errors: &mut Vec<String>,
    ) -> Result<(), NotionLiveError> {
        let mutation_result = LiveBoardMutator::set_favorited_with_context(
            session,
            &pending.mutation_context,
            pending.is_favorited,
        );
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        match mutation_result {
            Err(NotionLiveError::Session(error)) => {
                cache.pending_favorite_mutations.push_front(pending);
                return Err(NotionLiveError::Session(error));
            }
            Err(error) => {
                errors.push(format!("failed to update notion favorite state: {error}"));
            }
            Ok(()) => {
                cache
                    .authoritative_favorites_by_page
                    .insert(pending.page_block_id.clone(), pending.is_favorited);
            }
        }
        let (effective_favorite, page_sidebar_item) = effective_favorite_state(&cache, &pending);
        set_cached_page_favorited(
            &mut cache,
            &pending.page_block_id,
            effective_favorite,
            &page_sidebar_item,
        )
        .map_err(NotionLiveError::Fatal)
    }

    pub(crate) fn synchronize_snapshot(&self, snapshot: &mut BoardSnapshot) -> Result<(), String> {
        let cache = self.workspace_cache()?;
        let space_view_id = cache.user_context.space_view_id_for_space(&cache.space_id);
        snapshot.is_favorited = space_view_id.is_some_and(|space_view_id| {
            cache
                .user_context
                .is_page_bookmarked(space_view_id, &self.page_block_id)
        });
        let page_shell = snapshot.page_shell.as_mut().ok_or_else(|| {
            "Notion workspace snapshot did not include its page shell".to_string()
        })?;
        let current_sidebar_sections = std::mem::take(&mut page_shell.sidebar_sections);
        page_shell
            .builtin_links
            .clone_from(&cache.page_shell.builtin_links);
        page_shell
            .sidebar_sections
            .clone_from(&cache.page_shell.sidebar_sections);
        page_shell.merge_sidebar_hydration_from(&current_sidebar_sections);
        for section in &mut page_shell.sidebar_sections {
            for item in &mut section.items {
                synchronize_sidebar_item_active_state(item, &self.page_block_id, &self.board_url);
            }
        }
        Ok(())
    }

    pub(super) fn workspace_cache(&self) -> Result<LiveWorkspaceCacheSnapshot, String> {
        let cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        Ok(LiveWorkspaceCacheSnapshot {
            space_id: cache.space_id.clone(),
            user_context: cache.user_context.clone(),
            page_shell: cache.page_shell.clone(),
            sidebar_hydrated: cache.sidebar_hydrated,
            shared: self.workspace_cache.clone(),
        })
    }
}
