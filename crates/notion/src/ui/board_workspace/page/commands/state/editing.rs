use super::super::super::editor::editing::{
    PageEditEffect, PageEditHostEffect, PageEditWriteEffect, PageEditorEffect,
};
use super::super::super::editor::{mention_menu_token, PageEditSession, PageMutationPlan};
use crate::model::CardPageCodeSettings;
use crate::ui::surface::PageEditorState;
use crate::ui::{
    generated_notion_record_id, CardPageBlock, CardPageStructuralBlock, PageCommandTarget,
    PageComposerHandoff, PageComposerState, PageSlashMenuState,
};
use gpui_components::text_input::TextInputSnapshot;

pub(in crate::ui::board_workspace::page) enum PageComposerAction {
    Change {
        page_id: String,
        snapshot: TextInputSnapshot,
    },
    Submit {
        page_id: String,
        snapshot: TextInputSnapshot,
    },
    Select(PageCommandTarget),
}

pub(crate) enum PageComposerCompletion {
    Reset,
    CloseSlash,
    Handoff {
        page_id: String,
        block_id: String,
        slash_menu: Option<PageSlashMenuState>,
    },
}

impl PageComposerCompletion {
    pub(in crate::ui::board_workspace::page) fn apply(self, editor: &mut PageEditorState) {
        match self {
            Self::Reset => editor.reset_page_composer(),
            Self::CloseSlash => {
                if !editor.input.composer.slash_command_open {
                    return;
                }
                let page_id = editor
                    .input
                    .composer
                    .page_id
                    .clone()
                    .expect("open page composer command menu must have a target page");
                editor.input.composer.slash_command_open = false;
                *editor
                    .input
                    .resource_state()
                    .composer_focus_request
                    .borrow_mut() = Some(page_id);
            }
            Self::Handoff {
                page_id,
                block_id,
                slash_menu,
            } => {
                editor.reset_page_composer();
                editor.input.composer_handoff = Some(PageComposerHandoff { page_id, block_id });
                editor.page_slash_menu = slash_menu;
            }
        }
    }
}

impl PageEditSession<'_> {
    pub(super) fn apply_page_composer_text_change(
        &mut self,
        page_id: &str,
        snapshot: TextInputSnapshot,
    ) {
        if let Some(block_id) = self.editor.page_composer_handoff_block_id(page_id) {
            let cursor = snapshot.cursor;
            self.apply_page_block_text_change(&block_id, snapshot);
            self.effects.push(PageEditEffect::FocusBlock {
                block_id,
                offset: cursor,
            });
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let cursor = snapshot.cursor;
        let should_materialize = !snapshot.is_composing && !snapshot.text.is_empty();
        let slash_command_open = !snapshot.is_composing && snapshot.text.starts_with('/');
        if self.editor.input.composer.page_id.as_deref() != Some(page_id) {
            self.editor.input.composer = PageComposerState {
                page_id: Some(page_id.to_string()),
                ..Default::default()
            };
        }
        let composer = &mut self.editor.input.composer;
        let changed = composer.text != snapshot.text
            || composer.slash_command_open != slash_command_open
            || !composer.active;
        let query_changed = composer.text != snapshot.text;
        composer.active = true;
        composer.text = snapshot.text;
        composer.slash_command_open = slash_command_open;
        if query_changed {
            composer.selected_command_index = 0;
        }
        if should_materialize {
            self.commit_page_composer(page_id, cursor);
        } else if changed {
            self.effects.push(PageEditEffect::Notify);
        }
    }

    pub(super) fn finish_page_composer_submission(
        &mut self,
        page_id: &str,
        cursor: usize,
        settings: CardPageCodeSettings,
    ) -> Option<String> {
        if let Some(block_id) = self.editor.page_composer_handoff_block_id(page_id) {
            self.effects.push(PageEditEffect::FocusBlock {
                block_id: block_id.clone(),
                offset: cursor,
            });
            return Some(block_id);
        }
        if !self.editor.input.composer.slash_command_open {
            self.commit_page_composer(page_id, cursor);
            return None;
        }
        let commands = self.editor.filtered_page_commands();
        let selected = self.editor.selected_page_command_row_index(&commands);
        if let Some(command) = commands.get(selected) {
            self.select_page_command(command.target, settings);
        } else {
            self.effects
                .push(PageEditEffect::Editor(PageEditorEffect::CompleteComposer(
                    PageComposerCompletion::CloseSlash,
                )));
            self.effects.push(PageEditEffect::Notify);
        }
        None
    }
    pub(super) fn select_page_command(
        &mut self,
        target: PageCommandTarget,
        code_settings: CardPageCodeSettings,
    ) {
        let kind = match target {
            PageCommandTarget::Editable(kind) => kind,
            PageCommandTarget::Code => {
                self.insert_page_composer_code(code_settings);
                return;
            }
            PageCommandTarget::Divider => {
                self.insert_page_composer_divider();
                return;
            }
        };
        let page_id = self
            .editor
            .input
            .composer
            .page_id
            .clone()
            .expect("open page composer command menu must have a target page");
        let composer = &mut self.editor.input.composer;
        composer.active = true;
        composer.slash_command_open = false;
        composer.block_kind = kind;
        composer.text.clear();
        composer.selected_command_index = 0;
        *self
            .editor
            .input
            .resource_state()
            .composer_focus_request
            .borrow_mut() = Some(page_id);
        self.effects.push(PageEditEffect::Notify);
    }

    fn insert_page_composer_divider(&mut self) {
        let Some(page_id) = self.editor.input.composer.page_id.clone() else {
            return;
        };
        let Some(mut page) = self.documents.page_with_id(&page_id) else {
            return;
        };
        self.editor.record_page_structural_edit(&page, None);
        let divider = CardPageBlock::structural(
            generated_notion_record_id(),
            page_id.clone(),
            0,
            CardPageStructuralBlock::Divider,
        );
        page.blocks.push(divider.clone());
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::MarkPageHasContent(page_id.clone()),
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::appended_block(
                &page_id, &divider,
            )),
        ));
        self.effects
            .push(PageEditEffect::Editor(PageEditorEffect::CompleteComposer(
                PageComposerCompletion::Reset,
            )));
        self.effects.push(PageEditEffect::Notify);
    }

    fn commit_page_composer(&mut self, page_id: &str, cursor: usize) {
        if self.editor.input.composer.page_id.as_deref() != Some(page_id) {
            return;
        }
        let Some(mut next_page) = self.documents.page_with_id(page_id) else {
            return;
        };
        self.editor.record_page_structural_edit(&next_page, None);
        let text = self.editor.input.composer.text.clone();
        let new_block = CardPageBlock::editable(
            generated_notion_record_id(),
            page_id.to_string(),
            0,
            self.editor.input.composer.block_kind,
            text.clone(),
        );
        let new_block_id = new_block.block_id.clone();
        let slash_menu = self
            .editor
            .input
            .composer
            .materialized_slash_menu(&new_block_id);
        next_page.blocks.push(new_block.clone());
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::MarkPageHasContent(page_id.to_string()),
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(next_page));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::appended_block(
                page_id, &new_block,
            )),
        ));
        self.effects
            .push(PageEditEffect::Editor(PageEditorEffect::CompleteComposer(
                PageComposerCompletion::Handoff {
                    page_id: page_id.to_string(),
                    block_id: new_block_id.clone(),
                    slash_menu,
                },
            )));
        if let Some((trigger_offset, query)) = mention_menu_token(&text, cursor) {
            self.effects
                .push(PageEditEffect::Host(PageEditHostEffect::OpenPageMention {
                    block_id: new_block_id.clone(),
                    trigger_offset,
                    query,
                }));
        }
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: new_block_id,
            offset: cursor,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}

impl PageComposerState {
    fn materialized_slash_menu(&self, block_id: &str) -> Option<PageSlashMenuState> {
        self.slash_command_open.then(|| PageSlashMenuState {
            block_id: block_id.to_string(),
            trigger_offset: 0,
            query: self
                .text
                .strip_prefix('/')
                .unwrap_or(&self.text)
                .to_string(),
            selected_command_index: self.selected_command_index,
        })
    }
}
