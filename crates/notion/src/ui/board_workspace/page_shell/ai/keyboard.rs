use gpui::{Context, KeyDownEvent};

use super::actions::NotionAiAction;
use crate::ui::{
    keystroke_input_text,
    surface::{NotionAiMode, NotionChromeState, SurfaceState},
};

impl NotionChromeState {
    pub(crate) fn handle_notion_ai_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<SurfaceState>,
    ) -> bool {
        if !self.notion_ai_open {
            return false;
        }
        if self.notion_ai_mode == NotionAiMode::FullScreen
            && event.keystroke.modifiers.platform
            && event.keystroke.key == "["
        {
            self.notion_ai_mode = NotionAiMode::Sidebar;
            cx.notify();
            return true;
        }
        if self.notion_ai_mode_menu_open {
            self.handle_notion_ai_mode_menu_key(event.keystroke.key.as_str(), cx);
            return true;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.notion_ai_mode == NotionAiMode::FullScreen {
                    return true;
                }
                self.notion_ai_open = false;
                self.notion_ai_input_active = false;
                self.notion_ai_block_context = None;
                cx.notify();
                return true;
            }
            "backspace" if self.notion_ai_input_active => {
                self.notion_ai_input.pop();
                cx.notify();
                return true;
            }
            "enter" if self.notion_ai_input_active && !self.notion_ai_input.trim().is_empty() => {
                return true;
            }
            _ => {}
        }
        let Some(input) = self
            .notion_ai_input_active
            .then(|| keystroke_input_text(event))
            .flatten()
        else {
            return false;
        };
        self.notion_ai_input.push_str(input);
        cx.notify();
        true
    }

    fn handle_notion_ai_mode_menu_key(&mut self, key: &str, cx: &mut Context<SurfaceState>) {
        match key {
            "escape" => {
                if self.apply_notion_ai_action(NotionAiAction::DismissModeMenu) {
                    cx.notify();
                }
            }
            "up" | "arrowup" => {
                self.notion_ai_mode_menu_active = match self.notion_ai_mode_menu_active {
                    NotionAiMode::Sidebar => NotionAiMode::FullScreen,
                    NotionAiMode::Floating => NotionAiMode::Sidebar,
                    NotionAiMode::FullScreen => NotionAiMode::Floating,
                };
                cx.notify();
            }
            "down" | "arrowdown" => {
                self.notion_ai_mode_menu_active = match self.notion_ai_mode_menu_active {
                    NotionAiMode::Sidebar => NotionAiMode::Floating,
                    NotionAiMode::Floating => NotionAiMode::FullScreen,
                    NotionAiMode::FullScreen => NotionAiMode::Sidebar,
                };
                cx.notify();
            }
            "enter" | "space" => {
                self.apply_notion_ai_action(NotionAiAction::SelectMode(
                    self.notion_ai_mode_menu_active,
                ));
                cx.notify();
            }
            _ => {}
        }
    }
}
