use gpui::Context;

use crate::ui::board_workspace::{PageFocusSession, PageMutationAction};
use crate::ui::surface::database_view_rows;
use crate::ui::{generated_local_page_id, Arc, NotionWorkspaceApi, SurfaceState};

use super::super::page_creation::{primary_page_creation, PageCreationError, PageCreationState};
use super::super::page_open::card_for_board_item;
use super::action::{
    PageCreationHostEffect, PageCreationJob, PageDocumentAction, PageDocumentEffect,
    PageDocumentHostQueue, PageDocumentHostStep, PageDocumentJob, PageLoadedReplacement,
    PageOpenHostEffect, PageOpenJob,
};
use super::jobs::run_page_document_job;
use super::session::PageDocumentSession;

impl SurfaceState {
    pub(crate) fn dispatch_page_document_action(
        &mut self,
        action: PageDocumentAction,
        cx: &mut Context<Self>,
    ) {
        let mut pending = PageDocumentHostQueue::new(action);
        while let Some(step) = pending.pop() {
            let effect = match step {
                PageDocumentHostStep::Effect(effect) => effect,
                PageDocumentHostStep::Action(action) => {
                    let effects = PageDocumentSession::new(
                        &mut self.page_documents,
                        &mut self.page_editor,
                        &mut self.page_mutations,
                    )
                    .reduce(action);
                    pending.prepend_effects(effects);
                    continue;
                }
            };
            if !apply_page_document_effect(self, effect, &mut pending, cx) {
                return;
            }
        }
    }
}

fn apply_page_document_effect(
    surface: &mut SurfaceState,
    effect: PageDocumentEffect,
    pending: &mut PageDocumentHostQueue,
    cx: &mut Context<SurfaceState>,
) -> bool {
    match effect {
        PageDocumentEffect::Open(effect) => {
            apply_page_open_host_effect(surface, effect, pending, cx)
        }
        PageDocumentEffect::Creation(effect) => {
            apply_page_creation_host_effect(surface, effect, pending, cx)
        }
        PageDocumentEffect::RunBackground(job) => surface.spawn_background_task(
            job,
            cx,
            run_page_document_job,
            SurfaceState::dispatch_page_document_action,
        ),
        PageDocumentEffect::HandleWorkspaceFailure(failure) => {
            if surface.handle_notion_workspace_failure(failure.operation, failure.failure, cx) {
                return false;
            }
            if let Some(continuation) = failure.continuation {
                pending.prepend_action(continuation);
            }
        }
        PageDocumentEffect::PageMutation(action) => {
            surface.dispatch_page_mutation_action(action, cx);
        }
        PageDocumentEffect::ReconcileFocus => {
            PageFocusSession::new(&mut surface.page_editor, &mut surface.page_documents)
                .reconcile_active_projection(cx);
        }
        PageDocumentEffect::ResetComposer => surface.page_editor.reset_page_composer(),
        PageDocumentEffect::ClearSelectionUi => {
            surface.notion_chrome.share_dialog = None;
            surface
                .comments
                .dismiss_notion_comments_panel_without_notify();
        }
        PageDocumentEffect::ClearShareDialog => surface.notion_chrome.share_dialog = None,
        PageDocumentEffect::ReplaceLoaded(replacement) => {
            replace_loaded_page_at_host(surface, *replacement, cx);
        }
        PageDocumentEffect::Continue(action) => pending.prepend_action(*action),
        PageDocumentEffect::PrintError(error) => surface.print_notion_error(error),
        PageDocumentEffect::Notify => cx.notify(),
    }
    true
}

fn apply_page_open_host_effect(
    surface: &mut SurfaceState,
    effect: PageOpenHostEffect,
    pending: &mut PageDocumentHostQueue,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        PageOpenHostEffect::ResolveBoardItem(item) => {
            pending.prepend_action(PageDocumentAction::OpenCard(card_for_board_item(
                &surface.columns,
                &item,
            )));
        }
        PageOpenHostEffect::ForwardOrCapture(card) => {
            if let Some(page_host) = surface.presentation.page_host.clone() {
                page_host
                    .update(cx, |surface, cx| {
                        surface
                            .dispatch_page_document_action(PageDocumentAction::OpenCard(card), cx);
                    })
                    .expect("inline database page presentation requires its parent surface");
                return;
            }
            pending.prepend_effects(vec![
                PageDocumentEffect::PageMutation(PageMutationAction::CaptureVisibleProjection),
                PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(
                    super::action::PageDocumentInternalAction::PrepareOpen(card.into()),
                ))),
            ]);
        }
        PageOpenHostEffect::Begin(target) => begin_page_open_at_host(surface, target, pending),
    }
}

fn begin_page_open_at_host(
    surface: &mut SurfaceState,
    target: super::action::PageOpenTarget,
    pending: &mut PageDocumentHostQueue,
) {
    surface.page_editor.reset_page_composer();
    surface.notion_chrome.toolbar_dialog = None;
    surface.notion_chrome.inline_toolbar_dialog = None;
    surface.notion_chrome.ai_autofill_dialog = None;
    surface.notion_chrome.share_dialog = None;
    surface
        .comments
        .dismiss_notion_comments_panel_without_notify();
    surface.database_search.close();
    surface.notion_chrome.notion_search_open = false;
    surface
        .page_documents
        .set_selected_page_loading(target.block_id.clone(), target.title.clone());
    surface.board_view.hovered_card = None;
    surface.board_view.hovered_column_header = None;
    let action = if let Some(workspace_api) = surface.notion_startup.workspace_api() {
        let authority = surface
            .page_mutations
            .latest_committed_token(&target.block_id);
        PageDocumentEffect::RunBackground(PageDocumentJob::Open(PageOpenJob {
            workspace_api,
            block_id: target.block_id,
            authority,
        }))
    } else if let Some(pages) = surface.snapshot_pages.as_ref() {
        match pages.get(&target.block_id).cloned() {
            Some(page) => PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(
                super::action::PageDocumentInternalAction::InstallArchivedPage {
                    target,
                    page: Box::new(page),
                },
            ))),
            None => missing_open_source_effect(target, true),
        }
    } else {
        missing_open_source_effect(target, false)
    };
    pending.prepend_effects(vec![action]);
}

fn missing_open_source_effect(
    target: super::action::PageOpenTarget,
    archived: bool,
) -> PageDocumentEffect {
    PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(
        super::action::PageDocumentInternalAction::ReportMissingOpenSource { target, archived },
    )))
}

fn apply_page_creation_host_effect(
    surface: &mut SurfaceState,
    effect: PageCreationHostEffect,
    pending: &mut PageDocumentHostQueue,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        PageCreationHostEffect::ResolvePrimary => {
            pending.prepend_action(primary_page_creation(&surface.columns));
        }
        PageCreationHostEffect::ResolveColumn(column_index) => {
            let Some(column_title) = surface
                .columns
                .get(column_index)
                .map(|column| column.title.clone())
            else {
                return;
            };
            let effect = resolve_page_creation(
                column_index,
                column_title,
                surface.notion_startup.workspace_api(),
                surface.snapshot_pages.is_some(),
            );
            pending.prepend_effects(vec![effect]);
        }
        PageCreationHostEffect::Insert {
            column_index,
            block_id,
            column_title,
        } => {
            let card = PageCreationState::new(
                &mut surface.columns,
                &mut surface.board,
                &mut surface.snapshot_pages,
            )
            .insert(column_index, block_id, column_title);
            let Some(card) = card else {
                return;
            };
            surface.database_view_rows = database_view_rows(&surface.board);
            cx.notify();
            pending.prepend_action(PageDocumentAction::OpenCard(card));
        }
        PageCreationHostEffect::ShowError(message) => {
            PageCreationError::new(message).apply(surface, cx);
        }
    }
}

fn resolve_page_creation(
    column_index: usize,
    column_title: String,
    workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    archived_pages_available: bool,
) -> PageDocumentEffect {
    if let Some(workspace_api) = workspace_api {
        PageDocumentEffect::RunBackground(PageDocumentJob::Create(PageCreationJob {
            workspace_api,
            column_index,
            column_title,
        }))
    } else if !archived_pages_available {
        PageDocumentEffect::Continue(Box::new(PageDocumentAction::creation_error(
            "missing workspace api for new page creation".to_string(),
        )))
    } else {
        PageDocumentEffect::Continue(Box::new(PageDocumentAction::finish_creation(
            column_index,
            generated_local_page_id(),
            column_title,
        )))
    }
}

fn replace_loaded_page_at_host(
    surface: &mut SurfaceState,
    replacement: PageLoadedReplacement,
    cx: &mut Context<SurfaceState>,
) {
    let page = replacement.page;
    surface.page_mutations.capture_optimistic_page(page.clone());
    for card in surface
        .columns
        .iter_mut()
        .flat_map(|column| column.cards.iter_mut())
        .filter(|card| card.block_id == page.block_id)
    {
        card.title.clone_from(&page.title);
    }
    surface.board.synchronize_page_title(&page);
    let installed = surface.page_editor.replace_loaded_page_references(
        &mut surface.page_documents,
        &page,
        replacement.authority.as_ref(),
    );
    if installed.standalone_replaced() {
        surface.board.page_content = Some(page.clone());
    }
    surface.page_editor.reconcile_loaded_page_replacement(
        &mut surface.page_documents,
        &installed,
        cx,
    );
    surface.comments.reconcile_page(&page);
    if let Some(snapshot_pages) = surface.snapshot_pages.as_mut() {
        if snapshot_pages.contains_key(installed.page_id()) {
            snapshot_pages.insert(installed.page_id().to_string(), page);
        }
    }
}
