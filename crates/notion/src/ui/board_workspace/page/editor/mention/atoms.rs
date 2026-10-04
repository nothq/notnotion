use std::sync::Arc;

use app_model::AppearanceMode;
use chrono::NaiveDate;
use gpui_components::text_input::TextInputAtom;

use super::super::rich_text::{mention_spans, page_mention_foreground};
use crate::model::{CardPageEditableBlock, PageMention, PAGE_MENTION_TOKEN};

/// Notion paints the leading "@" of a chip at 60% of the label colour.
const AT_SIGN_OPACITY: f32 = 0.6;

/// The mention spans whose token character is still in the text, in document
/// order. A span whose token a pending edit removed is skipped so the atom
/// ranges always land on a `‣`.
fn page_mention_atom_spans(
    editable: &CardPageEditableBlock,
) -> impl Iterator<Item = (usize, &PageMention)> {
    mention_spans(editable).filter(|(offset, _)| {
        editable
            .text
            .get(*offset..)
            .is_some_and(|rest| rest.starts_with(PAGE_MENTION_TOKEN))
    })
}

/// The mention chips of a block, as atoms the text input draws in place of
/// the stored `‣` token. The label is display-only: the content the caret and
/// the CRDT see is still the single token character.
pub(in crate::ui::board_workspace::page::editor) fn page_text_input_atoms(
    editable: &CardPageEditableBlock,
    appearance_mode: AppearanceMode,
    today: NaiveDate,
) -> Arc<[TextInputAtom]> {
    let color = page_mention_foreground(appearance_mode);
    let mut prefix_color = color;
    prefix_color.a *= AT_SIGN_OPACITY;
    page_mention_atom_spans(editable)
        .map(|(offset, mention)| TextInputAtom {
            range: offset..offset + PAGE_MENTION_TOKEN.len_utf8(),
            display: format!("@{}", mention.label(today)).into(),
            prefix_len: "@".len(),
            color,
            prefix_color,
            hover_underline: true,
        })
        .collect()
}

/// The token offset of the atom the text input reported, which indexes the
/// same spans [`page_text_input_atoms`] built the atoms from.
pub(super) fn page_mention_offset_for_atom(
    editable: &CardPageEditableBlock,
    index: usize,
) -> Option<usize> {
    page_mention_atom_spans(editable)
        .map(|(offset, _)| offset)
        .nth(index)
}
