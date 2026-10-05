use super::super::super::super::{generated_notion_record_id, CardPageBlock};
use super::super::super::editor::editing::{
    PageEditEffect, PageEditHostEffect, PageEditWriteEffect, PageEditorEffect,
};
use crate::model::{CreatePageCodeBlockRequest, PageBlockPlacement, PageMutation};

impl super::super::super::editor::PageEditSession<'_> {
    pub(super) fn insert_page_composer_code(
        &mut self,
        settings: crate::model::CardPageCodeSettings,
    ) {
        let Some(page_id) = self.editor.input.composer.page_id.clone() else {
            return;
        };
        let Some(mut page) = self.documents.page_with_id(&page_id) else {
            return;
        };
        self.editor.record_page_structural_edit(&page, None);
        let code_id = generated_notion_record_id();
        let code = CardPageBlock::code(code_id.clone(), page_id.clone(), 0, settings.clone());
        let request = CreatePageCodeBlockRequest::new(
            code_id.clone(),
            page_id.clone(),
            PageBlockPlacement::Append,
            settings,
        )
        .expect("page composer Code creation must contain valid generated IDs");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page_id.clone(),
                mutation: PageMutation::CreateCodeBlock(request),
            },
        ));
        page.blocks.push(code);
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::MarkPageHasContent(page_id),
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects
            .push(PageEditEffect::Editor(PageEditorEffect::CompleteComposer(
                super::PageComposerCompletion::Reset,
            )));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: code_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}
