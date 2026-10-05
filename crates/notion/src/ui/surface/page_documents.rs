use crate::ui::{CardPeekState, LoadedCardPage};

mod hydration;
mod layout;
mod reveal;

pub(crate) use hydration::{
    PageHydrationCompletion, PageHydrationEffect, PageHydrationEvent, PageHydrationJob,
    PageHydrationReadiness, PageHydrationRequest, PageHydrationRetry,
};

use hydration::PageHydrationController;
pub(crate) use layout::{PageDocumentLayoutInvalidation, PageDocumentLayoutTarget};

/// The loaded documents shown by the workspace and its selected-page panel.
/// Editing tools share this registry instead of maintaining separate page authority.
#[derive(Default)]
pub(crate) struct PageDocuments {
    pub(crate) standalone: Option<LoadedCardPage>,
    pub(crate) selected_page: Option<CardPeekState>,
    pub(crate) selected_page_history: Vec<CardPeekState>,
    hydration: PageHydrationController,
}

impl PageDocuments {
    pub(crate) fn visible_page_mutation_snapshot(&self) -> Option<crate::ui::CardPage> {
        match self.selected_page.as_ref() {
            Some(CardPeekState::Loaded(page)) => Some(page.data.page.clone()),
            Some(CardPeekState::Loading { .. } | CardPeekState::Error { .. }) => None,
            None => self.standalone.as_ref().map(|page| page.data.page.clone()),
        }
    }

    pub(crate) fn new(standalone: Option<LoadedCardPage>, hydrate: bool) -> Self {
        Self {
            standalone,
            hydration: PageHydrationController::new(hydrate),
            ..Self::default()
        }
    }

    pub(crate) fn request_hydration(&mut self) {
        self.hydration.request();
    }

    pub(crate) fn prepare_hydration(
        &mut self,
        workspace_api: Option<crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>>,
        authority: Option<super::PageMutationRunToken>,
        readiness: PageHydrationReadiness,
    ) -> Option<PageHydrationJob> {
        self.hydration.prepare(
            self.standalone.as_ref(),
            workspace_api,
            authority,
            readiness,
        )
    }

    pub(crate) fn complete_hydration(
        &mut self,
        completion: PageHydrationCompletion,
        current_api: Option<crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>>,
        readiness: PageHydrationReadiness,
    ) -> PageHydrationEffect {
        self.hydration
            .complete(self.standalone.as_ref(), completion, current_api, readiness)
    }

    pub(crate) fn wait_to_retry_hydration(
        &mut self,
        request: PageHydrationRequest,
    ) -> PageHydrationRetry {
        self.hydration.wait_to_retry(request)
    }

    pub(crate) fn resume_hydration(
        &mut self,
        request: &PageHydrationRequest,
        current_api: Option<&crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>>,
    ) -> bool {
        self.hydration
            .resume(self.standalone.as_ref(), request, current_api)
    }

    pub(crate) fn reset_selected_page(&mut self) {
        self.selected_page = None;
        self.selected_page_history.clear();
    }
}
