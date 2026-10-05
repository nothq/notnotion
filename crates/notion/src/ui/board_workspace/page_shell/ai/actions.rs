use crate::ui::surface::{NotionAiMode, NotionChromeState};

pub(super) enum NotionAiAction {
    Open,
    ActivateInput,
    NewChat,
    Hide,
    OpenModeMenu,
    DismissModeMenu,
    HoverMode { mode: NotionAiMode, hovered: bool },
    SelectMode(NotionAiMode),
    SetPrompt(String),
}

impl NotionChromeState {
    pub(super) fn apply_notion_ai_action(&mut self, action: NotionAiAction) -> bool {
        match action {
            NotionAiAction::Open => {
                self.ai_autofill_dialog = None;
                self.notion_ai_block_context = None;
                self.notion_ai_open = true;
                self.notion_ai_input_active = true;
            }
            NotionAiAction::ActivateInput => self.notion_ai_input_active = true,
            NotionAiAction::NewChat => {
                self.notion_ai_mode_menu_open = false;
                self.notion_ai_input.clear();
                self.notion_ai_input_active = true;
            }
            NotionAiAction::Hide => {
                self.notion_ai_mode_menu_open = false;
                self.notion_ai_open = false;
                self.notion_ai_input_active = false;
                self.notion_ai_block_context = None;
            }
            NotionAiAction::OpenModeMenu => {
                if self.notion_ai_mode_menu_open {
                    return false;
                }
                self.notion_ai_mode_menu_open = true;
                self.notion_ai_mode_menu_active = NotionAiMode::Sidebar;
            }
            NotionAiAction::DismissModeMenu => {
                return std::mem::take(&mut self.notion_ai_mode_menu_open);
            }
            NotionAiAction::HoverMode { mode, hovered } => {
                if !hovered || self.notion_ai_mode_menu_active == mode {
                    return false;
                }
                self.notion_ai_mode_menu_active = mode;
            }
            NotionAiAction::SelectMode(mode) => {
                self.notion_ai_mode = mode;
                self.notion_ai_mode_menu_open = false;
                self.notion_ai_input_active = true;
            }
            NotionAiAction::SetPrompt(prompt) => {
                self.notion_ai_input = prompt;
                self.notion_ai_input_active = true;
            }
        }
        true
    }
}
