use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::PageEditSession;

use super::{
    CardLocation, Context, HoveredCard, HoveredCardAction, MouseMoveEvent, MouseUpEvent,
    MoveCardRequest, Pixels, Point, SurfaceState, Window, DRAG_THRESHOLD,
};
use crate::model::{NotionBoardUrl, NotionCollectionViewId};
use crate::ui::surface::BoardViewState;
use crate::ui::ViewTabKind;

impl BoardViewState {
    pub(crate) fn set_column_header_hover(
        &mut self,
        column_index: usize,
        is_hovered: bool,
    ) -> bool {
        let next = if is_hovered {
            Some(column_index)
        } else if self.hovered_column_header == Some(column_index) {
            None
        } else {
            self.hovered_column_header
        };
        if self.hovered_column_header == next {
            return false;
        }
        self.hovered_column_header = next;
        true
    }

    pub(crate) fn set_card_hover(&mut self, location: CardLocation, is_hovered: bool) -> bool {
        let next = if is_hovered {
            Some(HoveredCard {
                column_index: location.column_index,
                card_index: location.card_index,
                action: None,
            })
        } else if self.hovered_card.is_some_and(|hovered| {
            hovered.column_index == location.column_index
                && hovered.card_index == location.card_index
        }) {
            None
        } else {
            self.hovered_card
        };
        if self.hovered_card == next {
            return false;
        }
        self.hovered_card = next;
        true
    }

    pub(crate) fn set_card_action_hover(
        &mut self,
        location: CardLocation,
        action: HoveredCardAction,
        is_hovered: bool,
    ) -> bool {
        let next = if is_hovered {
            Some(HoveredCard {
                column_index: location.column_index,
                card_index: location.card_index,
                action: Some(action),
            })
        } else if self.hovered_card.is_some_and(|hovered| {
            hovered.column_index == location.column_index
                && hovered.card_index == location.card_index
                && hovered.action == Some(action)
        }) {
            Some(HoveredCard {
                column_index: location.column_index,
                card_index: location.card_index,
                action: None,
            })
        } else {
            self.hovered_card
        };
        if self.hovered_card == next {
            return false;
        }
        self.hovered_card = next;
        true
    }
}

impl SurfaceState {
    pub(crate) fn update_drag_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.database_search.is_filtering() {
            return;
        }
        let Some(mut drag) = self.board_view.drag.take() else {
            return;
        };
        drag.pointer_position = position;
        if !drag.moved
            && ((position.x - drag.pointer_start.x).abs().as_f32() >= DRAG_THRESHOLD
                || (position.y - drag.pointer_start.y).abs().as_f32() >= DRAG_THRESHOLD)
        {
            drag.moved = true;
        }
        let drag_moved = drag.moved;
        self.board_view.drag = Some(drag);
        if drag_moved {
            if let Some((target, maximum)) = self.board_drag_layout().auto_scroll_target(position) {
                self.board_view.set_scroll_x(target, maximum);
            }
        }
        cx.notify();
    }

    pub(crate) fn finish_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.board_view.drag.take() else {
            return;
        };
        if !drag.moved {
            self.dispatch_page_document_action(PageDocumentAction::OpenCard(drag.card), cx);
            return;
        }
        if let Some(target) = self
            .board_drag_layout()
            .drop_target(drag.pointer_position, &drag.card.block_id)
        {
            let source_column_title = self.columns[drag.source_column].title.clone();
            let target_column_title = self.columns[target.column_index].title.clone();
            let before_anchor = self
                .board_drag_layout()
                .move_anchor_before(target, &drag.card.block_id);
            let after_anchor = self
                .board_drag_layout()
                .move_anchor_after(target, &drag.card.block_id)
                .filter(|_| before_anchor.is_none());
            let workspace_api = self.notion_startup.workspace_api();
            let moved_block_id = drag.card.block_id.clone();

            super::board::drag::apply_board_drop(&mut self.columns, drag, target);
            if let Some(workspace_api) = workspace_api {
                self.spawn_background_task(
                    MoveCardRequest::new(
                        moved_block_id,
                        source_column_title,
                        target_column_title,
                        before_anchor,
                        after_anchor,
                    ),
                    cx,
                    move |request| workspace_api.move_card(request),
                    |this, result, cx| {
                        if let Err(error) = result {
                            this.handle_notion_workspace_failure("card move failed", error, cx);
                        }
                    },
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_page_block_hover(event.position, cx);
        let selection_changed = {
            let transition = {
                let mut edit = PageEditSession::new(&mut self.page_editor, &self.page_documents);
                let result = edit.update_page_text_selection_at_position(event.position, None, cx);
                edit.finish(result)
            };
            self.apply_page_edit_transition(transition, cx)
        };
        if selection_changed {
            return;
        }
        if self.board_view.drag.is_none() {
            return;
        }
        self.update_drag_position(event.position, cx);
    }

    pub(crate) fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page_editor.finish_page_text_selection() {
            cx.notify();
            return;
        }
        self.finish_drag(cx);
    }

    #[cfg(test)]
    pub(crate) fn activate_view_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.page_documents.selected_page.is_some() || index >= self.board.view_tabs.len() {
            return;
        }
        let provider_view_id = self.board.view_tabs[index].provider_view_id.clone();
        self.select_database_view(provider_view_id, cx);
    }

    pub(crate) fn select_database_view(
        &mut self,
        provider_view_id: NotionCollectionViewId,
        cx: &mut Context<Self>,
    ) {
        if self.page_documents.selected_page.is_some() {
            return;
        }
        let Some((index, label, active)) = self
            .board
            .view_tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| tab.provider_view_id == provider_view_id)
            .map(|(index, tab)| (index, tab.label.clone(), tab.active))
        else {
            return;
        };
        if active {
            return;
        }

        if let Some(current_board_url) = self.notion_startup.board_url() {
            if self.notion_startup.workspace_api().is_none() {
                return;
            }
            let current_board_url = current_board_url
                .parse::<NotionBoardUrl>()
                .expect("loaded Notion workspace must retain a valid board URL");
            let next_board_url = current_board_url
                .with_collection_view_id(&provider_view_id)
                .as_str()
                .to_string();
            self.open_notion_workspace(next_board_url, label, cx);
            return;
        }

        self.activate_view_tab_locally(index, cx);
    }

    fn activate_view_tab_locally(&mut self, index: usize, cx: &mut Context<Self>) {
        for (tab_index, tab) in self.board.view_tabs.iter_mut().enumerate() {
            tab.active = tab_index == index;
        }
        self.database_filter = crate::ui::surface::DatabaseFilterUiState::for_board(&self.board);
        cx.notify();
    }

    pub(crate) fn handle_card_mouse_down(
        &mut self,
        location: CardLocation,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.page_documents.selected_page.is_some() {
            return;
        }
        self.begin_drag(location.column_index, location.card_index, position, cx);
    }
}

impl crate::ui::BoardSnapshot {
    pub(crate) fn active_view_kind(&self) -> ViewTabKind {
        self.view_tabs
            .iter()
            .find(|tab| tab.active)
            .map(|tab| tab.kind)
            .unwrap_or(ViewTabKind::Board)
    }
}
