use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::rich_text::annotations::{
    apply_annotation_actions_to_range, apply_plain_text_annotation_replacement,
    apply_typing_annotations,
};
use super::super::slash::{page_block_markdown_shortcut, previous_cursor_offset};
use super::super::support::page_block_subtree_end;
use super::super::{CardPage, CardPageBlockKind, PageCommandTarget, TextInputSnapshot};
use super::PageRichTextInsertion;
use crate::model::PageCodeBlockSourceText;

pub(super) struct PageBlockTextChangePlan {
    pub(super) text_changed: bool,
    pub(super) previous_cursor: usize,
    pub(super) markdown_shortcut: Option<VerifiedPageBlockMarkdownShortcut>,
}

#[derive(Clone)]
pub(super) enum VerifiedPageBlockMarkdownShortcut {
    Editable(VerifiedPageTextBlockKind),
    Code(VerifiedPageCodeShortcut),
    Divider(VerifiedPageDividerShortcut),
}

#[derive(Clone)]
pub(super) struct VerifiedPageCodeShortcut {
    pub(super) block_index: usize,
    pub(super) source_text: PageCodeBlockSourceText,
}

#[derive(Clone, Copy)]
pub(super) struct VerifiedPageDividerShortcut {
    pub(super) block_index: usize,
}

pub(super) fn page_block_text_change_plan(
    page: &CardPage,
    block_id: &str,
    snapshot: &TextInputSnapshot,
) -> Option<PageBlockTextChangePlan> {
    let block_index = page
        .blocks
        .iter()
        .position(|block| block.block_id == block_id)?;
    let block = &page.blocks[block_index];
    let editable = block.editable_content()?;
    let text_changed = editable.text != snapshot.text;
    let markdown_shortcut = (!snapshot.is_composing
        && editable.kind == CardPageBlockKind::Text
        && snapshot.selection.is_empty())
    .then(|| page_block_markdown_shortcut(&snapshot.text, snapshot.cursor))
    .flatten()
    .and_then(|target| match target {
        PageCommandTarget::Editable(kind) => {
            VerifiedPageTextBlockKind::parse(kind).map(VerifiedPageBlockMarkdownShortcut::Editable)
        }
        PageCommandTarget::Code
            if block.parent_block_id == page.block_id
                && block.depth == 0
                && editable.annotations.is_empty()
                && page_block_subtree_end(&page.blocks, block_index) == block_index + 1 =>
        {
            Some(VerifiedPageBlockMarkdownShortcut::Code(
                VerifiedPageCodeShortcut {
                    block_index,
                    source_text: PageCodeBlockSourceText::triple_backtick(editable.text.clone())
                        .ok()?,
                },
            ))
        }
        PageCommandTarget::Code => None,
        PageCommandTarget::Divider
            if page_block_subtree_end(&page.blocks, block_index) == block_index + 1 =>
        {
            Some(VerifiedPageBlockMarkdownShortcut::Divider(
                VerifiedPageDividerShortcut { block_index },
            ))
        }
        PageCommandTarget::Divider => None,
    });
    Some(PageBlockTextChangePlan {
        text_changed,
        previous_cursor: if text_changed {
            previous_cursor_offset(&editable.text, &snapshot.text)
        } else {
            snapshot.cursor
        },
        markdown_shortcut,
    })
}

pub(super) fn apply_page_block_text_change(
    page: &mut CardPage,
    block_id: &str,
    snapshot: &TextInputSnapshot,
    plan: &PageBlockTextChangePlan,
    pending_insertion: Option<&PageRichTextInsertion>,
) -> Option<String> {
    let block = page
        .blocks
        .iter_mut()
        .find(|block| block.block_id == block_id)?;
    let editable = block.editable_content_mut()?;
    if plan.text_changed {
        apply_changed_text_annotations(editable, &snapshot.text, pending_insertion);
        editable.text.clone_from(&snapshot.text);
    }
    if let Some(PageRichTextInsertion::Composition(pending)) =
        pending_insertion.filter(|pending| match pending {
            PageRichTextInsertion::Composition(pending) => !pending.text.is_empty(),
            PageRichTextInsertion::Typing(_) => false,
        })
    {
        apply_annotation_actions_to_range(
            editable,
            pending.offset_utf8..pending.offset_utf8 + pending.text.len(),
            &pending.removals,
            &pending.additions,
        );
    }
    match &plan.markdown_shortcut {
        Some(VerifiedPageBlockMarkdownShortcut::Editable(conversion)) => {
            editable.set_kind(conversion.card_kind());
            editable.text.clear();
            editable.annotations.clear();
        }
        Some(VerifiedPageBlockMarkdownShortcut::Divider(_)) => return None,
        Some(VerifiedPageBlockMarkdownShortcut::Code(_)) => return None,
        None => {}
    }
    Some(editable.text.clone())
}

pub(super) fn apply_changed_text_annotations(
    editable: &mut crate::ui::CardPageEditableBlock,
    next_text: &str,
    pending_insertion: Option<&PageRichTextInsertion>,
) {
    if let Some(PageRichTextInsertion::Typing(pending)) = pending_insertion {
        apply_typing_annotations(
            editable,
            pending.offset_utf8,
            pending.text.len(),
            &pending.removals,
            &pending.additions,
        );
    } else {
        let previous_text = editable.text.clone();
        apply_plain_text_annotation_replacement(editable, &previous_text, next_text);
    }
}
