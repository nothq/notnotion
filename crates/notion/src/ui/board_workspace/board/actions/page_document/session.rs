use crate::ui::board_workspace::PageMutationAction;
use crate::ui::surface::{PageDocuments, PageEditorState, PageMutationCoordinator};

use super::action::{
    PageCreationHostEffect, PageDocumentAction, PageDocumentEffect, PageDocumentInternalAction,
    PageDocumentWorkspaceFailure, PageOpenCompletion, PageOpenHostEffect,
};

pub(super) struct PageDocumentSession<'a> {
    documents: &'a mut PageDocuments,
    editor: &'a mut PageEditorState,
    mutations: &'a mut PageMutationCoordinator,
}

impl<'a> PageDocumentSession<'a> {
    pub(super) fn new(
        documents: &'a mut PageDocuments,
        editor: &'a mut PageEditorState,
        mutations: &'a mut PageMutationCoordinator,
    ) -> Self {
        Self {
            documents,
            editor,
            mutations,
        }
    }

    pub(super) fn reduce(&mut self, action: PageDocumentAction) -> Vec<PageDocumentEffect> {
        match action {
            PageDocumentAction::OpenBoardItem(item) => vec![PageDocumentEffect::Open(
                PageOpenHostEffect::ResolveBoardItem(item),
            )],
            PageDocumentAction::OpenCard(card) => vec![PageDocumentEffect::Open(
                PageOpenHostEffect::ForwardOrCapture(card),
            )],
            PageDocumentAction::CreatePrimary => vec![PageDocumentEffect::Creation(
                PageCreationHostEffect::ResolvePrimary,
            )],
            PageDocumentAction::CreateInColumn(column_index) => vec![PageDocumentEffect::Creation(
                PageCreationHostEffect::ResolveColumn(column_index),
            )],
            PageDocumentAction::NavigateBack => {
                navigation_effects(PageDocumentInternalAction::CommitNavigateBack)
            }
            PageDocumentAction::CloseSelected => {
                navigation_effects(PageDocumentInternalAction::CommitCloseSelected)
            }
            PageDocumentAction::ReplaceLoaded(replacement) => {
                vec![PageDocumentEffect::ReplaceLoaded(replacement)]
            }
            PageDocumentAction::AdvanceAuthority { page_id, authority } => {
                self.editor
                    .advance_loaded_page_authority(self.documents, &page_id, authority);
                Vec::new()
            }
            PageDocumentAction::Internal(action) => self.reduce_internal(action),
        }
    }

    fn reduce_internal(&mut self, action: PageDocumentInternalAction) -> Vec<PageDocumentEffect> {
        match action {
            PageDocumentInternalAction::PrepareOpen(target) => self.prepare_open(target),
            PageDocumentInternalAction::Opened(completion) => self.finish_open(*completion),
            PageDocumentInternalAction::InstallArchivedPage { target, page } => {
                self.install_archived_page(target.block_id, *page)
            }
            PageDocumentInternalAction::ReportMissingOpenSource { target, archived } => vec![
                PageDocumentEffect::PrintError(if archived {
                    "missing archived page"
                } else {
                    "missing workspace api and archived page"
                }),
                PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(
                    PageDocumentInternalAction::MarkOpenError {
                        target,
                        restore_projection: archived,
                    },
                ))),
            ],
            PageDocumentInternalAction::MarkOpenError {
                target,
                restore_projection,
            } => self.mark_open_error(target.block_id, target.title, restore_projection),
            PageDocumentInternalAction::PrepareFailedOpenRestore(block_id) => vec![
                PageDocumentEffect::ClearShareDialog,
                PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(
                    PageDocumentInternalAction::CommitFailedOpenRestore(block_id),
                ))),
            ],
            PageDocumentInternalAction::CommitFailedOpenRestore(block_id) => {
                self.restore_failed_open(&block_id)
            }
            PageDocumentInternalAction::CommitNavigateBack => {
                self.documents.navigate_selected_page_back();
                completed_navigation_effects()
            }
            PageDocumentInternalAction::CommitCloseSelected => {
                self.documents.close_selected_page();
                completed_navigation_effects()
            }
            PageDocumentInternalAction::FinishCreation(completion) => {
                finish_page_creation(*completion)
            }
            PageDocumentInternalAction::ShowCreationError(message) => {
                vec![PageDocumentEffect::Creation(
                    PageCreationHostEffect::ShowError(message),
                )]
            }
        }
    }

    fn prepare_open(&mut self, target: super::action::PageOpenTarget) -> Vec<PageDocumentEffect> {
        if self
            .documents
            .opening_switches_page_projection(&target.block_id)
        {
            self.editor.clear_projection_state();
        }
        self.documents
            .remember_selected_page_before_open(&target.block_id);
        vec![PageDocumentEffect::Open(PageOpenHostEffect::Begin(target))]
    }

    fn finish_open(&mut self, completion: PageOpenCompletion) -> Vec<PageDocumentEffect> {
        if !self.documents.selected_page_matches(&completion.block_id) {
            return Vec::new();
        }
        match completion.result {
            Ok(mut page) => {
                let authority = self
                    .mutations
                    .reconcile_external_load(&mut page, completion.authority);
                let loaded = self
                    .editor
                    .loaded_card_page_with_authority_and_disclosure(page, authority);
                self.documents.install_selected_page(loaded);
                vec![
                    PageDocumentEffect::ReconcileFocus,
                    PageDocumentEffect::Notify,
                ]
            }
            Err(failure) => vec![PageDocumentEffect::HandleWorkspaceFailure(Box::new(
                PageDocumentWorkspaceFailure {
                    operation: "page loading failed",
                    failure,
                    continuation: Some(PageDocumentAction::Internal(
                        PageDocumentInternalAction::PrepareFailedOpenRestore(completion.block_id),
                    )),
                },
            ))],
        }
    }

    fn install_archived_page(
        &mut self,
        block_id: String,
        page: crate::model::CardPage,
    ) -> Vec<PageDocumentEffect> {
        if !self.documents.selected_page_matches(&block_id) {
            return Vec::new();
        }
        let loaded = self.editor.loaded_card_page_with_disclosure(page);
        self.documents.install_selected_page(loaded);
        vec![
            PageDocumentEffect::PageMutation(PageMutationAction::RestoreVisibleProjection),
            PageDocumentEffect::ReconcileFocus,
            PageDocumentEffect::Notify,
        ]
    }

    fn mark_open_error(
        &mut self,
        block_id: String,
        title: String,
        restore_projection: bool,
    ) -> Vec<PageDocumentEffect> {
        if !self.documents.mark_selected_page_error(block_id, title) {
            return Vec::new();
        }
        let mut effects = Vec::new();
        if restore_projection {
            effects.push(PageDocumentEffect::PageMutation(
                PageMutationAction::RestoreVisibleProjection,
            ));
        }
        effects.push(PageDocumentEffect::ReconcileFocus);
        effects.push(PageDocumentEffect::Notify);
        effects
    }

    fn restore_failed_open(&mut self, block_id: &str) -> Vec<PageDocumentEffect> {
        if !self.documents.restore_page_before_failed_open(block_id) {
            return Vec::new();
        }
        vec![
            PageDocumentEffect::PageMutation(PageMutationAction::RestoreVisibleProjection),
            PageDocumentEffect::ReconcileFocus,
            PageDocumentEffect::Notify,
        ]
    }
}

fn navigation_effects(commit: PageDocumentInternalAction) -> Vec<PageDocumentEffect> {
    vec![
        PageDocumentEffect::ClearSelectionUi,
        PageDocumentEffect::PageMutation(PageMutationAction::CaptureVisibleProjection),
        PageDocumentEffect::Continue(Box::new(PageDocumentAction::Internal(commit))),
    ]
}

fn completed_navigation_effects() -> Vec<PageDocumentEffect> {
    vec![
        PageDocumentEffect::PageMutation(PageMutationAction::RestoreVisibleProjection),
        PageDocumentEffect::ResetComposer,
        PageDocumentEffect::ReconcileFocus,
        PageDocumentEffect::Notify,
    ]
}

fn finish_page_creation(
    completion: super::action::PageCreationCompletion,
) -> Vec<PageDocumentEffect> {
    match completion.result {
        Ok(block_id) => vec![PageDocumentEffect::Creation(
            PageCreationHostEffect::Insert {
                column_index: completion.column_index,
                block_id,
                column_title: completion.column_title,
            },
        )],
        Err(failure) => vec![PageDocumentEffect::HandleWorkspaceFailure(Box::new(
            PageDocumentWorkspaceFailure {
                operation: "page creation failed",
                failure,
                continuation: None,
            },
        ))],
    }
}
