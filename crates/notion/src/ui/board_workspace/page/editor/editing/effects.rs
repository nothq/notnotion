use std::ops::Range;

use gpui::{ClipboardItem, Entity};
use gpui_components::text_input::{TextInput, TextInputSnapshot};

use super::super::history::PageHistoryRestoreState;
use super::super::rich_text::PageWriteTextProjection;
use super::super::{PageMutationPlan, PagePendingCrossBlockComposition};
use crate::model::{
    CardPageCodeLanguage, CardPageCodeWrap, EditPageBlockTextRequest, PageCodeBlockSourceText,
    PageMutation,
};
use crate::ui::CardPage;

pub(crate) struct PageEditTransition<T> {
    result: T,
    effects: Vec<PageEditEffect>,
}

impl<T> PageEditTransition<T> {
    pub(super) fn new(result: T, effects: Vec<PageEditEffect>) -> Self {
        Self { result, effects }
    }

    pub(super) fn into_parts(self) -> (T, Vec<PageEditEffect>) {
        (self.result, self.effects)
    }
}

pub(crate) enum PageEditEffect {
    Write(PageEditWriteEffect),
    Host(PageEditHostEffect),
    Editor(PageEditorEffect),
    WriteClipboard(ClipboardItem),
    ReplaceLoadedPage(CardPage),
    FocusBlock {
        block_id: String,
        offset: usize,
    },
    ReconcileTextMenus {
        block_id: String,
        snapshot: TextInputSnapshot,
        change_requires_notify: bool,
    },
    UpdateSlashMenu {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    OpenRichTextLinkInput,
    Error(String),
    Notify,
}

pub(crate) enum PageEditWriteEffect {
    ApplyMutationPlan(PageMutationPlan),
    EnqueueMutation {
        page_id: String,
        mutation: PageMutation,
    },
    EnqueueMutationWithProjection {
        page_id: String,
        mutation: PageMutation,
        projection: PageWriteTextProjection,
    },
    EnqueueRichTextEdit(EditPageBlockTextRequest),
    EnqueueCompletedCrossBlockComposition {
        page_id: String,
        survivor_id: String,
        removed_ids: Vec<String>,
        mutation: PageMutation,
    },
}

pub(crate) enum PageEditHostEffect {
    MarkPageHasContent(String),
    OpenPageMention {
        block_id: String,
        trigger_offset: usize,
        query: String,
    },
    ReplaceTextBlockWithCode {
        page: CardPage,
        block_index: usize,
        source_text: PageCodeBlockSourceText,
    },
    StageCodeSettings(PageCodeSettingsStage),
    RestorePageHistoryState(PageHistoryRestoreState),
}

pub(crate) enum PageCodeSettingsStage {
    Language(CardPageCodeLanguage),
    Wrap(CardPageCodeWrap),
}

pub(crate) enum PageEditorEffect {
    SimpleTable(super::super::simple_table::PageSimpleTableEditorEffect),
    CompleteComposer(crate::ui::board_workspace::page::commands::state::PageComposerCompletion),
    FinishCrossBlockReplacement {
        removed_ids: Vec<String>,
    },
    BeginCrossBlockComposition {
        removed_ids: Vec<String>,
        pending: Box<PagePendingCrossBlockComposition>,
        survivor_id: String,
        marked_range: Range<usize>,
    },
    FinishMultilinePaste,
    FinishMarkdownDivider {
        block_id: String,
    },
    FinishEditableMarkdown {
        block_id: String,
    },
    SynchronizeTextInputSelections {
        excluded_block_id: Option<String>,
    },
    CloseRichTextDialog,
    DismissRichTextColorDialog,
    ClearBlockContextMenu,
    ClearBlockContextMenuAndSelection,
    FinishTextBlockCodeReplacement {
        source_block_id: String,
    },
    ClearPageHistoryFocus,
    ClearRemovedBlockEditorState {
        removed_ids: Vec<String>,
    },
    RemoveBlockInput {
        block_id: String,
    },
    DeferInputSelection {
        input: Entity<TextInput>,
        range: Range<usize>,
        reversed: bool,
    },
}
