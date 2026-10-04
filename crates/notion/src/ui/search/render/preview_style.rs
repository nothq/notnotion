use std::ops::Range;

use gpui::{
    FontStyle, HighlightStyle, SharedString, StrikethroughStyle, StyledText, UnderlineStyle,
};

use crate::model::{
    CardPageBlockKind, CardPageEditableBlock, CardPageTextAnnotationSpan, PageShellIcon,
    PageTextAnnotation, PageTextColor,
};
use crate::ui::{div, px, rgb, AppearanceMode, Div, FontWeight, ParentElement, Styled};

use super::{
    SEARCH_PREVIEW_DARK_BODY_FILL, SEARCH_PREVIEW_DARK_HEADER_FILL, SEARCH_PREVIEW_LIGHT_BODY_FILL,
    SEARCH_PREVIEW_LIGHT_HEADER_FILL,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NotionSearchPreviewBlockRole {
    Paragraph,
    HeadingLarge,
    HeadingMedium,
    HeadingSmall,
    Bulleted,
    Numbered,
    ToDo,
    Toggle,
    PageLink,
    Callout,
    Quote,
    Code,
}

impl NotionSearchPreviewBlockRole {
    pub(super) const fn is_heading(self) -> bool {
        matches!(
            self,
            Self::HeadingLarge | Self::HeadingMedium | Self::HeadingSmall
        )
    }

    pub(super) const fn top_margin(self) -> f32 {
        match self {
            Self::HeadingLarge => 22.0,
            Self::HeadingMedium | Self::HeadingSmall => 18.0,
            _ => 0.0,
        }
    }

    pub(super) const fn bottom_margin(self) -> f32 {
        match self {
            Self::Paragraph => 0.0,
            Self::HeadingLarge | Self::HeadingMedium | Self::HeadingSmall => 12.0,
            Self::Callout | Self::Quote | Self::Code => 6.0,
            Self::Bulleted | Self::Numbered | Self::ToDo | Self::Toggle | Self::PageLink => 0.0,
        }
    }

    pub(super) const fn selector_segment(self) -> &'static str {
        match self {
            Self::Paragraph => "text",
            Self::HeadingLarge => "heading-large",
            Self::HeadingMedium => "heading-medium",
            Self::HeadingSmall => "heading-small",
            Self::Bulleted => "bulleted-list",
            Self::Numbered => "numbered-list",
            Self::ToDo => "to-do-list",
            Self::Toggle => "toggle-list",
            Self::PageLink => "page-link",
            Self::Callout => "callout",
            Self::Quote => "quote",
            Self::Code => "code",
        }
    }
}

pub(super) const fn notion_search_preview_block_role(
    kind: CardPageBlockKind,
) -> NotionSearchPreviewBlockRole {
    match kind {
        CardPageBlockKind::Text => NotionSearchPreviewBlockRole::Paragraph,
        CardPageBlockKind::SubHeader => NotionSearchPreviewBlockRole::HeadingLarge,
        CardPageBlockKind::SubSubHeader => NotionSearchPreviewBlockRole::HeadingMedium,
        CardPageBlockKind::Heading3 | CardPageBlockKind::Heading4 => {
            NotionSearchPreviewBlockRole::HeadingSmall
        }
        CardPageBlockKind::BulletedList => NotionSearchPreviewBlockRole::Bulleted,
        CardPageBlockKind::NumberedList => NotionSearchPreviewBlockRole::Numbered,
        CardPageBlockKind::ToDoList => NotionSearchPreviewBlockRole::ToDo,
        CardPageBlockKind::ToggleList => NotionSearchPreviewBlockRole::Toggle,
        CardPageBlockKind::PageLink => NotionSearchPreviewBlockRole::PageLink,
        CardPageBlockKind::Callout => NotionSearchPreviewBlockRole::Callout,
        CardPageBlockKind::Quote => NotionSearchPreviewBlockRole::Quote,
        CardPageBlockKind::Code => NotionSearchPreviewBlockRole::Code,
    }
}

pub(super) const fn notion_search_preview_text_metrics(
    role: NotionSearchPreviewBlockRole,
) -> (f32, f32, FontWeight) {
    match role {
        NotionSearchPreviewBlockRole::HeadingLarge => (15.0, 20.0, FontWeight::SEMIBOLD),
        NotionSearchPreviewBlockRole::HeadingMedium => (14.0, 18.0, FontWeight::SEMIBOLD),
        NotionSearchPreviewBlockRole::HeadingSmall => (13.0, 18.0, FontWeight::SEMIBOLD),
        NotionSearchPreviewBlockRole::Code => (13.0, 20.0, FontWeight::NORMAL),
        NotionSearchPreviewBlockRole::Paragraph
        | NotionSearchPreviewBlockRole::Bulleted
        | NotionSearchPreviewBlockRole::Numbered
        | NotionSearchPreviewBlockRole::ToDo
        | NotionSearchPreviewBlockRole::Toggle
        | NotionSearchPreviewBlockRole::PageLink
        | NotionSearchPreviewBlockRole::Callout
        | NotionSearchPreviewBlockRole::Quote => (13.0, 18.0, FontWeight::NORMAL),
    }
}

pub(super) fn notion_search_preview_text_marker(
    marker: impl Into<SharedString>,
    line_height: f32,
) -> Div {
    div()
        .w(px(16.0))
        .h(px(line_height))
        .flex_none()
        .flex()
        .items_center()
        .child(marker.into())
}

pub(super) fn notion_search_preview_styled_text(
    editable: &CardPageEditableBlock,
    appearance_mode: AppearanceMode,
) -> StyledText {
    let font_overrides = notion_search_preview_code_ranges(editable)
        .into_iter()
        .map(|range| (range, SharedString::from("Iosevka Fixed")))
        .collect::<Vec<_>>();
    StyledText::new(editable.text.clone())
        .with_highlights(notion_search_preview_annotation_highlights(
            &editable.annotations,
            appearance_mode,
        ))
        .with_font_family_overrides(font_overrides)
}

type AnnotationHighlights = Vec<(Range<usize>, HighlightStyle)>;

pub(super) fn notion_search_preview_annotation_highlights(
    spans: &[CardPageTextAnnotationSpan],
    appearance_mode: AppearanceMode,
) -> AnnotationHighlights {
    let mut boundaries = spans
        .iter()
        .flat_map(|span| [span.start_utf8, span.end_utf8])
        .collect::<Vec<_>>();
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
            (!annotations.is_empty()).then(|| {
                (
                    range,
                    notion_search_preview_highlight_style(&annotations, appearance_mode),
                )
            })
        })
        .collect()
}

fn notion_search_preview_highlight_style(
    annotations: &[&PageTextAnnotation],
    appearance_mode: AppearanceMode,
) -> HighlightStyle {
    let code = annotations
        .iter()
        .any(|annotation| matches!(annotation, PageTextAnnotation::Code));
    let color = notion_search_preview_highlight_text_color(annotations, appearance_mode)
        .or_else(|| code.then(|| notion_search_preview_inline_code_text(appearance_mode)));
    let background_color = notion_search_preview_highlight_background(annotations, appearance_mode)
        .or_else(|| code.then(|| notion_search_preview_inline_code_background(appearance_mode)));
    let decoration_color =
        color.unwrap_or_else(|| notion_search_preview_inline_code_text(appearance_mode));
    let linked = annotations
        .iter()
        .any(|annotation| matches!(annotation, PageTextAnnotation::Link(_)));
    HighlightStyle {
        color,
        background_color,
        font_weight: annotations
            .iter()
            .any(|annotation| matches!(annotation, PageTextAnnotation::Bold))
            .then_some(FontWeight::BOLD),
        font_style: annotations
            .iter()
            .any(|annotation| matches!(annotation, PageTextAnnotation::Italic))
            .then_some(FontStyle::Italic),
        underline: (linked
            || annotations
                .iter()
                .any(|annotation| matches!(annotation, PageTextAnnotation::Underline)))
        .then_some(UnderlineStyle {
            color: Some(decoration_color),
            thickness: px(1.0),
            wavy: false,
        }),
        strikethrough: annotations
            .iter()
            .any(|annotation| matches!(annotation, PageTextAnnotation::Strike))
            .then_some(StrikethroughStyle {
                color: Some(decoration_color),
                thickness: px(1.0),
            }),
        ..Default::default()
    }
}

fn notion_search_preview_highlight_text_color(
    annotations: &[&PageTextAnnotation],
    appearance_mode: AppearanceMode,
) -> Option<gpui::Hsla> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match annotation {
            PageTextAnnotation::TextColor(color) => {
                notion_search_preview_annotation_text_color(*color, appearance_mode)
            }
            _ => None,
        })
}

fn notion_search_preview_highlight_background(
    annotations: &[&PageTextAnnotation],
    appearance_mode: AppearanceMode,
) -> Option<gpui::Hsla> {
    annotations
        .iter()
        .rev()
        .find_map(|annotation| match annotation {
            PageTextAnnotation::BackgroundColor(color) => {
                notion_search_preview_annotation_background(*color, appearance_mode)
            }
            _ => None,
        })
}

pub(super) fn notion_search_preview_code_ranges(
    editable: &CardPageEditableBlock,
) -> Vec<Range<usize>> {
    if editable.kind == CardPageBlockKind::Code {
        return (!editable.text.is_empty())
            .then_some(0..editable.text.len())
            .into_iter()
            .collect();
    }
    let code_spans = editable
        .annotations
        .iter()
        .filter(|span| matches!(&span.annotation, PageTextAnnotation::Code))
        .collect::<Vec<_>>();
    let mut boundaries = code_spans
        .iter()
        .flat_map(|span| [span.start_utf8, span.end_utf8])
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
        .windows(2)
        .filter_map(|boundary| {
            let range = boundary[0]..boundary[1];
            code_spans
                .iter()
                .any(|span| span.start_utf8 <= range.start && span.end_utf8 >= range.end)
                .then_some(range)
        })
        .collect()
}

fn notion_search_preview_annotation_text_color(
    color: PageTextColor,
    _appearance_mode: AppearanceMode,
) -> Option<gpui::Hsla> {
    let hex = match color {
        PageTextColor::Default => return None,
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
    Some(rgb(hex).into())
}

fn notion_search_preview_annotation_background(
    color: PageTextColor,
    appearance_mode: AppearanceMode,
) -> Option<gpui::Hsla> {
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

fn notion_search_preview_inline_code_text(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0x787774,
        AppearanceMode::Dark => 0xada9a3,
    })
    .into()
}

fn notion_search_preview_inline_code_background(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0xededeb,
        AppearanceMode::Dark => 0x383836,
    })
    .into()
}

pub(super) const fn notion_search_preview_header_fill(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => SEARCH_PREVIEW_LIGHT_HEADER_FILL,
        AppearanceMode::Dark => SEARCH_PREVIEW_DARK_HEADER_FILL,
    }
}

pub(super) const fn notion_search_preview_body_fill(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => SEARCH_PREVIEW_LIGHT_BODY_FILL,
        AppearanceMode::Dark => SEARCH_PREVIEW_DARK_BODY_FILL,
    }
}

pub(super) fn notion_search_preview_has_explicit_icon(icon: &PageShellIcon) -> bool {
    match icon.kind.as_str() {
        "emoji" | "external" | "custom" => true,
        "named" => !icon.value.is_empty() && icon.value != "page",
        _ => false,
    }
}
