use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::surface::{
    PageEditorState, PageHydrationEffect, PageHydrationEvent, PageHydrationReadiness,
    PageMutationCoordinator,
};
use crate::ui::{Context, SurfaceState};

impl SurfaceState {
    pub(crate) fn start_notion_page_hydration(&mut self, cx: &mut Context<Self>) {
        let page_id = self
            .page_documents
            .standalone
            .as_ref()
            .map(|page| page.data.page.block_id.as_str());
        let readiness = page_hydration_readiness(
            cx.has_active_drag(),
            page_id,
            &self.page_mutations,
            &self.page_editor,
        );
        let authority =
            page_id.and_then(|page_id| self.page_mutations.latest_committed_token(page_id));
        let workspace_api = self.notion_startup.workspace_api();
        let Some(job) = self
            .page_documents
            .prepare_hydration(workspace_api, authority, readiness)
        else {
            return;
        };
        job.spawn(cx, handle_page_hydration_event);
    }
}

fn handle_page_hydration_event(
    surface: &mut SurfaceState,
    event: PageHydrationEvent,
    cx: &mut Context<SurfaceState>,
) {
    match event {
        PageHydrationEvent::Completed(completion) => {
            let page_id = completion.request.page_id.as_str();
            let readiness = page_hydration_readiness(
                cx.has_active_drag(),
                Some(page_id),
                &surface.page_mutations,
                &surface.page_editor,
            );
            let current_api = surface.notion_startup.workspace_api();
            let effect =
                surface
                    .page_documents
                    .complete_hydration(*completion, current_api, readiness);
            match effect {
                PageHydrationEffect::None => {}
                PageHydrationEffect::Loaded(loaded) => {
                    let mut page = loaded.page;
                    let authority = surface
                        .page_mutations
                        .reconcile_external_load(&mut page, loaded.authority);
                    surface.dispatch_page_document_action(
                        PageDocumentAction::replace_loaded_and_authority(page, authority),
                        cx,
                    );
                    cx.notify();
                }
                PageHydrationEffect::ReportFailure(failure) => {
                    if !surface.handle_notion_workspace_failure(
                        "page hydration failed",
                        failure.error,
                        cx,
                    ) {
                        surface
                            .page_documents
                            .wait_to_retry_hydration(failure.request)
                            .spawn(cx, handle_page_hydration_event);
                    }
                    cx.notify();
                }
            }
        }
        PageHydrationEvent::Retry(request) => {
            let current_api = surface.notion_startup.workspace_api();
            if surface
                .page_documents
                .resume_hydration(&request, current_api.as_ref())
            {
                cx.notify();
            }
        }
    }
}

fn page_hydration_readiness(
    active_drag: bool,
    page_id: Option<&str>,
    page_mutations: &PageMutationCoordinator,
    page_editor: &PageEditorState,
) -> PageHydrationReadiness {
    let pending_mutation = page_id.is_some_and(|page_id| {
        page_mutations.has_work(page_id)
            || !page_editor.page_block_compositions.is_empty()
            || page_editor
                .tables
                .editor()
                .has_pending_composition_for_page(page_id)
            || page_editor.page_pending_rich_text_typing.is_some()
            || page_editor.page_pending_rich_text_composition.is_some()
            || page_editor.page_pending_cross_block_composition.is_some()
    });
    PageHydrationReadiness::new(active_drag, pending_mutation)
}
