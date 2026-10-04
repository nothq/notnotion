use std::{rc::Rc, sync::Arc};

use gpui_components::text_input::{TextInputPreMutationDecision, TextInputVisualLine};

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::surface::{
    PageSimpleTableCellFocusMode, PageSimpleTableCellGeneration,
    PageSimpleTableCellReplacementPlan, PageSimpleTableRuntime,
};
use crate::ui::view_actions::{ViewActionSink, ViewNotifier};

use super::super::render::{PageRenderAction, PageTableHistoryRestore, PageTableRenderAction};
use super::super::simple_table::navigation::{
    simple_table_navigation_target, PageSimpleTableCellNavigation,
};
use super::super::simple_table::PageSimpleTableEditAction;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageSimpleTableInputContext {
    data: Arc<crate::ui::LoadedCardPageData>,
    tables: PageSimpleTableRuntime,
    actions: ViewActionSink<PageRenderAction>,
}

impl PageSimpleTableInputContext {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        data: Arc<crate::ui::LoadedCardPageData>,
        tables: PageSimpleTableRuntime,
        actions: ViewActionSink<PageRenderAction>,
    ) -> Self {
        Self {
            data,
            tables,
            actions,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn page_id(&self) -> &str {
        &self.data.page.block_id
    }

    pub(in crate::ui::board_workspace::page::editor) fn actions(
        &self,
    ) -> ViewActionSink<PageRenderAction> {
        self.actions.clone()
    }
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_change(
    page_id: String,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    actions: ViewActionSink<PageRenderAction>,
) -> gpui_components::text_input::TextInputStateChange {
    Rc::new(move |snapshot, window, cx| {
        actions.emit(
            PageRenderAction::Table(PageTableRenderAction::Edit(
                PageSimpleTableEditAction::Changed {
                    page_id: page_id.clone(),
                    address: address.clone(),
                    generation,
                    snapshot,
                },
            )),
            window,
            cx,
        );
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_pre_mutation(
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    tables: PageSimpleTableRuntime,
    notifier: ViewNotifier,
) -> gpui_components::text_input::TextInputPreMutationActionHandler {
    Rc::new(move |action, _, cx| {
        let plan = PageSimpleTableCellReplacementPlan::from_action(action);
        let error = {
            let mut editor = tables.editor().borrow_mut();
            let Some(active) = editor
                .as_mut()
                .filter(|active| active.address == address && active.generation == generation)
            else {
                return TextInputPreMutationDecision::Continue;
            };
            match plan {
                Ok(plan) => {
                    active.pending_replacement = plan;
                    None
                }
                Err(error) => {
                    active.pending_replacement = None;
                    Some(error)
                }
            }
        };
        if let Some(error) = error {
            println!("notnotion: {error}");
            notifier.notify(cx);
        }
        TextInputPreMutationDecision::Continue
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_selection_change(
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    actions: ViewActionSink<PageRenderAction>,
) -> gpui_components::text_input::TextInputStateChange {
    Rc::new(move |snapshot, window, cx| {
        actions.emit(
            PageRenderAction::Table(PageTableRenderAction::SelectionChanged {
                address: address.clone(),
                generation,
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_navigation(
    context: PageSimpleTableInputContext,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    navigation: PageSimpleTableCellNavigation,
) -> gpui_components::text_input::TextInputKeyAction {
    Rc::new(move |snapshot, modifiers, window, cx| {
        let context = context.clone();
        let address = address.clone();
        cx.defer_in(window, move |input, window, cx| {
            if !input.focus_handle_clone().is_focused(window)
                || snapshot.is_composing
                || modifiers.shift
                || !context.tables.editor().matches(&address, generation)
            {
                return;
            }
            let Some(target) = simple_table_navigation_target(&context.data, &address, navigation)
            else {
                return;
            };
            context.actions.emit(
                PageRenderAction::Table(PageTableRenderAction::Edit(
                    PageSimpleTableEditAction::Activate {
                        page_id: context.data.page.block_id.clone(),
                        address: target,
                        focus: simple_table_navigation_focus(navigation),
                    },
                )),
                window,
                cx,
            );
        });
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_tab(
    context: PageSimpleTableInputContext,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
) -> gpui_components::text_input::TextInputKeyAction {
    Rc::new(move |snapshot, modifiers, window, cx| {
        if snapshot.is_composing || !context.tables.editor().matches(&address, generation) {
            return;
        }
        let navigation = if modifiers.shift {
            PageSimpleTableCellNavigation::Previous
        } else {
            PageSimpleTableCellNavigation::Next
        };
        let Some(target) = simple_table_navigation_target(&context.data, &address, navigation)
        else {
            return;
        };
        context.actions.emit(
            PageRenderAction::Table(PageTableRenderAction::Edit(
                PageSimpleTableEditAction::Activate {
                    page_id: context.data.page.block_id.clone(),
                    address: target,
                    focus: simple_table_navigation_focus(navigation),
                },
            )),
            window,
            cx,
        );
        cx.stop_propagation();
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_vertical_navigation(
    context: PageSimpleTableInputContext,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    navigation: PageSimpleTableCellNavigation,
) -> gpui_components::text_input::TextInputVerticalBoundaryAction {
    Rc::new(move |boundary, modifiers, window, cx| {
        if boundary.snapshot.is_composing
            || modifiers.shift
            || !context.tables.editor().matches(&address, generation)
        {
            return false;
        }
        let line = match navigation {
            PageSimpleTableCellNavigation::Up => TextInputVisualLine::Last,
            PageSimpleTableCellNavigation::Down => TextInputVisualLine::First,
            _ => unreachable!("vertical table navigation requires Up or Down"),
        };
        let Some(target) = simple_table_navigation_target(&context.data, &address, navigation)
        else {
            return false;
        };
        context.actions.emit(
            PageRenderAction::Table(PageTableRenderAction::Edit(
                PageSimpleTableEditAction::Activate {
                    page_id: context.data.page.block_id.clone(),
                    address: target,
                    focus: PageSimpleTableCellFocusMode::Vertical {
                        window_x: boundary.window_x,
                        line,
                    },
                },
            )),
            window,
            cx,
        );
        true
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_history(
    context: PageSimpleTableInputContext,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    redo: bool,
) -> gpui_components::text_input::TextInputKeyAction {
    Rc::new(move |snapshot, _, window, cx| {
        if !snapshot.is_composing {
            context.actions.emit(
                PageRenderAction::Table(PageTableRenderAction::RestoreHistory(
                    PageTableHistoryRestore {
                        page_id: context.data.page.block_id.clone(),
                        address: address.clone(),
                        generation,
                        cursor: snapshot.cursor,
                        redo,
                    },
                )),
                window,
                cx,
            );
        }
    })
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_layout_change(
    page_id: String,
    address: CardPageSimpleTableCellAddress,
    actions: ViewActionSink<PageRenderAction>,
) -> gpui_components::text_input::TextInputLayoutChange {
    Rc::new(move |window, cx| {
        actions.emit(
            PageRenderAction::Table(PageTableRenderAction::Edit(
                PageSimpleTableEditAction::Remeasure {
                    page_id: page_id.clone(),
                    address: address.clone(),
                },
            )),
            window,
            cx,
        );
    })
}

const fn simple_table_navigation_focus(
    navigation: PageSimpleTableCellNavigation,
) -> PageSimpleTableCellFocusMode {
    match navigation {
        PageSimpleTableCellNavigation::Left | PageSimpleTableCellNavigation::Previous => {
            PageSimpleTableCellFocusMode::End
        }
        PageSimpleTableCellNavigation::Right
        | PageSimpleTableCellNavigation::Next
        | PageSimpleTableCellNavigation::Up
        | PageSimpleTableCellNavigation::Down => PageSimpleTableCellFocusMode::Start,
    }
}
