//! Notion's temporary input: while the `@` menu is open the typed trigger sits
//! in a faint pill, and the rest of the selected suggestion's label trails the
//! caret as ghost text. Measured on Notion desktop 7.31.3, see
//! `reference/notion-7.31.3/mention-menu.json`.

use std::ops::Range;

use gpui::{px, Hsla};
use gpui_components::text_input::{TextInputGhost, TextInputHighlight};

use super::super::rich_text::{page_mention_ghost_foreground, page_mention_input_pill_background};
use super::state::PageMentionController;
use super::PageMentionClock;
use crate::ui::{AppearanceMode, PageMentionMenuState};

/// Notion draws the pill as a 5.5 px outline in its own background colour,
/// which reads as horizontal padding around the typed trigger.
pub(crate) const PAGE_MENTION_PILL_PADDING_X: f32 = 5.5;
pub(crate) const PAGE_MENTION_PILL_RADIUS: f32 = 0.5;

#[cfg(test)]
mod tests;

impl PageMentionController {
    /// The completion drawn after the caret: what the selected row would add
    /// to the query. Notion shows it whenever the label starts with the query,
    /// so a bare `@` ghosts the whole label.
    pub(crate) fn input_ghost(
        &self,
        block_id: &str,
        clock: &PageMentionClock,
        appearance_mode: AppearanceMode,
    ) -> Option<TextInputGhost> {
        let menu = self.menu_for_block(block_id)?;
        let label = self.selected_row(menu, clock)?.label();
        let completion = page_mention_ghost_completion(&label, &menu.query)?;
        Some(TextInputGhost {
            offset: page_mention_input_range(menu).end,
            text: completion.into(),
            color: page_mention_ghost_foreground(appearance_mode),
        })
    }

    /// The typed `@query` the pill covers, in block text offsets.
    pub(crate) fn pill_range(&self, block_id: &str) -> Option<Range<usize>> {
        self.menu_for_block(block_id).map(page_mention_input_range)
    }
}

/// What `label` adds to what has been typed, or `None` when the label is not a
/// continuation of the query.
pub(super) fn page_mention_ghost_completion(label: &str, query: &str) -> Option<String> {
    if !label.is_char_boundary(query.len()) {
        return None;
    }
    let (typed, completion) = label.split_at(query.len());
    (typed.eq_ignore_ascii_case(query) && !completion.is_empty()).then(|| completion.to_string())
}

/// `@` plus the query: the span the caret is still editing.
fn page_mention_input_range(menu: &PageMentionMenuState) -> Range<usize> {
    menu.trigger_offset..menu.trigger_offset + '@'.len_utf8() + menu.query.len()
}

/// Lay the pill under `range`, keeping whatever styling already applies there
/// so a mention typed inside bold text stays bold. `highlights` is sorted and
/// non-overlapping, and so is the result.
pub(in crate::ui::board_workspace::page::editor) fn page_mention_input_pill_highlights(
    highlights: Vec<TextInputHighlight>,
    range: Range<usize>,
    text_color: Hsla,
    appearance_mode: AppearanceMode,
) -> Vec<TextInputHighlight> {
    let mut painted = Vec::with_capacity(highlights.len() + 2);
    let mut covered = range.start;
    for highlight in highlights {
        if highlight.range.end <= range.start || highlight.range.start >= range.end {
            painted.push(highlight);
            continue;
        }
        if covered < highlight.range.start {
            painted.push(pill(
                bare(covered..highlight.range.start, text_color),
                appearance_mode,
            ));
        }
        covered = highlight.range.end.min(range.end).max(covered);
        for piece in split_highlight(highlight, &range) {
            let inside = piece.range.start >= range.start && piece.range.end <= range.end;
            painted.push(if inside {
                pill(piece, appearance_mode)
            } else {
                piece
            });
        }
    }
    if covered < range.end {
        painted.push(pill(bare(covered..range.end, text_color), appearance_mode));
    }
    painted.sort_by_key(|highlight| highlight.range.start);
    painted
}

/// One highlight cut at the pill's edges, keeping every other field.
fn split_highlight(highlight: TextInputHighlight, range: &Range<usize>) -> Vec<TextInputHighlight> {
    let mut edges = vec![
        highlight.range.start,
        range
            .start
            .clamp(highlight.range.start, highlight.range.end),
        range.end.clamp(highlight.range.start, highlight.range.end),
        highlight.range.end,
    ];
    edges.dedup();
    edges
        .windows(2)
        .map(|edge| TextInputHighlight {
            range: edge[0]..edge[1],
            ..highlight.clone()
        })
        .collect()
}

fn bare(range: Range<usize>, text_color: Hsla) -> TextInputHighlight {
    TextInputHighlight {
        range,
        color: text_color,
        ..TextInputHighlight::default()
    }
}

fn pill(mut highlight: TextInputHighlight, appearance_mode: AppearanceMode) -> TextInputHighlight {
    highlight.background = Some(page_mention_input_pill_background(appearance_mode));
    highlight.background_corner_radius = px(PAGE_MENTION_PILL_RADIUS);
    highlight.background_padding_x = px(PAGE_MENTION_PILL_PADDING_X);
    highlight
}
