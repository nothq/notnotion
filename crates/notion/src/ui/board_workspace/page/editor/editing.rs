mod effects;
mod host;

pub(crate) use effects::{
    PageCodeSettingsStage, PageEditEffect, PageEditHostEffect, PageEditTransition,
    PageEditWriteEffect, PageEditorEffect,
};
pub(super) use host::page_text_pre_mutation_action_sink;

use crate::ui::surface::{PageDocuments, PageEditorState};

/// A short-lived editing transaction over the two owners of page-editing data.
///
/// The session never crosses a GPUI defer boundary. Cross-domain work is emitted
/// as ordered, owned effects and applied by the host after these borrows end.
pub(crate) struct PageEditSession<'a> {
    pub(in crate::ui::board_workspace::page) editor: &'a mut PageEditorState,
    pub(in crate::ui::board_workspace::page) documents: &'a PageDocuments,
    pub(in crate::ui::board_workspace::page) effects: Vec<PageEditEffect>,
}

impl<'a> PageEditSession<'a> {
    pub(crate) fn new(editor: &'a mut PageEditorState, documents: &'a PageDocuments) -> Self {
        Self {
            editor,
            documents,
            effects: Vec::new(),
        }
    }

    pub(crate) fn finish<T>(self, result: T) -> PageEditTransition<T> {
        PageEditTransition::new(result, self.effects)
    }
}
