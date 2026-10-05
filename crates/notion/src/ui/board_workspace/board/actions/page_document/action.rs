use std::collections::VecDeque;
use std::sync::Arc;

use crate::model::{BoardItem, CardPage, NotionWorkspaceOperationFailure, NotionWorkspaceResult};
use crate::ui::board_workspace::PageMutationAction;
use crate::ui::surface::PageMutationRunToken;
use crate::ui::{Card, NotionWorkspaceApi};

pub(crate) enum PageDocumentAction {
    OpenBoardItem(BoardItem),
    OpenCard(Card),
    CreatePrimary,
    CreateInColumn(usize),
    NavigateBack,
    CloseSelected,
    ReplaceLoaded(Box<PageLoadedReplacement>),
    AdvanceAuthority {
        page_id: String,
        authority: Arc<CardPage>,
    },
    Internal(PageDocumentInternalAction),
}

impl PageDocumentAction {
    pub(crate) fn replace_loaded(page: CardPage) -> Self {
        Self::ReplaceLoaded(Box::new(PageLoadedReplacement {
            page,
            authority: None,
        }))
    }

    pub(crate) fn replace_loaded_and_authority(page: CardPage, authority: Arc<CardPage>) -> Self {
        Self::ReplaceLoaded(Box::new(PageLoadedReplacement {
            page,
            authority: Some(authority),
        }))
    }

    pub(crate) fn finish_creation(
        column_index: usize,
        block_id: String,
        column_title: String,
    ) -> Self {
        Self::Internal(PageDocumentInternalAction::FinishCreation(Box::new(
            PageCreationCompletion {
                column_index,
                column_title,
                result: Ok(block_id),
            },
        )))
    }

    pub(in crate::ui::board_workspace::board::actions) fn creation_error(message: String) -> Self {
        Self::Internal(PageDocumentInternalAction::ShowCreationError(message))
    }
}

pub(crate) struct PageLoadedReplacement {
    pub(super) page: CardPage,
    pub(super) authority: Option<Arc<CardPage>>,
}

pub(crate) enum PageDocumentInternalAction {
    PrepareOpen(PageOpenTarget),
    Opened(Box<PageOpenCompletion>),
    InstallArchivedPage {
        target: PageOpenTarget,
        page: Box<CardPage>,
    },
    ReportMissingOpenSource {
        target: PageOpenTarget,
        archived: bool,
    },
    MarkOpenError {
        target: PageOpenTarget,
        restore_projection: bool,
    },
    PrepareFailedOpenRestore(String),
    CommitFailedOpenRestore(String),
    CommitNavigateBack,
    CommitCloseSelected,
    FinishCreation(Box<PageCreationCompletion>),
    ShowCreationError(String),
}

pub(crate) struct PageOpenTarget {
    pub(super) block_id: String,
    pub(super) title: String,
}

impl From<Card> for PageOpenTarget {
    fn from(card: Card) -> Self {
        Self {
            block_id: card.block_id,
            title: card.title,
        }
    }
}

pub(crate) struct PageOpenCompletion {
    pub(super) block_id: String,
    pub(super) authority: Option<PageMutationRunToken>,
    pub(super) result: NotionWorkspaceResult<CardPage>,
}

pub(crate) struct PageCreationCompletion {
    pub(super) column_index: usize,
    pub(super) column_title: String,
    pub(super) result: NotionWorkspaceResult<String>,
}

pub(super) enum PageDocumentEffect {
    Open(PageOpenHostEffect),
    Creation(PageCreationHostEffect),
    RunBackground(PageDocumentJob),
    HandleWorkspaceFailure(Box<PageDocumentWorkspaceFailure>),
    PageMutation(PageMutationAction),
    ReconcileFocus,
    ResetComposer,
    ClearSelectionUi,
    ClearShareDialog,
    ReplaceLoaded(Box<PageLoadedReplacement>),
    Continue(Box<PageDocumentAction>),
    PrintError(&'static str),
    Notify,
}

pub(super) enum PageOpenHostEffect {
    ResolveBoardItem(BoardItem),
    ForwardOrCapture(Card),
    Begin(PageOpenTarget),
}

pub(super) enum PageCreationHostEffect {
    ResolvePrimary,
    ResolveColumn(usize),
    Insert {
        column_index: usize,
        block_id: String,
        column_title: String,
    },
    ShowError(String),
}

pub(super) enum PageDocumentJob {
    Open(PageOpenJob),
    Create(PageCreationJob),
}

pub(super) struct PageOpenJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) block_id: String,
    pub(super) authority: Option<PageMutationRunToken>,
}

pub(super) struct PageCreationJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) column_index: usize,
    pub(super) column_title: String,
}

pub(super) struct PageDocumentWorkspaceFailure {
    pub(super) operation: &'static str,
    pub(super) failure: NotionWorkspaceOperationFailure,
    pub(super) continuation: Option<PageDocumentAction>,
}

pub(super) enum PageDocumentHostStep {
    Action(PageDocumentAction),
    Effect(PageDocumentEffect),
}

pub(super) struct PageDocumentHostQueue {
    pending: VecDeque<PageDocumentHostStep>,
}

impl PageDocumentHostQueue {
    pub(super) fn new(action: PageDocumentAction) -> Self {
        Self {
            pending: VecDeque::from([PageDocumentHostStep::Action(action)]),
        }
    }

    pub(super) fn pop(&mut self) -> Option<PageDocumentHostStep> {
        self.pending.pop_front()
    }

    pub(super) fn prepend_effects(&mut self, effects: Vec<PageDocumentEffect>) {
        for effect in effects.into_iter().rev() {
            self.pending
                .push_front(PageDocumentHostStep::Effect(effect));
        }
    }

    pub(super) fn prepend_action(&mut self, action: PageDocumentAction) {
        self.pending
            .push_front(PageDocumentHostStep::Action(action));
    }
}
