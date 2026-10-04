use super::super::{point, px, Context, DragState, Pixels, Point, SurfaceState, ToolbarDialogKind};
use crate::ui::surface::NotionChromeState;

mod page_creation;
mod page_document;
mod page_open;

pub(crate) use page_document::PageDocumentAction;

impl SurfaceState {
    pub(crate) fn begin_drag(
        &mut self,
        column_index: usize,
        card_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.page_documents.selected_page.is_some() {
            return;
        }

        let Some(card) = self
            .columns
            .get(column_index)
            .and_then(|column| column.cards.get(card_index))
            .cloned()
        else {
            return;
        };

        let left = self.page_layout().sidebar_layout_width()
            + self.board_drag_layout().card_left(column_index);
        let top = self.board_drag_layout().card_top(column_index, card_index);

        self.board_view.drag = Some(DragState {
            card,
            source_column: column_index,
            source_index: card_index,
            pointer_start: position,
            pointer_position: position,
            cursor_offset: point(position.x - px(left), position.y - px(top)),
            moved: false,
        });
        cx.notify();
    }

    pub(crate) fn toggle_favorite(&mut self, cx: &mut Context<Self>) {
        let next_favorited = !self.favorited;
        let workspace_api = self.notion_startup.workspace_api();
        let Some(workspace_api) = workspace_api else {
            self.favorited = next_favorited;
            self.board.is_favorited = next_favorited;
            cx.notify();
            return;
        };
        let cache_dispatch = match workspace_api.cache_favorited(next_favorited) {
            Ok(dispatch) => dispatch,
            Err(error) => {
                self.handle_notion_workspace_failure("favorite cache update failed", error, cx);
                return;
            }
        };
        self.favorited = next_favorited;
        self.board.is_favorited = next_favorited;
        match cache_dispatch {
            None => self.spawn_background_task(
                next_favorited,
                cx,
                move |next_favorited| workspace_api.set_favorited(next_favorited),
                |this, result, cx| {
                    if let Err(error) = result {
                        this.handle_notion_workspace_failure("favorite update failed", error, cx);
                    }
                },
            ),
            Some(true) => self.spawn_background_task(
                (),
                cx,
                move |_| workspace_api.flush_favorite_mutations(),
                |this, result, cx| this.finish_favorite_synchronization(result, cx),
            ),
            Some(false) => {}
        }
        cx.notify();
    }

    fn finish_favorite_synchronization(
        &mut self,
        result: crate::model::NotionWorkspaceResult<Vec<String>>,
        cx: &mut Context<Self>,
    ) {
        let errors = match result {
            Ok(errors) => errors,
            Err(error) => {
                if self.handle_notion_workspace_failure(
                    "favorite synchronization failed",
                    error,
                    cx,
                ) {
                    return;
                }
                Vec::new()
            }
        };
        if let Some(api) = self.notion_startup.workspace_api() {
            if let Err(error) = api.synchronize_workspace_snapshot(&mut self.board) {
                if self.handle_notion_workspace_failure(
                    "workspace synchronization failed",
                    error,
                    cx,
                ) {
                    return;
                }
            }
        }
        self.favorited = self.board.is_favorited;
        self.notion_sidebar
            .reconcile_node_state(self.board.page_shell.as_ref());
        self.notion_sidebar
            .rebuild_rows(self.board.page_shell.as_ref());
        if !errors.is_empty() {
            self.print_notion_error(errors.join("; "));
        }
        cx.notify();
    }

    pub(crate) fn open_toolbar_dialog(
        &mut self,
        dialog: ToolbarDialogKind,
        cx: &mut Context<Self>,
    ) {
        self.page_editor.reset_page_composer();
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_chrome.ai_autofill_dialog = None;
        let opening = self.notion_chrome.toolbar_dialog != Some(dialog);
        self.notion_chrome.toolbar_dialog = opening.then_some(dialog);
        if opening && dialog == ToolbarDialogKind::Filter {
            self.database_filter.open_property_picker();
        }
        self.database_search.close();
        self.notion_chrome.notion_search_open = false;
        self.notion_search.input.borrow_mut().take();
        self.board_view.hovered_card = None;
        self.board_view.hovered_column_header = None;
        cx.notify();
    }

    pub(crate) fn toggle_toolbar_search(&mut self, cx: &mut Context<Self>) {
        let search_was_open = self.database_search.open;
        self.page_editor.reset_page_composer();
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_chrome.ai_autofill_dialog = None;
        self.notion_chrome.toolbar_dialog = None;
        self.notion_chrome.inline_toolbar_dialog = None;
        self.notion_chrome.notion_search_open = false;
        self.notion_search.input.borrow_mut().take();
        if search_was_open {
            self.database_search.close();
        } else {
            self.database_search.open();
        }
        self.board_view.hovered_card = None;
        self.board_view.hovered_column_header = None;
        cx.notify();
    }
}

impl NotionChromeState {
    pub(crate) fn close_toolbar_dialog(&mut self, cx: &mut Context<SurfaceState>) {
        if self.toolbar_dialog.is_none() {
            return;
        }
        self.toolbar_dialog = None;
        cx.notify();
    }
}
