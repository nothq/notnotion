use crate::ui::board_workspace::{PageEditSession, PageSimpleTableShortcut};
use crate::ui::board_workspace::{ShareEvent, StatusPropertyPickerAction};

use super::{Context, NotionSidebarTab, SurfaceState, Window};
use crate::ui::surface::{NotionNavigationDirection, PageEditorState, SupportedPageBlockAction};

impl SurfaceState {
    pub(crate) fn handle_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_surface_key_down(event, window, cx)
            || self.handle_surface_dialog_key_down(event, window, cx)
        {
            return;
        }
        self.handle_surface_editor_key_down(event, window, cx);
    }

    fn handle_surface_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.notion_startup.cached_workspace_visible()
            || self.handle_notion_search_key_down(event, cx)
        {
            cx.stop_propagation();
            return true;
        }
        if is_platform_shortcut(event, "k")
            && (self.notion_chrome.notion_search_open
                || !self.notion_keyboard_shortcut_dialog_open())
        {
            self.activate_notion_search(cx);
            window.prevent_default();
            cx.stop_propagation();
            return true;
        }
        let navigation_direction = match event.keystroke.key.as_str() {
            "[" if is_platform_shortcut(event, "[") => Some(NotionNavigationDirection::Backward),
            "]" if is_platform_shortcut(event, "]") => Some(NotionNavigationDirection::Forward),
            _ => None,
        };
        if let Some(direction) = navigation_direction {
            if !self.notion_chrome.notion_search_open
                && !self.notion_keyboard_shortcut_dialog_open()
                && self.navigate_notion_workspace_history(direction, cx)
            {
                window.prevent_default();
                cx.stop_propagation();
                return true;
            }
            return false;
        }
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key == "o"
            && modifiers.platform
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.function
            && !modifiers.shift
        {
            crate::ui::board_workspace::activate_sidebar_tab(self, NotionSidebarTab::Chat, cx);
            cx.stop_propagation();
            return true;
        }
        if event.keystroke.key != "escape" || !cx.stop_active_drag(window) {
            return false;
        }
        self.page_editor.cancel_drag();
        cx.notify();
        cx.stop_propagation();
        true
    }

    fn notion_keyboard_shortcut_dialog_open(&self) -> bool {
        self.comments.panel.is_some()
            || self.notion_chrome.share_dialog.is_some()
            || self.notion_chrome.status_property_picker.is_some()
            || self.notion_chrome.date_undated_dialog.is_some()
            || self.notion_chrome.inline_database_view_menu.is_some()
            || self.notion_chrome.inline_toolbar_dialog.is_some()
            || self.notion_chrome.ai_autofill_dialog.is_some()
            || self.notion_chrome.toolbar_dialog.is_some()
            || self.notion_chrome.notion_page_menu_open
            || self.notion_chrome.notion_ai_mode_menu_open
            || self.notion_sidebar.inbox_filter_menu_open
            || self.notion_sidebar.inbox_archive_menu_open
            || self.page_editor.page_rich_text_dialog.is_some()
            || self.page_editor.page_link_icons.picker_is_open()
            || self.page_editor.page_block_context_menu.is_some()
    }

    fn handle_surface_dialog_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let handled = self.comments.handle_notion_comments_key_down(event, cx)
            || (self
                .notion_chrome
                .notion_share_dialog_handles_key_down(event)
                && {
                    self.dispatch_notion_share_event(ShareEvent::Dismiss, cx);
                    true
                })
            || (self
                .notion_chrome
                .status_property_picker_handles_key_down(event)
                && {
                    self.dispatch_status_property_picker_action(
                        StatusPropertyPickerAction::Dismiss,
                        cx,
                    );
                    true
                })
            || self.handle_date_undated_dialog_key_down(event, cx)
            || (self.notion_chrome.toolbar_dialog == Some(crate::ui::ToolbarDialogKind::Filter)
                && self.handle_database_filter_key_down(event, cx));
        if handled {
            cx.stop_propagation();
            return true;
        }
        if self
            .notion_chrome
            .handle_inline_database_view_menu_key_down(event, window, cx)
        {
            return true;
        }
        let handled = self
            .notion_chrome
            .handle_inline_toolbar_dialog_key_down(event, cx)
            || self.notion_chrome.handle_ai_autofill_key_down(event, cx)
            || self.notion_chrome.handle_notion_ai_key_down(event, cx);
        if handled {
            cx.stop_propagation();
        }
        handled
    }

    fn handle_surface_editor_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_supported_page_block_shortcut(event, window, cx)
            || self.handle_page_block_context_menu_shortcut(event, window, cx)
            || if let Some(shortcut) =
                PageSimpleTableShortcut::capture(&self.page_editor.tables, event, window, cx)
            {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut self.page_editor, &self.page_documents);
                    shortcut.apply(&mut edit);
                    edit.finish(())
                };
                self.apply_page_edit_transition(transition, cx);
                true
            } else {
                false
            }
            || {
                let handled = self.page_editor.mention.handle_picker_key_down(event);
                if handled {
                    cx.notify();
                }
                handled
            }
            || {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut self.page_editor, &self.page_documents);
                    let result = edit.handle_page_rich_text_key_down(event, cx);
                    edit.finish(result)
                };
                self.apply_page_edit_transition(transition, cx)
            }
            || self.handle_page_history_key_down(event, cx)
            || self.handle_selected_page_key_down(event, cx)
        {
            cx.stop_propagation();
        }
    }

    fn handle_supported_page_block_shortcut(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        let action = match event.keystroke.key.as_str() {
            "l" if modifiers.platform
                && modifiers.control
                && !modifiers.alt
                && !modifiers.function
                && !modifiers.shift =>
            {
                SupportedPageBlockAction::CopyLink
            }
            "d" if modifiers.platform
                && !modifiers.control
                && !modifiers.alt
                && !modifiers.function
                && !modifiers.shift =>
            {
                SupportedPageBlockAction::Duplicate
            }
            "j" if modifiers.platform
                && !modifiers.control
                && !modifiers.alt
                && !modifiers.function
                && !modifiers.shift =>
            {
                SupportedPageBlockAction::AskAi
            }
            "backspace" | "delete"
                if !modifiers.platform
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.function
                    && !modifiers.shift
                    && (self.page_editor.page_block_context_menu.is_some()
                        || !self.page_editor.page_block_selection.block_ids.is_empty()) =>
            {
                SupportedPageBlockAction::Delete
            }
            _ => return false,
        };
        let Some(block_id) = self.page_editor.keyboard_target() else {
            return false;
        };
        if !self.activate_supported_page_block_action(&block_id, action, cx) {
            return false;
        }
        window.prevent_default();
        true
    }

    fn handle_page_block_context_menu_shortcut(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if !modifiers.platform
            || modifiers.control
            || modifiers.alt
            || modifiers.function
            || !matches!(event.keystroke.key.as_str(), "/" | "?")
        {
            return false;
        }
        let Some(block_id) = self.page_editor.keyboard_target() else {
            return false;
        };
        window.prevent_default();
        self.toggle_page_block_context_menu_from_keyboard(block_id, cx);
        true
    }
}

impl PageEditorState {
    fn cancel_drag(&mut self) {
        let mut drag = self.drag.borrow_mut();
        drag.cancelled = true;
        drag.auto_scroll = None;
        drag.auto_scroll_epoch = drag.auto_scroll_epoch.wrapping_add(1);
        drag.layouts = Default::default();
        drag.target = None;
    }

    fn keyboard_target(&self) -> Option<String> {
        self.page_block_context_menu
            .as_ref()
            .map(|menu| menu.block_id.clone())
            .or_else(|| self.page_block_selection.block_ids.last().cloned())
            .or_else(|| self.active_page_block.clone())
            .or_else(|| self.hovered_page_block.clone())
    }
}

fn is_platform_shortcut(event: &crate::ui::KeyDownEvent, key: &str) -> bool {
    let modifiers = event.keystroke.modifiers;
    event.keystroke.key == key
        && modifiers.platform
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.function
        && !modifiers.shift
}
