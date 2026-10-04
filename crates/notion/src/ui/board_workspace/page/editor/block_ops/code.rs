use gpui::{App, ClipboardItem};

use super::super::editing::{
    PageCodeSettingsStage, PageEditEffect, PageEditHostEffect, PageEditSession,
    PageEditWriteEffect, PageEditorEffect,
};
use super::super::CardPageBlockKind;
use crate::model::{
    CardPageCodeLanguage, CardPageCodeSettings, PageCodeBlockSourceText, PageMutation,
    ReplacePageBlockWithCodeRequest, SetPageCodeLanguageRequest, SetPageCodeWrapRequest,
};
use crate::ui::surface::PageEditorState;

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn replace_verified_page_text_block_with_code(
        &mut self,
        mut page: crate::ui::CardPage,
        block_index: usize,
        source_text: PageCodeBlockSourceText,
        settings: CardPageCodeSettings,
    ) {
        let source = page.blocks[block_index].clone();
        let code_id = super::super::generated_notion_record_id();
        let code = crate::ui::CardPageBlock::code(
            code_id.clone(),
            source.parent_block_id,
            source.depth,
            settings.clone(),
        );
        let request = ReplacePageBlockWithCodeRequest::new(
            source.block_id.clone(),
            code_id.clone(),
            source_text,
            settings,
        )
        .expect("verified Code replacement must use distinct generated block IDs");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page.block_id.clone(),
                mutation: PageMutation::ReplaceBlockWithCode(request),
            },
        ));
        page.blocks[block_index] = code;
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::FinishTextBlockCodeReplacement {
                source_block_id: source.block_id,
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: code_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn copy_page_code_text(
        &mut self,
        block_id: &str,
    ) {
        let Some(text) = self
            .documents
            .page_containing_block(block_id)
            .and_then(|page| {
                page.blocks
                    .into_iter()
                    .find(|block| block.block_id == block_id)
                    .and_then(|block| {
                        block.editable_content().and_then(|editable| {
                            (editable.kind == CardPageBlockKind::Code)
                                .then(|| editable.text.clone())
                        })
                    })
            })
        else {
            return;
        };
        self.effects
            .push(PageEditEffect::WriteClipboard(ClipboardItem::new_string(
                text,
            )));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenu,
        ));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn set_page_code_language(
        &mut self,
        block_id: &str,
        language: CardPageCodeLanguage,
        cx: &App,
    ) {
        let Some((mut page, index)) = self.documents.page_code_block(block_id) else {
            return;
        };
        let current = page.blocks[index]
            .editable_content()
            .expect("validated Code target must remain editable")
            .code_language()
            .expect("validated Code target must retain a language");
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenu,
        ));
        self.effects
            .push(PageEditEffect::Host(PageEditHostEffect::StageCodeSettings(
                PageCodeSettingsStage::Language(language.clone()),
            )));
        if current == &language {
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let updated = page.blocks[index]
            .editable_content_mut()
            .expect("validated Code target must remain editable")
            .set_code_language(language.clone());
        debug_assert!(updated, "validated Code block must retain its language");
        let request = SetPageCodeLanguageRequest::new(block_id, language)
            .expect("validated Code target must contain a block ID");
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::SetCodeLanguage(request),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn toggle_page_code_wrap(
        &mut self,
        block_id: &str,
        cx: &App,
    ) {
        let Some((mut page, index)) = self.documents.page_code_block(block_id) else {
            return;
        };
        let current = page.blocks[index]
            .editable_content()
            .expect("validated Code target must remain editable")
            .code_wrap()
            .expect("validated Code target must retain a wrap mode");
        let next = current.toggled();
        self.effects
            .push(PageEditEffect::Host(PageEditHostEffect::StageCodeSettings(
                PageCodeSettingsStage::Wrap(next),
            )));
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let updated = page.blocks[index]
            .editable_content_mut()
            .expect("validated Code target must remain editable")
            .set_code_wrap(next);
        debug_assert!(updated, "validated Code block must retain its wrap mode");
        let request = SetPageCodeWrapRequest::new(block_id, next)
            .expect("validated Code target must contain a block ID");
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::SetCodeWrap(request),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn finish_page_text_block_code_replacement(
        &mut self,
        source_block_id: &str,
    ) {
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .remove(source_block_id);
        self.page_block_compositions.remove(source_block_id);
        self.clear_page_composer_handoff(source_block_id);
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.active_page_block = None;
        self.page_block_selection.block_ids.clear();
    }
}

impl crate::ui::surface::PageDocuments {
    pub(in crate::ui::board_workspace::page::editor) fn page_code_block(
        &self,
        block_id: &str,
    ) -> Option<(crate::ui::CardPage, usize)> {
        let page = self.page_containing_block(block_id)?;
        let index = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)?;
        if !page.blocks[index]
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::Code)
        {
            return None;
        }
        Some((page, index))
    }
}
