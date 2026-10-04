use super::super::editing::{
    PageEditEffect, PageEditHostEffect, PageEditSession, PageEditTransition, PageEditWriteEffect,
};
use super::super::persistence::{PageBlockTextConversion, VerifiedPageTextBlockKind};
use super::super::rich_text::annotations::replace_annotated_text_range;
use super::super::support::page_block_subtree_end;
use super::super::{
    generated_notion_record_id, CardPage, CardPageBlock, CardPageBlockKind,
    CardPageStructuralBlock, PageCommandTarget, PageMutationPlan, PageSlashMenuState,
};
use super::support::{clamped_slash_command_index, filtered_page_slash_commands, slash_menu_token};
use crate::model::{PageCodeBlockSourceText, PageMutation, ReplacePageBlockWithDividerRequest};
use crate::ui::surface::PageEditorState;
use crate::ui::PageEditFocus;
use gpui_components::text_input::TextInputSnapshot;

struct PreparedPageSlashCommand {
    page: CardPage,
    block_index: usize,
    menu: PageSlashMenuState,
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn reconcile_slash_menu(
        mut self,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> PageEditTransition<bool> {
        let changed = self.update_native_page_slash_menu(block_id, snapshot);
        self.finish(changed)
    }

    pub(crate) fn update_native_page_slash_menu(
        &mut self,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> bool {
        let supports_slash_commands = self.page_block_supports_native_slash_commands(block_id);
        let next = if snapshot.is_composing || !supports_slash_commands {
            None
        } else {
            slash_menu_token(&snapshot.text, snapshot.cursor).map(|(trigger_offset, query)| {
                let selected_command_index = self
                    .editor
                    .page_slash_menu
                    .as_ref()
                    .filter(|menu| {
                        menu.block_id == block_id
                            && menu.trigger_offset == trigger_offset
                            && menu.query == query
                    })
                    .map_or(0, |menu| menu.selected_command_index);
                PageSlashMenuState {
                    block_id: block_id.to_string(),
                    trigger_offset,
                    query,
                    selected_command_index,
                }
            })
        };
        if next.is_none()
            && self
                .editor
                .page_slash_menu
                .as_ref()
                .is_some_and(|menu| menu.block_id != block_id)
        {
            return false;
        }
        if self.editor.page_slash_menu == next {
            return false;
        }
        self.editor.page_slash_menu = next;
        true
    }

    pub(crate) fn commit_native_page_slash_selection(&mut self, block_id: &str) -> bool {
        let Some(menu) = self
            .editor
            .page_slash_menu
            .as_ref()
            .filter(|menu| menu.block_id == block_id)
            .cloned()
        else {
            return false;
        };
        let commands = filtered_page_slash_commands(&menu.query);
        let Some(command) = commands.get(clamped_slash_command_index(
            menu.selected_command_index,
            commands.len(),
        )) else {
            self.editor.page_slash_menu = None;
            self.editor.mention.clear_menu();
            self.effects.push(PageEditEffect::Notify);
            return true;
        };
        self.apply_native_page_slash_command(block_id, command.target);
        true
    }

    pub(crate) fn apply_native_page_slash_command(
        &mut self,
        block_id: &str,
        target: PageCommandTarget,
    ) {
        let conversion = match target {
            PageCommandTarget::Editable(kind) => self.editor.slash_command_conversion(kind),
            PageCommandTarget::Code | PageCommandTarget::Divider => None,
        };
        if matches!(target, PageCommandTarget::Editable(_)) && conversion.is_none() {
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let Some(prepared) = self.prepare_native_page_slash_command(block_id) else {
            return;
        };
        if target == PageCommandTarget::Divider {
            self.replace_native_page_slash_block_with_divider(prepared);
            return;
        }
        if target == PageCommandTarget::Code {
            self.replace_native_page_slash_block_with_code(prepared);
            return;
        }
        self.apply_editable_page_slash_command(
            prepared,
            conversion.expect("editable slash commands require a conversion"),
        );
    }

    fn apply_editable_page_slash_command(
        &mut self,
        mut prepared: PreparedPageSlashCommand,
        conversion: VerifiedPageTextBlockKind,
    ) {
        let block_id = prepared.page.blocks[prepared.block_index].block_id.clone();
        let page_id = prepared.page.block_id.clone();
        self.editor.record_page_structural_edit(
            &prepared.page,
            Some(PageEditFocus::block(
                &block_id,
                prepared.menu.trigger_offset,
            )),
        );
        let editable = prepared.page.blocks[prepared.block_index]
            .editable_content_mut()
            .expect("slash command target must remain editable");
        let query_end = prepared.menu.trigger_offset + 1 + prepared.menu.query.len();
        replace_annotated_text_range(editable, prepared.menu.trigger_offset..query_end, "");
        editable.set_kind(conversion.card_kind());
        let persisted_text = editable.text.clone();
        self.editor.page_slash_menu = None;
        self.editor.mention.clear_menu();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::text_and_convert(
                &page_id,
                &block_id,
                PageBlockTextConversion {
                    text: persisted_text,
                    conversion,
                },
            )),
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(prepared.page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id,
            offset: prepared.menu.trigger_offset,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    fn prepare_native_page_slash_command(
        &mut self,
        block_id: &str,
    ) -> Option<PreparedPageSlashCommand> {
        let menu = self
            .editor
            .page_slash_menu
            .as_ref()
            .filter(|menu| menu.block_id == block_id)
            .cloned()?;
        let page = self.documents.page_containing_block(block_id)?;
        let block_index = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)?;
        let query_end = menu.trigger_offset + 1 + menu.query.len();
        let block = &page.blocks[block_index];
        let valid = block
            .editable_content()
            .filter(|editable| VerifiedPageTextBlockKind::parse(editable.kind).is_some())
            .is_some_and(|editable| {
                query_end <= editable.text.len()
                    && editable.text.is_char_boundary(menu.trigger_offset)
                    && editable.text.is_char_boundary(query_end)
                    && editable.text[menu.trigger_offset..query_end].strip_prefix('/')
                        == Some(menu.query.as_str())
            });
        if !valid {
            self.editor.page_slash_menu = None;
            self.editor.mention.clear_menu();
            self.effects.push(PageEditEffect::Notify);
            return None;
        }
        Some(PreparedPageSlashCommand {
            page,
            block_index,
            menu,
        })
    }

    fn replace_native_page_slash_block_with_divider(
        &mut self,
        mut prepared: PreparedPageSlashCommand,
    ) {
        let source = prepared.page.blocks[prepared.block_index].clone();
        let subtree_end = page_block_subtree_end(&prepared.page.blocks, prepared.block_index);
        let query_end = prepared.menu.trigger_offset + 1 + prepared.menu.query.len();
        let source_text_is_only_command = source.editable_content().is_some_and(|editable| {
            prepared.menu.trigger_offset == 0 && query_end == editable.text.len()
        });
        if subtree_end != prepared.block_index + 1 || !source_text_is_only_command {
            self.editor.page_slash_menu = None;
            self.editor.mention.clear_menu();
            self.effects.push(PageEditEffect::Error(
                "A divider can only replace a leaf block containing just the slash command."
                    .to_string(),
            ));
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let page_id = prepared.page.block_id.clone();
        let divider_id = generated_notion_record_id();
        let divider = CardPageBlock::structural(
            divider_id.clone(),
            source.parent_block_id.clone(),
            source.depth,
            CardPageStructuralBlock::Divider,
        );
        let focus = PageEditFocus::block(source.block_id.clone(), prepared.menu.trigger_offset);
        self.editor
            .record_page_structural_edit(&prepared.page, Some(focus));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::ReplaceBlockWithDivider(
                    ReplacePageBlockWithDividerRequest::new(source.block_id, divider_id)
                        .expect("slash divider replacement requires distinct generated block IDs"),
                ),
            },
        ));
        let removed_ids = prepared.page.blocks[prepared.block_index..subtree_end]
            .iter()
            .map(|block| block.block_id.clone())
            .collect::<Vec<_>>();
        prepared
            .page
            .blocks
            .splice(prepared.block_index..subtree_end, [divider]);
        self.editor
            .clear_removed_page_block_editor_state(&removed_ids);
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(prepared.page));
        self.effects.push(PageEditEffect::Notify);
    }

    fn replace_native_page_slash_block_with_code(&mut self, prepared: PreparedPageSlashCommand) {
        let source = &prepared.page.blocks[prepared.block_index];
        let subtree_end = page_block_subtree_end(&prepared.page.blocks, prepared.block_index);
        let query_end = prepared.menu.trigger_offset + 1 + prepared.menu.query.len();
        let valid = source.parent_block_id == prepared.page.block_id
            && source.depth == 0
            && subtree_end == prepared.block_index + 1
            && source.editable_content().is_some_and(|editable| {
                editable.kind == CardPageBlockKind::Text
                    && editable.annotations.is_empty()
                    && prepared.menu.trigger_offset == 0
                    && query_end == editable.text.len()
            });
        if !valid {
            self.editor.page_slash_menu = None;
            self.editor.mention.clear_menu();
            self.effects.push(PageEditEffect::Error(
                "Code can only replace a root-level leaf Text block containing just the slash command."
                    .to_string(),
            ));
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let source_text = PageCodeBlockSourceText::slash_token(
            source
                .editable_content()
                .expect("verified Code slash source must remain editable")
                .text
                .clone(),
        )
        .expect("verified Code slash source must be a complete slash token");
        self.editor.record_page_structural_edit(
            &prepared.page,
            Some(PageEditFocus::block(
                &source.block_id,
                prepared.menu.trigger_offset,
            )),
        );
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::ReplaceTextBlockWithCode {
                page: prepared.page,
                block_index: prepared.block_index,
                source_text,
            },
        ));
    }

    fn page_block_supports_native_slash_commands(&self, block_id: &str) -> bool {
        self.documents
            .page_containing_block(block_id)
            .and_then(|page| {
                page.blocks
                    .into_iter()
                    .find(|block| block.block_id == block_id)
            })
            .and_then(|block| {
                block
                    .editable_content()
                    .and_then(|editable| VerifiedPageTextBlockKind::parse(editable.kind))
            })
            .is_some()
    }
}

impl PageEditorState {
    fn slash_command_conversion(
        &mut self,
        kind: CardPageBlockKind,
    ) -> Option<VerifiedPageTextBlockKind> {
        let conversion = VerifiedPageTextBlockKind::parse(kind);
        if conversion.is_none() {
            self.page_slash_menu = None;
            self.mention.clear_menu();
        }
        conversion
    }

    pub(crate) fn move_native_page_slash_selection(
        &mut self,
        block_id: &str,
        delta: isize,
    ) -> bool {
        let Some(menu) = self
            .page_slash_menu
            .as_mut()
            .filter(|menu| menu.block_id == block_id)
        else {
            return false;
        };
        let command_count = filtered_page_slash_commands(&menu.query).len();
        if command_count == 0 {
            return false;
        }
        menu.selected_command_index = (menu.selected_command_index as isize + delta)
            .rem_euclid(command_count as isize) as usize;
        true
    }

    pub(crate) fn close_native_page_slash_menu(&mut self, block_id: &str) -> bool {
        if self
            .page_slash_menu
            .as_ref()
            .is_none_or(|menu| menu.block_id != block_id)
        {
            return false;
        }
        self.page_slash_menu = None;
        self.mention.clear_menu();
        true
    }

    pub(super) fn hover_native_page_slash_selection(
        &mut self,
        block_id: &str,
        index: usize,
    ) -> bool {
        let Some(menu) = self
            .page_slash_menu
            .as_mut()
            .filter(|menu| menu.block_id == block_id)
        else {
            return false;
        };
        if menu.selected_command_index == index {
            return false;
        }
        menu.selected_command_index = index;
        true
    }
}
