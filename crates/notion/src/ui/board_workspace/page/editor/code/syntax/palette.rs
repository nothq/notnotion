use gpui::{rgb, Hsla};
use gpui_components::text_input::TextInputHighlight;
use zed_syntax::{SyntaxHighlight, SyntaxLanguage, SyntaxTokenKind};

use super::AppearanceMode;

pub(super) fn page_code_token_highlights(
    highlights: &[SyntaxHighlight],
    language: SyntaxLanguage,
    appearance: AppearanceMode,
) -> Vec<TextInputHighlight> {
    highlights
        .iter()
        .filter_map(|highlight| {
            let color = token_color(highlight.kind, language, appearance)?;
            Some(TextInputHighlight {
                range: highlight.range.clone(),
                color,
                ..Default::default()
            })
        })
        .collect()
}

fn token_color(
    kind: SyntaxTokenKind,
    language: SyntaxLanguage,
    appearance: AppearanceMode,
) -> Option<Hsla> {
    if language == SyntaxLanguage::Nix
        && matches!(kind, SyntaxTokenKind::Property | SyntaxTokenKind::Variable)
    {
        return None;
    }
    let hex = match appearance {
        AppearanceMode::Light => light_token_hex(kind),
        AppearanceMode::Dark => dark_token_hex(kind),
    }?;
    Some(rgb(hex).into())
}

const fn light_token_hex(kind: SyntaxTokenKind) -> Option<u32> {
    match kind {
        SyntaxTokenKind::Comment => Some(0x708090),
        SyntaxTokenKind::Punctuation => Some(0x999999),
        SyntaxTokenKind::Property | SyntaxTokenKind::Literal => Some(0x990055),
        SyntaxTokenKind::String => Some(0x669900),
        SyntaxTokenKind::Operator => Some(0x9a6e3a),
        SyntaxTokenKind::Keyword => Some(0x0077aa),
        SyntaxTokenKind::Function => Some(0xdd4a68),
        SyntaxTokenKind::Variable => Some(0xee9900),
    }
}

const fn dark_token_hex(kind: SyntaxTokenKind) -> Option<u32> {
    match kind {
        SyntaxTokenKind::Comment => Some(0x7d7a75),
        SyntaxTokenKind::Punctuation => Some(0x9b9a97),
        SyntaxTokenKind::Property | SyntaxTokenKind::Literal => Some(0xc14c8a),
        SyntaxTokenKind::String => Some(0x50946e),
        SyntaxTokenKind::Operator => Some(0x9f765a),
        SyntaxTokenKind::Keyword => Some(0x387dc9),
        SyntaxTokenKind::Function => Some(0xcf5148),
        SyntaxTokenKind::Variable => Some(0xd27b2d),
    }
}
