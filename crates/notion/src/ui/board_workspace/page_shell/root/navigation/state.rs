use crate::model::{CardPageCodeSettingsCapability, NotionLaunchRoute};
use crate::ui::surface::{NotionNavigationHistoryUpdate, PageMutationAuthoritySnapshot};
use crate::ui::{AppearanceMode, Arc, BoardSnapshot, CardPage, NotionWorkspaceApi, Viewport};

/// A page id and the authoritative page reconciled for it.
type PageAuthority = (String, Arc<CardPage>);

pub(super) struct NotionWorkspaceReplacement {
    pub(super) board: BoardSnapshot,
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) code_settings: CardPageCodeSettingsCapability,
    pub(super) route: NotionLaunchRoute,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) viewport: Viewport,
    pub(super) page_authority: Option<PageAuthority>,
}

pub(crate) enum NotionNavigationStart {
    Started,
    AlreadyShown,
    AlreadyPending,
}

pub(super) struct NotionNavigationCompletion {
    pub(super) board_url: String,
    pub(super) route: NotionLaunchRoute,
    pub(super) request_id: u64,
    pub(super) page_authorities: PageMutationAuthoritySnapshot,
    pub(super) history_update: NotionNavigationHistoryUpdate,
}

impl NotionNavigationCompletion {
    pub(super) fn reconcile_page_authority(
        &self,
        page: Option<&mut CardPage>,
        mutations: &mut crate::ui::surface::PageMutationCoordinator,
    ) -> Option<PageAuthority> {
        page.map(|page| {
            let authority = self.page_authorities.token_for(&page.block_id);
            let page_id = page.block_id.clone();
            let authority = mutations.reconcile_external_load(page, authority);
            (page_id, authority)
        })
    }
}
