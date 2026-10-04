use std::{collections::HashSet, sync::Arc};

use gpui::App;

use super::super::{PageLinkIconPickerState, PageLinkIconPickerTab};
use super::PageLinkIconPickerUi;
use crate::model::NotionCustomEmojiLibrary;
use crate::ui::notion_named_icon_matches;

pub(super) fn new_page_link_icon_picker_state(
    instance_id: u64,
    page_id: String,
    block_id: String,
    named_icon_preference: super::super::PageLinkNamedIconPreference,
    ui: PageLinkIconPickerUi,
) -> PageLinkIconPickerState {
    PageLinkIconPickerState {
        instance_id,
        page_id,
        block_id,
        tab: PageLinkIconPickerTab::Emoji,
        query: String::new(),
        search_input: ui.search_input,
        emoji_list_state: ui.emoji_list_state,
        named_icon_list_state: ui.named_icon_list_state,
        named_icon_matches: ui.named_icon_matches,
        recent_named_icons: ui.recent_named_icons,
        category_index: 0,
        skin_tone: emojis::SkinTone::Default,
        named_icon_preference,
        control_menu: None,
        upload_preview: None,
        upload_generation: 0,
        upload_pending: false,
        upload_committed: false,
        upload_add_to_library: false,
        upload_name: String::new(),
        upload_name_input: ui.upload_name_input,
        custom_emojis: Arc::from([]),
        custom_emoji_names: Arc::new(HashSet::new()),
        custom_emoji_library_generation: 0,
        custom_emoji_library_pending: false,
        custom_emoji_creation_allowed: false,
        custom_emoji_total_count: 0,
        custom_emoji_limit: None,
    }
}

pub(super) fn apply_custom_emoji_library(
    picker: &mut PageLinkIconPickerState,
    library: NotionCustomEmojiLibrary,
) {
    picker.custom_emoji_names = Arc::new(
        library
            .emojis
            .iter()
            .map(|emoji| emoji.name.clone())
            .collect(),
    );
    picker.custom_emojis = library.emojis.into();
    picker.custom_emoji_total_count = library.total_count;
    picker.custom_emoji_creation_allowed = library.creation_allowed;
    picker.custom_emoji_limit = library.limit;
    picker
        .emoji_list_state
        .reset(super::page_link_picker_emoji_item_count(
            &picker.query,
            &picker.custom_emojis,
        ));
}

pub(super) fn reset_page_link_icon_picker(picker: &mut PageLinkIconPickerState, cx: &mut App) {
    picker.control_menu = None;
    picker.category_index = 0;
    picker
        .emoji_list_state
        .reset(super::page_link_picker_emoji_item_count(
            "",
            &picker.custom_emojis,
        ));
    picker.named_icon_matches = notion_named_icon_matches("");
    picker
        .named_icon_list_state
        .reset(super::page_link_picker_named_icon_item_count(
            picker.named_icon_matches.len(),
            picker.recent_named_icons.len(),
            true,
        ));
    picker.query.clear();
    picker.upload_preview = None;
    picker.upload_generation = picker.upload_generation.wrapping_add(1);
    picker.upload_pending = false;
    picker.upload_committed = false;
    picker.upload_add_to_library = false;
    picker.upload_name.clear();
    picker.search_input.update(cx, |input, cx| {
        input.set_text(String::new(), cx);
    });
    picker.upload_name_input.update(cx, |input, cx| {
        input.set_text(String::new(), cx);
    });
}
