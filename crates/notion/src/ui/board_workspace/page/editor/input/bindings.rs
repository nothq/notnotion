use std::{collections::HashMap, rc::Rc};

use gpui::Context;
use gpui_components::text_input::{TextInputKeyPreAction, TextInputPreMutationActionHandler};

use super::super::callbacks::{
    page_block_on_cross_selection_line_break, page_block_on_horizontal_selection_collapse,
    page_input_action_sink,
};
use super::super::document_edit::page_block_on_pre_mutation_action;
use super::super::editing::page_text_pre_mutation_action_sink;
use super::super::mention::{page_mention_action_sink, PageMentionAction};
use super::super::navigation::PageBlockNavigation;
use super::super::{LoadedCardPageData, SurfaceState};
use super::behavior::PageBlockInputBehavior;
use super::PageInputAction;
use crate::ui::view_actions::ViewActionSink;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageInputBindings {
    pub(super) actions: ViewActionSink<PageInputAction>,
    pub(super) mention_actions: ViewActionSink<PageMentionAction>,
    synchronous: Rc<HashMap<String, PageInputSynchronousCallbacks>>,
}

#[derive(Clone)]
struct PageInputSynchronousCallbacks {
    pre_mutation: Option<TextInputPreMutationActionHandler>,
    cross_selection_line_break: Option<TextInputKeyPreAction>,
    collapse_left: TextInputKeyPreAction,
    collapse_right: TextInputKeyPreAction,
}

impl PageInputBindings {
    pub(in crate::ui::board_workspace::page::editor) fn capture(
        data: &LoadedCardPageData,
        cx: &Context<SurfaceState>,
    ) -> Self {
        let actions = page_input_action_sink(cx);
        let mention_actions = page_mention_action_sink(cx);
        let pre_mutation_actions = page_text_pre_mutation_action_sink(cx);
        let synchronous = data
            .page
            .blocks
            .iter()
            .filter_map(|block| {
                let editable = block.editable_content()?;
                let block_id = block.block_id.clone();
                let behavior = PageBlockInputBehavior::from_editable(editable);
                let pre_mutation = matches!(
                    behavior,
                    PageBlockInputBehavior::Document | PageBlockInputBehavior::PageLink
                )
                .then(|| {
                    page_block_on_pre_mutation_action(
                        block_id.clone(),
                        pre_mutation_actions.clone(),
                    )
                });
                let cross_selection_line_break =
                    matches!(behavior, PageBlockInputBehavior::Document)
                        .then(|| page_block_on_cross_selection_line_break(block_id.clone(), cx));
                let callbacks = PageInputSynchronousCallbacks {
                    pre_mutation,
                    cross_selection_line_break,
                    collapse_left: page_block_on_horizontal_selection_collapse(
                        block_id.clone(),
                        PageBlockNavigation::Left,
                        cx,
                    ),
                    collapse_right: page_block_on_horizontal_selection_collapse(
                        block_id.clone(),
                        PageBlockNavigation::Right,
                        cx,
                    ),
                };
                Some((block_id, callbacks))
            })
            .collect();
        Self {
            actions,
            mention_actions,
            synchronous: Rc::new(synchronous),
        }
    }

    pub(super) fn pre_mutation(&self, block_id: &str) -> TextInputPreMutationActionHandler {
        self.callbacks(block_id)
            .pre_mutation
            .clone()
            .expect("document input requires its pre-mutation binding")
    }

    pub(super) fn cross_selection_line_break(&self, block_id: &str) -> TextInputKeyPreAction {
        self.callbacks(block_id)
            .cross_selection_line_break
            .clone()
            .expect("document input requires its cross-selection line-break binding")
    }

    pub(super) fn collapse_left(&self, block_id: &str) -> TextInputKeyPreAction {
        self.callbacks(block_id).collapse_left.clone()
    }

    pub(super) fn collapse_right(&self, block_id: &str) -> TextInputKeyPreAction {
        self.callbacks(block_id).collapse_right.clone()
    }

    fn callbacks(&self, block_id: &str) -> &PageInputSynchronousCallbacks {
        self.synchronous
            .get(block_id)
            .expect("editable page block requires captured input callbacks")
    }
}
