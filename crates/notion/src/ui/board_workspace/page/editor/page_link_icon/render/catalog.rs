use std::sync::OnceLock;

use crate::model::NotionCustomEmoji;

use super::{
    PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX, PAGE_LINK_EMOJI_COLUMN_COUNT,
    PAGE_LINK_NAMED_ICON_COLUMN_COUNT,
};

#[derive(Clone)]
pub(super) enum PageLinkPickerEmojiSearchMatch {
    BuiltIn(&'static emojis::Emoji),
    Custom(NotionCustomEmoji),
}

struct PageLinkPickerEmojiEntry {
    emoji: &'static emojis::Emoji,
    search_text: String,
}

pub(super) struct PageLinkPickerEmojiSection {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) emojis: Vec<&'static emojis::Emoji>,
}

#[derive(Clone, Copy)]
pub(super) enum PageLinkPickerEmojiListItem {
    SectionHeader {
        section_index: usize,
    },
    EmojiRow {
        section_index: usize,
        row_index: usize,
        emoji_start_index: usize,
    },
}

#[derive(Clone, Copy)]
pub(super) struct PageLinkPickerCategorySpec {
    pub(super) label: &'static str,
    pub(super) section_index: Option<usize>,
}

pub(super) fn named_icon_item_count(
    match_count: usize,
    recent_count: usize,
    query_is_empty: bool,
) -> usize {
    if match_count == 0 {
        return 1;
    }
    let rows = match_count.div_ceil(PAGE_LINK_NAMED_ICON_COLUMN_COUNT);
    if !query_is_empty {
        return rows;
    }
    rows + 1 + usize::from(recent_count > 0) * 2
}

pub(super) fn emoji_item_count(query: &str, custom_emojis: &[NotionCustomEmoji]) -> usize {
    if query.trim().is_empty() {
        emoji_list_items().len() + 1 + custom_emoji_row_count(custom_emojis.len())
    } else {
        emoji_search_matches(query, custom_emojis)
            .len()
            .div_ceil(PAGE_LINK_EMOJI_COLUMN_COUNT)
    }
}

pub(super) fn emoji_section_item_index(section_index: usize) -> Option<usize> {
    if section_index == PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX {
        return Some(emoji_list_items().len());
    }
    emoji_list_items().iter().position(|item| {
        matches!(
            item,
            PageLinkPickerEmojiListItem::SectionHeader {
                section_index: item_section_index,
            } if *item_section_index == section_index
        )
    })
}

pub(super) fn category_index_for_item(item_index: usize) -> Option<usize> {
    let section_index = match emoji_list_item(item_index, usize::MAX)? {
        PageLinkPickerEmojiListItem::SectionHeader { section_index }
        | PageLinkPickerEmojiListItem::EmojiRow { section_index, .. } => section_index,
    };
    Some(match section_index {
        0 | 1 => 0,
        _ => section_index - 1,
    })
}

pub(super) fn emoji_search_matches(
    query: &str,
    custom_emojis: &[NotionCustomEmoji],
) -> Vec<PageLinkPickerEmojiSearchMatch> {
    let query = query.trim().to_lowercase();
    custom_emojis
        .iter()
        .filter(|emoji| emoji.name.to_lowercase().contains(&query))
        .cloned()
        .map(PageLinkPickerEmojiSearchMatch::Custom)
        .chain(
            matching_emojis(&query)
                .into_iter()
                .map(PageLinkPickerEmojiSearchMatch::BuiltIn),
        )
        .take(96)
        .collect()
}

pub(super) fn emoji_list_item(
    item_index: usize,
    custom_emoji_count: usize,
) -> Option<PageLinkPickerEmojiListItem> {
    if let Some(item) = emoji_list_items().get(item_index) {
        return Some(*item);
    }
    let custom_header_index = emoji_list_items().len();
    if item_index == custom_header_index {
        return Some(PageLinkPickerEmojiListItem::SectionHeader {
            section_index: PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX,
        });
    }
    let row_index = item_index.checked_sub(custom_header_index + 1)?;
    (row_index < custom_emoji_row_count(custom_emoji_count)).then_some(
        PageLinkPickerEmojiListItem::EmojiRow {
            section_index: PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX,
            row_index,
            emoji_start_index: row_index * PAGE_LINK_EMOJI_COLUMN_COUNT,
        },
    )
}

pub(super) fn emoji_sections() -> &'static [PageLinkPickerEmojiSection] {
    static SECTIONS: OnceLock<Vec<PageLinkPickerEmojiSection>> = OnceLock::new();
    SECTIONS
        .get_or_init(|| {
            let explicit = |glyphs: &[&str]| {
                glyphs
                    .iter()
                    .filter_map(|glyph| emojis::get(glyph))
                    .collect::<Vec<_>>()
            };
            let groups = [
                ("people", "People", emojis::Group::PeopleAndBody),
                (
                    "animals",
                    "Animals & Nature",
                    emojis::Group::AnimalsAndNature,
                ),
                ("food", "Food & Drink", emojis::Group::FoodAndDrink),
                ("activities", "Activities", emojis::Group::Activities),
                ("travel", "Travel & Places", emojis::Group::TravelAndPlaces),
                ("objects", "Objects", emojis::Group::Objects),
                ("symbols", "Symbols", emojis::Group::Symbols),
                ("flags", "Flags", emojis::Group::Flags),
            ];
            let mut sections = vec![
                PageLinkPickerEmojiSection {
                    id: "recent",
                    label: "Recent",
                    emojis: explicit(&["😃", "😀", "💡", "😄"]),
                },
                PageLinkPickerEmojiSection {
                    id: "callout",
                    label: "Callout",
                    emojis: explicit(&[
                        "💡", "👉", "☝", "👌", "🔑", "🚧", "⚠️", "🔥", "📌", "✂️", "❓", "🚫",
                        "⛔", "⏰", "☎️", "🚨", "♻️", "✅", "🔒", "📎", "📖", "🗣", "➡️", "📢", "🛠",
                        "⚙",
                    ]),
                },
            ];
            for (id, label, group) in groups {
                sections.push(PageLinkPickerEmojiSection {
                    id,
                    label,
                    emojis: emojis::iter()
                        .filter(|emoji| emoji.group() == group)
                        .take(240)
                        .collect(),
                });
            }
            sections.push(PageLinkPickerEmojiSection {
                id: "custom",
                label: "Custom",
                emojis: Vec::new(),
            });
            sections
        })
        .as_slice()
}

pub(super) fn category_specs() -> &'static [PageLinkPickerCategorySpec] {
    const SPECS: &[PageLinkPickerCategorySpec] = &[
        PageLinkPickerCategorySpec {
            label: "recent",
            section_index: Some(0),
        },
        PageLinkPickerCategorySpec {
            label: "People & Body",
            section_index: Some(2),
        },
        PageLinkPickerCategorySpec {
            label: "Animals & Nature",
            section_index: Some(3),
        },
        PageLinkPickerCategorySpec {
            label: "Food & Drink",
            section_index: Some(4),
        },
        PageLinkPickerCategorySpec {
            label: "Activities",
            section_index: Some(5),
        },
        PageLinkPickerCategorySpec {
            label: "Travel & Places",
            section_index: Some(6),
        },
        PageLinkPickerCategorySpec {
            label: "Objects",
            section_index: Some(7),
        },
        PageLinkPickerCategorySpec {
            label: "Symbols",
            section_index: Some(8),
        },
        PageLinkPickerCategorySpec {
            label: "Flags",
            section_index: Some(9),
        },
        PageLinkPickerCategorySpec {
            label: "custom",
            section_index: Some(10),
        },
        PageLinkPickerCategorySpec {
            label: "add",
            section_index: None,
        },
    ];
    SPECS
}

pub(super) fn skin_tones() -> [emojis::SkinTone; 6] {
    [
        emojis::SkinTone::Default,
        emojis::SkinTone::Light,
        emojis::SkinTone::MediumLight,
        emojis::SkinTone::Medium,
        emojis::SkinTone::MediumDark,
        emojis::SkinTone::Dark,
    ]
}

pub(super) fn skin_tone_glyph(skin_tone: emojis::SkinTone) -> String {
    emojis::get("✋")
        .and_then(|emoji| emoji.with_skin_tone(skin_tone))
        .map_or_else(|| "✋".to_string(), |emoji| emoji.as_str().to_string())
}

pub(super) fn skin_tone_label(skin_tone: emojis::SkinTone) -> &'static str {
    match skin_tone {
        emojis::SkinTone::Default => "No skin tone",
        emojis::SkinTone::Light => "Light skin tone",
        emojis::SkinTone::MediumLight => "Medium-light skin tone",
        emojis::SkinTone::Medium => "Medium skin tone",
        emojis::SkinTone::MediumDark => "Medium-dark skin tone",
        emojis::SkinTone::Dark => "Dark skin tone",
        _ => "Skin tone",
    }
}

fn matching_emojis(query: &str) -> Vec<&'static emojis::Emoji> {
    let query = query.trim().to_lowercase();
    emoji_catalog()
        .iter()
        .filter(|entry| query.is_empty() || entry.search_text.contains(&query))
        .map(|entry| entry.emoji)
        .take(if query.is_empty() { 120 } else { 96 })
        .collect()
}

fn emoji_catalog() -> &'static [PageLinkPickerEmojiEntry] {
    static CATALOG: OnceLock<Vec<PageLinkPickerEmojiEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            emojis::iter()
                .map(|emoji| PageLinkPickerEmojiEntry {
                    emoji,
                    search_text: std::iter::once(emoji.name())
                        .chain(emoji.shortcodes())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase(),
                })
                .collect()
        })
        .as_slice()
}

fn emoji_section_row_count(section: &PageLinkPickerEmojiSection) -> usize {
    section
        .emojis
        .len()
        .div_ceil(PAGE_LINK_EMOJI_COLUMN_COUNT)
        .max(1)
}

fn emoji_list_items() -> &'static [PageLinkPickerEmojiListItem] {
    static ITEMS: OnceLock<Vec<PageLinkPickerEmojiListItem>> = OnceLock::new();
    ITEMS
        .get_or_init(|| {
            let mut emoji_start_index = 0;
            emoji_sections()
                .iter()
                .take(PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX)
                .enumerate()
                .flat_map(|(section_index, section)| {
                    let section_start_index = emoji_start_index;
                    emoji_start_index += section.emojis.len();
                    std::iter::once(PageLinkPickerEmojiListItem::SectionHeader { section_index })
                        .chain((0..emoji_section_row_count(section)).map(move |row_index| {
                            PageLinkPickerEmojiListItem::EmojiRow {
                                section_index,
                                row_index,
                                emoji_start_index: section_start_index
                                    + row_index * PAGE_LINK_EMOJI_COLUMN_COUNT,
                            }
                        }))
                })
                .collect()
        })
        .as_slice()
}

fn custom_emoji_row_count(custom_emoji_count: usize) -> usize {
    custom_emoji_count
        .div_ceil(PAGE_LINK_EMOJI_COLUMN_COUNT)
        .max(1)
}
