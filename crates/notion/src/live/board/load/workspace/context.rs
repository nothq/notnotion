use super::{
    find_sidebar_item, navigation_bootstrap_key, request_navigation_bootstrap, Arc, BoardTarget,
    HashMap, LiveWorkspaceCache, LiveWorkspaceContext, Mutex, NavigationBootstrap,
    NavigationBootstrapCache, NavigationBootstrapKey, NavigationBootstrapPriority,
    PageShellNodeIdentity, PageShellSidebarItem, PageShellSnapshot, UserContext, VecDeque,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

pub(super) struct LiveWorkspaceContextInput<'a> {
    pub(super) space_id: String,
    pub(super) page_block_id: String,
    pub(super) root_type: &'a str,
    pub(super) board_url: &'a str,
    pub(super) user_context: Arc<UserContext>,
    pub(super) page_shell: PageShellSnapshot,
    pub(super) page_title: &'a str,
    pub(super) workspace_cache: Option<Arc<Mutex<LiveWorkspaceCache>>>,
    pub(super) navigation_key: NavigationBootstrapKey,
    pub(super) initial_bootstrap: NavigationBootstrap,
}

impl LiveWorkspaceContext {
    pub(super) fn new(input: LiveWorkspaceContextInput<'_>) -> Result<Self, String> {
        let LiveWorkspaceContextInput {
            space_id,
            page_block_id,
            root_type,
            board_url,
            user_context,
            page_shell,
            page_title,
            workspace_cache,
            navigation_key,
            initial_bootstrap,
        } = input;
        let page_sidebar_item = page_sidebar_item(
            &page_shell,
            &page_block_id,
            root_type,
            board_url,
            page_title,
        )?;
        let quick_find_records =
            super::super::super::quick_find_records::QuickFindRecords::for_scope(
                &user_context.user_id,
                &space_id,
            );
        if workspace_cache.is_none() {
            quick_find_records.merge_hydrated_root(
                initial_bootstrap.response.record_map()?,
                &page_block_id,
            )?;
        }
        let workspace_cache = match workspace_cache {
            Some(workspace_cache) => workspace_cache,
            None => new_workspace_cache(
                space_id,
                user_context,
                page_shell,
                navigation_key,
                initial_bootstrap,
            ),
        };
        Ok(Self {
            board_url: board_url.to_string(),
            page_block_id,
            workspace_cache,
            page_sidebar_item,
            quick_find_records,
        })
    }

    pub(crate) fn prefetch_notion_workspace_with_session(
        &self,
        session: &NotionDesktopSession,
        board_url: &str,
    ) -> Result<(), NotionLiveError> {
        let board_target = BoardTarget::parse(board_url)?;
        if board_target.collection_view_block_id == self.page_block_id {
            return Ok(());
        }
        let _ = self.load_navigation_bootstrap(
            session,
            &board_target,
            NavigationBootstrapPriority::Prefetch,
        )?;
        Ok(())
    }

    pub(crate) fn invalidate_navigation_bootstrap(
        &self,
        page_block_id: &str,
    ) -> Result<(), String> {
        self.workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?
            .navigation_bootstraps
            .invalidate_page(page_block_id);
        Ok(())
    }

    pub(crate) fn invalidate_current_navigation_bootstrap(&self) -> Result<(), String> {
        self.invalidate_navigation_bootstrap(&self.page_block_id)
    }

    pub(super) fn load_navigation_bootstrap(
        &self,
        session: &NotionDesktopSession,
        board_target: &BoardTarget,
        priority: NavigationBootstrapPriority,
    ) -> Result<Option<NavigationBootstrap>, NotionLiveError> {
        let quick_find_generation = self
            .quick_find_records
            .preview_root_generation(&board_target.collection_view_block_id)?;
        let cache_snapshot = self.workspace_cache()?;
        let key = navigation_bootstrap_key(
            &cache_snapshot.user_context.user_id,
            &cache_snapshot.space_id,
            board_target,
        );
        let cell = {
            let mut cache = cache_snapshot
                .shared
                .lock()
                .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
            cache.navigation_bootstraps.acquire(key.clone(), priority)
        };
        let Some(cell) = cell else {
            return Ok(None);
        };
        let result = cell
            .get_or_init(|| request_navigation_bootstrap(session, board_target))
            .clone();
        let bootstrap = match result {
            Ok(bootstrap) => bootstrap,
            Err(error) => {
                cache_snapshot
                    .shared
                    .lock()
                    .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?
                    .navigation_bootstraps
                    .discard_if_same(&key, &cell);
                return Err(error);
            }
        };
        if !bootstrap.is_cacheable_for(&key) {
            cache_snapshot
                .shared
                .lock()
                .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?
                .navigation_bootstraps
                .discard_if_same(&key, &cell);
        }
        if bootstrap.active_user_id != cache_snapshot.user_context.user_id {
            return Err(NotionLiveError::Fatal(
                "Notion Desktop active user changed while loading a page".to_string(),
            ));
        }
        self.quick_find_records.merge_hydrated_root_at_generation(
            bootstrap.response.record_map()?,
            &board_target.collection_view_block_id,
            quick_find_generation,
        )?;
        Ok(Some(bootstrap))
    }
}

fn page_sidebar_item(
    page_shell: &PageShellSnapshot,
    page_block_id: &str,
    root_type: &str,
    board_url: &str,
    page_title: &str,
) -> Result<PageShellSidebarItem, String> {
    let identity = match root_type {
        "page" | "transcription" => PageShellNodeIdentity::Page {
            block_id: page_block_id.to_string(),
        },
        "collection_view" | "collection_view_page" => PageShellNodeIdentity::Database {
            block_id: page_block_id.to_string(),
        },
        root_type => return Err(format!("unsupported Notion root block type {root_type}")),
    };
    Ok(
        find_sidebar_item(&page_shell.sidebar_sections, page_block_id)
            .cloned()
            .unwrap_or_else(|| PageShellSidebarItem {
                identity: Some(identity),
                title: page_title.to_string(),
                icon: page_shell.page_icon.clone(),
                active: true,
                target_board_url: Some(board_url.to_string()),
                children: Vec::new(),
                child_block_ids: Vec::new(),
                unresolved_child_block_ids: Vec::new(),
                sidebar_children_resolved: false,
            }),
    )
}

fn new_workspace_cache(
    space_id: String,
    user_context: Arc<UserContext>,
    page_shell: PageShellSnapshot,
    navigation_key: NavigationBootstrapKey,
    initial_bootstrap: NavigationBootstrap,
) -> Arc<Mutex<LiveWorkspaceCache>> {
    let mut navigation_bootstraps = NavigationBootstrapCache::default();
    navigation_bootstraps.seed(navigation_key, initial_bootstrap);
    let quick_find_breadcrumbs_by_block_id = Arc::new(
        crate::live::board::context::sidebar_breadcrumbs(&page_shell),
    );
    Arc::new(Mutex::new(LiveWorkspaceCache {
        space_id,
        user_context,
        page_shell,
        quick_find_breadcrumbs_by_block_id,
        sidebar_hydrated: false,
        favorite_worker_running: false,
        authoritative_favorites_by_page: HashMap::new(),
        pending_favorite_mutations: VecDeque::new(),
        custom_emoji_library: None,
        custom_emoji_library_revision: 0,
        custom_emoji_library_loading_revision: None,
        custom_emoji_library_changed: Arc::new(std::sync::Condvar::new()),
        custom_emoji_library_failure: None,
        navigation_bootstraps,
    }))
}
