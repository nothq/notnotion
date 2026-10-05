use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::PageMutationAction;
use std::sync::Arc;

use gpui::Context;

use super::super::SurfaceState;
use super::cache_merge::merge_cached_page_edits;
use crate::model::CardPage;

impl SurfaceState {
    pub(super) fn reconcile_optimistic_cached_page(
        &mut self,
        replacement: &mut Self,
        cached_page_edits: Option<(CardPage, CardPage)>,
        cx: &mut Context<Self>,
    ) {
        let mut live_page = replacement
            .page_documents
            .standalone
            .as_ref()
            .map(|page| page.data.page.clone());
        let live_authority = live_page
            .as_mut()
            .map(|page| self.page_mutations.reconcile_external_load(page, None));
        let optimistic = cached_page_edits
            .and_then(|edits| {
                self.merge_cached_page_handoff(replacement, live_authority.as_deref(), edits, cx)
            })
            .or(live_page);
        if let Some(page) = optimistic {
            let authority = live_authority.unwrap_or_else(|| Arc::new(page.clone()));
            replacement.install_optimistic_cached_page(page, authority, cx);
        }
    }

    fn merge_cached_page_handoff(
        &mut self,
        replacement: &Self,
        live_page: Option<&CardPage>,
        cached_page_edits: (CardPage, CardPage),
        cx: &mut Context<Self>,
    ) -> Option<CardPage> {
        let (baseline, edited) = cached_page_edits;
        let live_page = live_page.filter(|live| live.block_id == edited.block_id);
        let mut visual = replacement
            .page_documents
            .merge_cached_page_visual(&baseline, edited, live_page);
        let Some(live) = live_page else {
            if self.page_mutations.has_pending_writes(&baseline.block_id) {
                self.dispatch_page_mutation_action(
                    PageMutationAction::BeginQueuedReplay {
                        page_id: baseline.block_id.clone(),
                        baseline: Arc::new(baseline),
                        workspace_api: replacement.notion_startup.workspace_api(),
                    },
                    cx,
                );
            }
            return visual;
        };
        match self
            .page_mutations
            .rebase_queued_page_writes(&baseline, live)
        {
            Ok(Some(replayed)) => {
                let authority = Arc::new(live.clone());
                let optimistic = Arc::new(replayed.clone());
                assert!(self.page_mutations.accept_idle_projection(
                    &baseline.block_id,
                    authority,
                    Some(optimistic),
                ));
                visual = visual
                    .and_then(|page| merge_cached_page_edits(live, &replayed, &page))
                    .or(Some(replayed));
            }
            Ok(None) => {}
            Err(error) => {
                self.print_notion_error(format!("could not replay cached page edits: {error}"));
                self.dispatch_page_mutation_action(
                    PageMutationAction::BeginQueuedReplay {
                        page_id: baseline.block_id.clone(),
                        baseline: Arc::new(baseline),
                        workspace_api: replacement.notion_startup.workspace_api(),
                    },
                    cx,
                );
            }
        }
        visual
    }

    fn install_optimistic_cached_page(
        &mut self,
        page: CardPage,
        authority: Arc<CardPage>,
        cx: &mut Context<Self>,
    ) {
        if self
            .page_documents
            .standalone
            .as_ref()
            .is_some_and(|current| current.data.page.block_id == page.block_id)
        {
            self.dispatch_page_document_action(
                PageDocumentAction::replace_loaded_and_authority(page, authority),
                cx,
            );
            return;
        }
        if self.page_documents.standalone.is_some() {
            return;
        }
        self.board.page_content = Some(page.clone());
        let expanded = self
            .page_editor
            .page_toggle_disclosure
            .expanded_block_ids(&page.block_id);
        self.page_documents.standalone = Some(crate::ui::LoadedCardPage::with_authority(
            page, authority, expanded,
        ));
        self.page_documents.request_hydration();
    }
}
