use super::super::super::{Context, SurfaceState, PAGE_COMMANDS};
use crate::ui::surface::PageEditorState;
use crate::ui::PageCommand;

mod callbacks;
mod code;
mod editing;
mod host;

pub(in crate::ui::board_workspace::page) use editing::{
    PageComposerAction, PageComposerCompletion,
};

pub(in crate::ui::board_workspace::page) use callbacks::{
    page_composer_on_backspace_when_empty, page_composer_on_change, page_composer_on_escape,
    page_composer_on_focus, page_composer_on_move, page_composer_on_submit,
};

impl PageEditorState {
    pub(crate) fn move_page_command_selection(
        &mut self,
        delta: isize,
        cx: &mut Context<SurfaceState>,
    ) {
        let commands = self.filtered_page_commands();
        let row_count = commands.len() + 1;
        let current = self.selected_page_command_row_index(&commands) as isize;
        self.input.composer.selected_command_index =
            (current + delta).rem_euclid(row_count as isize) as usize;
        cx.notify();
    }

    pub(crate) fn close_page_composer_slash_menu(&mut self, cx: &mut Context<SurfaceState>) {
        if !self.input.composer.slash_command_open {
            return;
        }
        let page_id = self
            .input
            .composer
            .page_id
            .clone()
            .expect("open page composer command menu must have a target page");
        self.input.composer.slash_command_open = false;
        *self
            .input
            .resource_state()
            .composer_focus_request
            .borrow_mut() = Some(page_id);
        cx.notify();
    }

    fn focus_page_composer(&mut self, page_id: &str, cx: &mut Context<SurfaceState>) {
        if self.page_composer_handoff_block_id(page_id).is_some() {
            return;
        }
        if self.input.composer.page_id.as_deref() != Some(page_id) {
            self.input.composer = super::super::super::PageComposerState {
                page_id: Some(page_id.to_string()),
                active: true,
                ..Default::default()
            };
            cx.notify();
        } else if !self.input.composer.active {
            self.input.composer.active = true;
            cx.notify();
        }
    }

    pub(in crate::ui::board_workspace::page) fn escape_page_composer(
        &mut self,
        page_id: &str,
        cx: &mut Context<SurfaceState>,
    ) {
        if self.input.composer.page_id.as_deref() != Some(page_id) {
            return;
        }
        if self.input.composer.slash_command_open {
            self.close_page_composer_slash_menu(cx);
        } else {
            self.reset_page_composer();
            cx.notify();
        }
    }

    fn page_composer_handoff_block_id(&self, page_id: &str) -> Option<String> {
        self.input
            .composer_handoff
            .as_ref()
            .filter(|handoff| handoff.page_id == page_id)
            .map(|handoff| handoff.block_id.clone())
    }

    pub(crate) fn clear_page_composer_handoff(&mut self, block_id: &str) {
        if self
            .input
            .composer_handoff
            .as_ref()
            .is_some_and(|handoff| handoff.block_id == block_id)
        {
            self.input.composer_handoff = None;
        }
    }

    pub(crate) fn filtered_page_commands(&self) -> Vec<PageCommand> {
        let query = self
            .input
            .composer
            .text
            .strip_prefix('/')
            .unwrap_or(&self.input.composer.text)
            .trim()
            .to_ascii_lowercase();
        PAGE_COMMANDS
            .into_iter()
            .filter(|command| {
                query.is_empty()
                    || command.label.to_ascii_lowercase().contains(&query)
                    || command
                        .keywords
                        .iter()
                        .any(|keyword| keyword.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }

    pub(crate) fn selected_page_command_row_index(&self, commands: &[PageCommand]) -> usize {
        self.input
            .composer
            .selected_command_index
            .min(commands.len())
    }
}
