use std::ops::Range;

use gpui::{px, rgb, FontWeight, Hsla};
use gpui_components::text_input::TextInputHighlight;

use super::{
    page_mention_ghost_completion, page_mention_input_pill_highlights, PAGE_MENTION_PILL_PADDING_X,
    PAGE_MENTION_PILL_RADIUS,
};
use crate::ui::AppearanceMode;

fn text_color() -> Hsla {
    rgb(0x2c2c2b).into()
}

fn bold(range: Range<usize>) -> TextInputHighlight {
    TextInputHighlight {
        range,
        color: rgb(0x111111).into(),
        font_weight: Some(FontWeight::BOLD),
        ..TextInputHighlight::default()
    }
}

fn painted(highlights: Vec<TextInputHighlight>) -> Vec<(Range<usize>, bool)> {
    page_mention_input_pill_highlights(highlights, 6..10, text_color(), AppearanceMode::Light)
        .into_iter()
        .map(|highlight| (highlight.range, highlight.background.is_some()))
        .collect()
}

#[test]
fn plain_text_under_the_pill_gets_one_background_run() {
    let highlights =
        page_mention_input_pill_highlights(Vec::new(), 6..10, text_color(), AppearanceMode::Light);
    assert_eq!(highlights.len(), 1);
    assert_eq!(highlights[0].range, 6..10);
    assert_eq!(highlights[0].color, text_color());
    assert_eq!(
        highlights[0].background_padding_x,
        px(PAGE_MENTION_PILL_PADDING_X)
    );
    assert_eq!(
        highlights[0].background_corner_radius,
        px(PAGE_MENTION_PILL_RADIUS)
    );
}

#[test]
fn styling_under_the_pill_survives_and_is_split_at_its_edges() {
    let highlights = page_mention_input_pill_highlights(
        vec![bold(4..12)],
        6..10,
        text_color(),
        AppearanceMode::Light,
    );
    assert_eq!(
        highlights
            .iter()
            .map(|highlight| (highlight.range.clone(), highlight.background.is_some()))
            .collect::<Vec<_>>(),
        vec![(4..6, false), (6..10, true), (10..12, false)]
    );
    assert!(highlights
        .iter()
        .all(|highlight| highlight.font_weight == Some(FontWeight::BOLD)));
}

#[test]
fn text_beside_a_styled_run_still_gets_the_pill() {
    assert_eq!(
        painted(vec![bold(8..12)]),
        vec![(6..8, true), (8..10, true), (10..12, false)]
    );
}

#[test]
fn highlights_outside_the_pill_are_untouched() {
    assert_eq!(
        painted(vec![bold(0..3), bold(12..14)]),
        vec![(0..3, false), (6..10, true), (12..14, false)]
    );
}

#[test]
fn a_label_that_continues_the_query_ghosts_the_rest() {
    assert_eq!(
        page_mention_ghost_completion("Today", "tod"),
        Some("ay".to_string())
    );
    assert_eq!(
        page_mention_ghost_completion("Today", ""),
        Some("Today".to_string())
    );
}

#[test]
fn a_label_that_does_not_continue_the_query_has_no_ghost() {
    assert_eq!(page_mention_ghost_completion("Next Monday", "sep 7"), None);
    assert_eq!(page_mention_ghost_completion("Today", "today"), None);
    assert_eq!(page_mention_ghost_completion("Today", "todayish"), None);
}
