use super::editing::{
    PageEditEffect, PageEditHostEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::persistence::VerifiedPageTextBlockKind;
use super::{
    generated_notion_record_id, CardPage, CardPageBlock, CardPageBlockContent, CardPageBlockKind,
    CardPageStructuralBlock, PagePendingCrossBlockComposition, PagePendingRichTextComposition,
    PagePendingRichTextTyping, TextInputSnapshot,
};
use crate::model::{
    ConvertPageBlockToDividerRequest, EditPageBlockTextRequest, PageMutation, PageTextEditTarget,
};
use crate::ui::board_workspace::page::editor::PageMutationPlan;
use crate::ui::surface::PageEditorState;

mod cross_block_composition;
mod lifecycle;
mod plan;
mod workflow;

use lifecycle::PageBlockTextChangeState;
use plan::apply_changed_text_annotations;
use plan::{
    apply_page_block_text_change, page_block_text_change_plan, PageBlockTextChangePlan,
    VerifiedPageBlockMarkdownShortcut, VerifiedPageCodeShortcut, VerifiedPageDividerShortcut,
};
use workflow::{PageBlockTextChangeCommit, PageBlockTextChangePreparation};

enum PageRichTextInsertion {
    Typing(PagePendingRichTextTyping),
    Composition(PagePendingRichTextComposition),
}

impl PageEditSession<'_> {
    pub(crate) fn apply_page_block_text_change(
        &mut self,
        block_id: &str,
        snapshot: TextInputSnapshot,
    ) {
        let Some(preparation) = self.prepare_page_block_text_change(block_id, &snapshot) else {
            return;
        };
        let PageBlockTextChangePreparation {
            page,
            plan,
            pending_insertion,
            cross_block_composition,
            was_composing,
            composition_committed,
        } = preparation;
        let change = PageBlockTextChangeState {
            block_id,
            snapshot: &snapshot,
            plan: &plan,
            composition_committed,
        };
        self.editor.record_page_block_text_change_history(
            &page,
            &change,
            pending_insertion.as_ref(),
            was_composing,
        );
        let Some(page) = self.apply_terminal_page_markdown_shortcut(page, block_id, &plan) else {
            return;
        };
        self.commit_page_block_text_change(PageBlockTextChangeCommit {
            page,
            pending_insertion,
            cross_block_composition,
            change: &change,
        });
    }

    fn apply_page_markdown_divider_shortcut(
        &mut self,
        mut page: CardPage,
        block_id: &str,
        shortcut: VerifiedPageDividerShortcut,
    ) {
        let source = page
            .blocks
            .get(shortcut.block_index)
            .filter(|block| block.block_id == block_id)
            .cloned()
            .expect("verified divider shortcut target must remain at its planned index");
        let mut divider = source.clone();
        divider.content = CardPageBlockContent::Structural(CardPageStructuralBlock::Divider);
        let continuation_id = generated_notion_record_id();
        let continuation = CardPageBlock::editable(
            continuation_id.clone(),
            source.parent_block_id,
            source.depth,
            CardPageBlockKind::Text,
            String::new(),
        );
        let request =
            ConvertPageBlockToDividerRequest::new(source.block_id.clone(), continuation_id.clone())
                .expect("generated divider continuation must have a distinct Notion block ID");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page.block_id.clone(),
                mutation: PageMutation::ConvertBlockToDivider(request),
            },
        ));
        page.blocks.splice(
            shortcut.block_index..shortcut.block_index + 1,
            [divider, continuation],
        );
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::FinishMarkdownDivider {
                block_id: block_id.to_string(),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: continuation_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    fn apply_page_markdown_code_shortcut(
        &mut self,
        page: CardPage,
        _block_id: &str,
        shortcut: VerifiedPageCodeShortcut,
    ) {
        debug_assert_eq!(
            page.blocks[shortcut.block_index].block_id, _block_id,
            "verified Code shortcut target must remain at its planned index"
        );
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::ReplaceTextBlockWithCode {
                page,
                block_index: shortcut.block_index,
                source_text: shortcut.source_text,
            },
        ));
    }

    fn replace_changed_page(&mut self, page: CardPage, changed: bool) {
        if changed {
            self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        }
    }

    fn persist_page_block_change(&mut self, persistence: PageBlockPersistence<'_>) {
        if let Some(composition) = persistence.cross_block_composition {
            self.persist_completed_cross_block_composition(composition, persistence.text);
            return;
        }
        if !persistence.should_persist {
            return;
        }
        if let Some(conversion) = persistence.conversion {
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::text_and_convert(
                    persistence.page_block_id,
                    persistence.block_id,
                    super::persistence::PageBlockTextConversion {
                        text: persistence.text,
                        conversion,
                    },
                )),
            ));
        } else if let Some(insertion) = persistence.insertion {
            if let Some(pending) = insertion.into_typing() {
                self.persist_page_rich_text_typing(persistence.page_block_id, pending);
            }
        } else if !persistence.composition_active {
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::text(
                    persistence.page_block_id,
                    persistence.block_id,
                    persistence.text,
                )),
            ));
        }
    }

    fn persist_page_rich_text_typing(
        &mut self,
        page_block_id: &str,
        pending: PagePendingRichTextTyping,
    ) {
        let request = EditPageBlockTextRequest::new(
            page_block_id.to_string(),
            vec![PageTextEditTarget::Typing {
                block_id: pending.block_id,
                offset_utf8: pending.offset_utf8,
                text: pending.text,
            }],
            pending.removals,
            pending.additions,
        )
        .expect("captured rich-text typing must form a valid request");
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueRichTextEdit(request),
        ));
    }
}

impl PageEditorState {
    pub(super) fn update_page_block_composition(
        &mut self,
        block_id: &str,
        is_composing: bool,
    ) -> (bool, bool) {
        let was_composing = self.page_block_compositions.contains(block_id);
        if is_composing {
            self.page_block_compositions.insert(block_id.to_string());
        } else {
            self.page_block_compositions.remove(block_id);
        }
        (was_composing, was_composing && !is_composing)
    }

    fn take_page_rich_text_insertion(
        &mut self,
        page: &CardPage,
        block_id: &str,
        snapshot: &TextInputSnapshot,
        composition_committed: bool,
    ) -> Option<PageRichTextInsertion> {
        if composition_committed {
            return self
                .take_matching_page_rich_text_composition(block_id, snapshot)
                .map(PageRichTextInsertion::Composition);
        }
        self.take_matching_page_rich_text_typing(
            block_id,
            page_block_text(page, block_id),
            snapshot,
        )
        .map(PageRichTextInsertion::Typing)
    }

    fn take_matching_page_rich_text_typing(
        &mut self,
        block_id: &str,
        previous_text: Option<&str>,
        snapshot: &TextInputSnapshot,
    ) -> Option<PagePendingRichTextTyping> {
        let pending = self.page_pending_rich_text_typing.take()?;
        let previous_text = previous_text?;
        let matching = pending.block_id == block_id
            && pending.offset_utf8 <= previous_text.len()
            && previous_text.is_char_boundary(pending.offset_utf8)
            && inserted_text_matches(previous_text, &pending, &snapshot.text);
        if !matching {
            return None;
        }
        if let Some(forced) = self.page_forced_text_annotations.as_mut().filter(|forced| {
            forced.block_id == block_id && forced.offset_utf8 == pending.offset_utf8
        }) {
            forced.offset_utf8 += pending.text.len();
        }
        Some(pending)
    }

    fn take_matching_page_rich_text_composition(
        &mut self,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> Option<PagePendingRichTextComposition> {
        let pending = self.page_pending_rich_text_composition.take()?;
        let matching = pending.block_id == block_id
            && inserted_text_matches_parts(
                &pending.baseline_text,
                pending.offset_utf8,
                &pending.text,
                &snapshot.text,
            );
        if !matching {
            return None;
        }
        if let Some(forced) = self.page_forced_text_annotations.as_mut().filter(|forced| {
            forced.block_id == block_id && forced.offset_utf8 == pending.offset_utf8
        }) {
            forced.offset_utf8 += pending.text.len();
        }
        Some(pending)
    }
}

struct PageBlockPersistence<'a> {
    page_block_id: &'a str,
    block_id: &'a str,
    text: String,
    conversion: Option<VerifiedPageTextBlockKind>,
    insertion: Option<PageRichTextInsertion>,
    composition_active: bool,
    cross_block_composition: Option<PagePendingCrossBlockComposition>,
    should_persist: bool,
}

impl PageRichTextInsertion {
    fn into_typing(self) -> Option<PagePendingRichTextTyping> {
        match self {
            Self::Typing(pending) => Some(pending),
            Self::Composition(pending) if pending.text.is_empty() => None,
            Self::Composition(pending) => Some(PagePendingRichTextTyping {
                block_id: pending.block_id,
                offset_utf8: pending.offset_utf8,
                text: pending.text,
                removals: pending.removals,
                additions: pending.additions,
            }),
        }
    }
}

fn page_block_text<'a>(page: &'a CardPage, block_id: &str) -> Option<&'a str> {
    page.blocks
        .iter()
        .find(|block| block.block_id == block_id)?
        .editable_content()
        .map(|editable| editable.text.as_str())
}

fn inserted_text_matches(
    previous_text: &str,
    pending: &PagePendingRichTextTyping,
    next_text: &str,
) -> bool {
    inserted_text_matches_parts(previous_text, pending.offset_utf8, &pending.text, next_text)
}

fn inserted_text_matches_parts(
    previous_text: &str,
    offset: usize,
    inserted: &str,
    next_text: &str,
) -> bool {
    offset <= previous_text.len()
        && previous_text.is_char_boundary(offset)
        && next_text.is_char_boundary(offset)
        && next_text.is_char_boundary(offset + inserted.len())
        && next_text.len() == previous_text.len() + inserted.len()
        && next_text[..offset] == previous_text[..offset]
        && next_text[offset..].starts_with(inserted)
        && next_text[offset + inserted.len()..] == previous_text[offset..]
}
