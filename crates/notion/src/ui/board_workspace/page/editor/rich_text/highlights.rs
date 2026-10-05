use gpui::{
    px, rgb, FontStyle, FontWeight, HighlightStyle, Hsla, SharedString, StrikethroughStyle,
    StyledText, TextStyle, UnderlineStyle,
};
use gpui_components::text_input::TextInputHighlight;

use crate::model::{
    CardPageSimpleTableCell, CardPageTextAnnotationSpan, PageTextAnnotation, PageTextColor,
};
use crate::ui::{
    alpha, page_block_foreground, page_block_primary_hex, AppearanceMode, CardPageBlockColor,
    CardPageEditableBlock, LoadedCardPageSimpleTableAnnotationRun,
};

pub(crate) fn page_text_input_highlights(
    editable: &CardPageEditableBlock,
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
) -> Vec<TextInputHighlight> {
    let checked = editable
        .to_do_state()
        .is_some_and(|state| state.is_checked());
    page_text_highlights(
        &editable.text,
        &editable.annotations,
        checked,
        appearance_mode,
        block_color,
    )
}

pub(in crate::ui::board_workspace::page::editor) fn page_simple_table_text_input_highlights(
    cell: &CardPageSimpleTableCell,
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
) -> Vec<TextInputHighlight> {
    page_text_highlights(
        cell.text(),
        cell.annotations(),
        false,
        appearance_mode,
        block_color,
    )
}

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page::editor) struct PageStaticTextFont {
    pub(in crate::ui::board_workspace::page::editor) size: f32,
    pub(in crate::ui::board_workspace::page::editor) line_height: f32,
    pub(in crate::ui::board_workspace::page::editor) weight: FontWeight,
}

pub(in crate::ui::board_workspace::page::editor) fn page_static_rich_text(
    text: SharedString,
    runs: &[LoadedCardPageSimpleTableAnnotationRun],
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
    font: PageStaticTextFont,
) -> StyledText {
    let PageStaticTextFont {
        size: font_size,
        line_height,
        weight: font_weight,
    } = font;
    let font_overrides = runs
        .iter()
        .filter(|run| run.code)
        .map(|run| (run.range.clone(), SharedString::from("Iosevka Fixed")))
        .collect::<Vec<_>>();
    let highlight_styles = runs.iter().map(|run| {
        let color = run.text_color.map_or_else(
            || {
                if run.code {
                    inline_code_foreground(appearance_mode)
                } else {
                    page_block_foreground(block_color, appearance_mode)
                }
            },
            |color| page_text_color(color, appearance_mode),
        );
        let background = run
            .background_color
            .and_then(|color| page_background_color(color, appearance_mode))
            .or_else(|| run.code.then(|| code_background(appearance_mode)));
        (
            run.range.clone(),
            HighlightStyle {
                color: Some(color),
                background_color: background,
                font_weight: run.bold.then_some(FontWeight::BOLD),
                font_style: run.italic.then_some(FontStyle::Italic),
                underline: (run.link || run.underline).then_some(UnderlineStyle {
                    color: Some(color),
                    thickness: px(1.0),
                    wavy: false,
                }),
                strikethrough: run.strikethrough.then_some(StrikethroughStyle {
                    color: Some(color),
                    thickness: px(1.0),
                }),
                ..Default::default()
            },
        )
    });
    let default_style = TextStyle {
        color: page_block_foreground(block_color, appearance_mode),
        font_size: px(font_size).into(),
        line_height: px(line_height).into(),
        font_weight,
        ..Default::default()
    };
    StyledText::new(text)
        .with_default_highlights(&default_style, highlight_styles)
        .with_font_family_overrides(font_overrides)
}

fn page_text_highlights(
    text: &str,
    spans: &[CardPageTextAnnotationSpan],
    checked: bool,
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
) -> Vec<TextInputHighlight> {
    let mut boundaries = spans
        .iter()
        .flat_map(|span| [span.start_utf8, span.end_utf8])
        .collect::<Vec<_>>();
    if checked && !text.is_empty() {
        boundaries.extend([0, text.len()]);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    boundaries
        .windows(2)
        .filter_map(|boundary| {
            let range = boundary[0]..boundary[1];
            let annotations = spans
                .iter()
                .filter(|span| span.start_utf8 <= range.start && span.end_utf8 >= range.end)
                .map(|span| &span.annotation)
                .collect::<Vec<_>>();
            if annotations.is_empty() && !checked {
                return None;
            }
            let mut highlight =
                highlight_for_annotations(range, &annotations, appearance_mode, block_color);
            if checked {
                highlight.color = highlight.color.opacity(0.7);
                highlight.strikethrough = Some(StrikethroughStyle {
                    color: Some(
                        Hsla::from(rgb(page_block_primary_hex(appearance_mode))).opacity(0.5),
                    ),
                    thickness: px(1.0),
                });
            }
            Some(highlight)
        })
        .collect()
}

fn highlight_for_annotations(
    range: std::ops::Range<usize>,
    annotations: &[&PageTextAnnotation],
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
) -> TextInputHighlight {
    if has_annotation(annotations, |annotation| {
        matches!(annotation, PageTextAnnotation::Mention(_))
    }) {
        return mention_highlight(range, appearance_mode);
    }
    let code = has_annotation(annotations, |annotation| {
        matches!(annotation, PageTextAnnotation::Code)
    });
    let color = selected_foreground(annotations).map_or_else(
        || {
            if code {
                inline_code_foreground(appearance_mode)
            } else {
                page_block_foreground(block_color, appearance_mode)
            }
        },
        |color| page_text_color(color, appearance_mode),
    );

    decorated_text_highlight(range, annotations, appearance_mode, color, code)
}

/// The token is painted as an atom by the text input; this highlight keeps the
/// run colour consistent when atoms are unavailable.
fn mention_highlight(
    range: std::ops::Range<usize>,
    appearance_mode: AppearanceMode,
) -> TextInputHighlight {
    TextInputHighlight {
        range,
        color: page_mention_foreground(appearance_mode),
        ..TextInputHighlight::default()
    }
}

fn decorated_text_highlight(
    range: std::ops::Range<usize>,
    annotations: &[&PageTextAnnotation],
    appearance_mode: AppearanceMode,
    color: Hsla,
    code: bool,
) -> TextInputHighlight {
    let linked = has_annotation(annotations, |annotation| {
        matches!(annotation, PageTextAnnotation::Link(_))
    });
    TextInputHighlight {
        range,
        color,
        background: selected_background(annotations)
            .and_then(|color| page_background_color(color, appearance_mode))
            .or_else(|| code.then(|| code_background(appearance_mode))),
        font_family: code.then(|| SharedString::from("Iosevka Fixed")),
        background_corner_radius: if code { px(4.0) } else { px(0.0) },
        background_padding_x: if code { px(3.0) } else { px(0.0) },
        background_inset_y: if code { px(1.0) } else { px(0.0) },
        font_weight: has_annotation(annotations, |annotation| {
            matches!(annotation, PageTextAnnotation::Bold)
        })
        .then_some(FontWeight::BOLD),
        font_style: has_annotation(annotations, |annotation| {
            matches!(annotation, PageTextAnnotation::Italic)
        })
        .then_some(FontStyle::Italic),
        underline: (linked
            || has_annotation(annotations, |annotation| {
                matches!(annotation, PageTextAnnotation::Underline)
            }))
        .then_some(UnderlineStyle {
            color: Some(color),
            thickness: px(1.0),
            wavy: false,
        }),
        strikethrough: has_annotation(annotations, |annotation| {
            matches!(annotation, PageTextAnnotation::Strike)
        })
        .then_some(StrikethroughStyle {
            color: Some(color),
            thickness: px(1.0),
        }),
        monospace: code,
    }
}

fn selected_foreground(annotations: &[&PageTextAnnotation]) -> Option<PageTextColor> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match annotation {
            PageTextAnnotation::TextColor(color) => Some(*color),
            _ => None,
        })
}

fn selected_background(annotations: &[&PageTextAnnotation]) -> Option<PageTextColor> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match annotation {
            PageTextAnnotation::BackgroundColor(color) => Some(*color),
            _ => None,
        })
}

fn has_annotation(
    annotations: &[&PageTextAnnotation],
    predicate: impl Fn(&PageTextAnnotation) -> bool,
) -> bool {
    annotations.iter().any(|annotation| predicate(annotation))
}

pub(super) fn page_text_color(color: PageTextColor, appearance_mode: AppearanceMode) -> Hsla {
    let hex = match color {
        PageTextColor::Default => match appearance_mode {
            AppearanceMode::Light => 0x2c2c2b,
            AppearanceMode::Dark => 0xf0efed,
        },
        PageTextColor::Gray => 0x7d7a75,
        PageTextColor::Brown => 0x9f765a,
        PageTextColor::Orange => 0xd27b2d,
        PageTextColor::Yellow => 0xcb9434,
        PageTextColor::Green => 0x50946e,
        PageTextColor::Blue => 0x387dc9,
        PageTextColor::Purple => 0x9a6bb4,
        PageTextColor::Pink => 0xc14c8a,
        PageTextColor::Red => 0xcf5148,
    };
    rgb(hex).into()
}

pub(super) fn page_background_color(
    color: PageTextColor,
    appearance_mode: AppearanceMode,
) -> Option<Hsla> {
    let hex = match (appearance_mode, color) {
        (_, PageTextColor::Default) => return None,
        (AppearanceMode::Light, PageTextColor::Gray) => 0xf0efed,
        (AppearanceMode::Light, PageTextColor::Brown) => 0xf5ede9,
        (AppearanceMode::Light, PageTextColor::Orange) => 0xfbebde,
        (AppearanceMode::Light, PageTextColor::Yellow) => 0xf9f3dc,
        (AppearanceMode::Light, PageTextColor::Green) => 0xe8f1ec,
        (AppearanceMode::Light, PageTextColor::Blue) => 0xe5f2fc,
        (AppearanceMode::Light, PageTextColor::Purple) => 0xf3ebf9,
        (AppearanceMode::Light, PageTextColor::Pink) => 0xfae9f1,
        (AppearanceMode::Light, PageTextColor::Red) => 0xfce9e7,
        (AppearanceMode::Dark, PageTextColor::Gray) => 0x383836,
        (AppearanceMode::Dark, PageTextColor::Brown) => 0x45362d,
        (AppearanceMode::Dark, PageTextColor::Orange) => 0x53361f,
        (AppearanceMode::Dark, PageTextColor::Yellow) => 0x504425,
        (AppearanceMode::Dark, PageTextColor::Green) => 0x263d30,
        (AppearanceMode::Dark, PageTextColor::Blue) => 0x233850,
        (AppearanceMode::Dark, PageTextColor::Purple) => 0x3c2d47,
        (AppearanceMode::Dark, PageTextColor::Pink) => 0x4e2b3c,
        (AppearanceMode::Dark, PageTextColor::Red) => 0x502c29,
    };
    Some(rgb(hex).into())
}

/// Notion's tertiary ink, used for the ghost completion after the caret.
pub(crate) fn page_mention_ghost_foreground(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => rgb(0xa19e99).into(),
        AppearanceMode::Dark => rgb(0x7d7a75).into(),
    }
}

/// The temporary-input pill behind a trigger that is still being typed. The
/// light value is measured; the dark one is the same idea in reverse, as
/// Notion's dark capture is not part of the reference.
pub(crate) fn page_mention_input_pill_background(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x422303, 0.03),
        AppearanceMode::Dark => alpha(0xffffff, 0.045),
    }
}

/// Notion's secondary text colour, which every inline mention uses.
pub(crate) fn page_mention_foreground(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => rgb(0x7d7a75).into(),
        AppearanceMode::Dark => rgb(0x9b9b9b).into(),
    }
}

fn code_background(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => rgb(0xededeb).into(),
        AppearanceMode::Dark => rgb(0x383836).into(),
    }
}

fn inline_code_foreground(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => rgb(0xeb5757).into(),
        AppearanceMode::Dark => rgb(0xff7369).into(),
    }
}
