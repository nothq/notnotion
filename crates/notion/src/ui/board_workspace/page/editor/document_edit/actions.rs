use std::{cell::RefCell, rc::Rc};

use gpui::{App, ClipboardItem};
use gpui_components::text_input::{
    TextInputPreMutationAction, TextInputPreMutationActionHandler, TextInputPreMutationDecision,
    TextInputReplacementOrigin, TextInputSnapshot,
};

use super::super::editing::{PageEditEffect, PageEditSession};
use super::replacement::{
    PreparedCrossBlockPageComposition, PreparedMultilinePageTextPaste, PreparedPageTextReplacement,
};
use super::CrossBlockPageTextSelection;
use crate::model::PageTextSelectionAction;

pub(in super::super) enum PageTextPreMutationAction {
    Prepare(PageTextPreMutationRequest),
    Apply(Box<PreparedPageTextAction>),
}

type PageTextPreMutationSender = dyn Fn(PageTextPreMutationAction, &mut App);

#[derive(Clone)]
pub(in super::super) struct PageTextPreMutationActionSink {
    send: Rc<PageTextPreMutationSender>,
}

impl PageTextPreMutationActionSink {
    pub(in super::super) fn new(
        send: impl Fn(PageTextPreMutationAction, &mut App) + 'static,
    ) -> Self {
        Self {
            send: Rc::new(send),
        }
    }

    fn emit(&self, action: PageTextPreMutationAction, cx: &mut App) {
        (self.send)(action, cx);
    }
}

pub(in super::super) struct PageTextPreMutationRequest {
    pub(in super::super) block_id: String,
    pub(in super::super) action: TextInputPreMutationAction,
    pub(in super::super) response: Rc<RefCell<Option<PageTextPreMutationResponse>>>,
}

pub(in super::super) enum PageTextPreMutationResponse {
    Continue,
    Handled {
        clipboard: Option<ClipboardItem>,
        prepared: Option<Box<PreparedPageTextAction>>,
    },
}

pub(in super::super) enum PreparedPageTextAction {
    Replacement(PreparedPageTextReplacement),
    Composition(PreparedCrossBlockPageComposition),
    MultilinePaste(PreparedMultilinePageTextPaste),
}

impl PreparedPageTextAction {
    pub(in super::super) fn apply(self, session: &mut PageEditSession<'_>) {
        match self {
            Self::Replacement(replacement) => replacement.apply(session),
            Self::Composition(composition) => composition.apply(session),
            Self::MultilinePaste(paste) => paste.apply(session),
        }
    }
}

pub(in super::super) fn page_block_on_pre_mutation_action(
    block_id: String,
    actions: PageTextPreMutationActionSink,
) -> TextInputPreMutationActionHandler {
    Rc::new(move |action, _window, cx| {
        let response = Rc::new(RefCell::new(None));
        actions.emit(
            PageTextPreMutationAction::Prepare(PageTextPreMutationRequest {
                block_id: block_id.clone(),
                action,
                response: response.clone(),
            }),
            cx,
        );
        let Some(response) = response.borrow_mut().take() else {
            return TextInputPreMutationDecision::Continue;
        };
        match response {
            PageTextPreMutationResponse::Continue => TextInputPreMutationDecision::Continue,
            PageTextPreMutationResponse::Handled {
                clipboard,
                prepared,
            } => {
                if let Some(item) = clipboard {
                    cx.write_to_clipboard(item);
                }
                if let Some(prepared) = prepared {
                    let actions = actions.clone();
                    cx.defer(move |cx| {
                        actions.emit(PageTextPreMutationAction::Apply(prepared), cx);
                    });
                }
                TextInputPreMutationDecision::Handled
            }
        }
    })
}

impl PageEditSession<'_> {
    pub(in super::super) fn prepare_page_text_pre_mutation(
        &mut self,
        block_id: &str,
        action: TextInputPreMutationAction,
    ) -> PageTextPreMutationResponse {
        if let TextInputPreMutationAction::Paste {
            snapshot,
            text,
            item,
            ..
        } = &action
        {
            if text.contains('\n') {
                if item.text().is_none() {
                    return self.page_text_pre_mutation_error(
                        "Multiline paste requires plain text.".to_string(),
                    );
                }
                return match self.prepare_multiline_page_text_paste(
                    block_id,
                    snapshot,
                    text.clone(),
                ) {
                    Ok(prepared) => PageTextPreMutationResponse::Handled {
                        clipboard: None,
                        prepared: Some(Box::new(PreparedPageTextAction::MultilinePaste(prepared))),
                    },
                    Err(error) => self.page_text_pre_mutation_error(error),
                };
            }
        }

        let snapshot = action_snapshot(&action);
        let selection = match self.prepare_cross_block_page_text_selection(block_id, snapshot) {
            Ok(Some(selection)) => selection,
            Ok(None) => return PageTextPreMutationResponse::Continue,
            Err(()) => return PageTextPreMutationResponse::handled(),
        };
        let prepared = match prepare_selection_action(selection, action) {
            Ok(prepared) => prepared,
            Err(error) => return self.page_text_pre_mutation_error(error),
        };
        PageTextPreMutationResponse::Handled {
            clipboard: prepared.clipboard,
            prepared: prepared.action.map(Box::new),
        }
    }

    fn page_text_pre_mutation_error(&mut self, error: String) -> PageTextPreMutationResponse {
        self.effects.push(PageEditEffect::Error(error));
        self.effects.push(PageEditEffect::Notify);
        PageTextPreMutationResponse::handled()
    }
}

impl PageTextPreMutationResponse {
    fn handled() -> Self {
        Self::Handled {
            clipboard: None,
            prepared: None,
        }
    }
}

struct PreparedSelectionAction {
    clipboard: Option<ClipboardItem>,
    action: Option<PreparedPageTextAction>,
}

fn prepare_selection_action(
    selection: CrossBlockPageTextSelection,
    action: TextInputPreMutationAction,
) -> Result<PreparedSelectionAction, String> {
    match action {
        TextInputPreMutationAction::Copy { .. } => Ok(PreparedSelectionAction {
            clipboard: Some(selection.clipboard_item()?),
            action: None,
        }),
        TextInputPreMutationAction::Cut { .. } => {
            let clipboard = selection.clipboard_item()?;
            let replacement =
                selection.prepare_replacement(String::new(), PageTextSelectionAction::Cut)?;
            Ok(PreparedSelectionAction {
                clipboard: Some(clipboard),
                action: Some(PreparedPageTextAction::Replacement(replacement)),
            })
        }
        TextInputPreMutationAction::Paste { text, item, .. } => {
            if item.text().is_none() {
                return Err("Cross-block paste requires plain text.".to_string());
            }
            let prepared = if text.contains('\n') {
                PreparedPageTextAction::MultilinePaste(selection.prepare_multiline_paste(text)?)
            } else {
                PreparedPageTextAction::Replacement(
                    selection.prepare_replacement(text, PageTextSelectionAction::Paste)?,
                )
            };
            Ok(PreparedSelectionAction {
                clipboard: None,
                action: Some(prepared),
            })
        }
        TextInputPreMutationAction::Replace {
            text,
            origin: TextInputReplacementOrigin::ImeMarked,
            ..
        } => prepare_composition(selection, text),
        TextInputPreMutationAction::Replace { text, .. } => {
            if text.contains('\n') {
                return Err(
                    "Cross-block replacement currently supports one line of text.".to_string(),
                );
            }
            Ok(PreparedSelectionAction {
                clipboard: None,
                action: Some(PreparedPageTextAction::Replacement(
                    selection.prepare_replacement(text, PageTextSelectionAction::TextMutation)?,
                )),
            })
        }
    }
}

fn prepare_composition(
    selection: CrossBlockPageTextSelection,
    text: String,
) -> Result<PreparedSelectionAction, String> {
    if text.contains('\n') {
        return Err("Cross-block composition currently supports one line of text.".to_string());
    }
    let action = if text.is_empty() {
        PreparedPageTextAction::Replacement(
            selection.prepare_replacement(text, PageTextSelectionAction::TextMutation)?,
        )
    } else {
        PreparedPageTextAction::Composition(selection.prepare_composition(text)?)
    };
    Ok(PreparedSelectionAction {
        clipboard: None,
        action: Some(action),
    })
}

fn action_snapshot(action: &TextInputPreMutationAction) -> &TextInputSnapshot {
    match action {
        TextInputPreMutationAction::Copy { snapshot }
        | TextInputPreMutationAction::Cut { snapshot }
        | TextInputPreMutationAction::Paste { snapshot, .. }
        | TextInputPreMutationAction::Replace { snapshot, .. } => snapshot,
    }
}
