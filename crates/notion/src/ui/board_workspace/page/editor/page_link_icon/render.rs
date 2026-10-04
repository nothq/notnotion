mod catalog;
mod emoji_cells;
mod emoji_grid;
mod inputs;
mod menus;
mod named;
mod panel;
mod search;
mod tabs;
mod upload;

pub(super) use inputs::new_page_link_icon_picker_ui;

const PAGE_LINK_EMOJI_COLUMN_COUNT: usize = 12;
const PAGE_LINK_EMOJI_ROW_HEIGHT: f32 = 32.0;
const PAGE_LINK_EMOJI_SECTION_HEADER_HEIGHT: f32 = 26.0;
const PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX: usize = 10;
const PAGE_LINK_NAMED_ICON_COLUMN_COUNT: usize = 12;
pub(super) const PAGE_LINK_NAMED_ICON_CELL_SIZE: f32 = 32.0;
pub(super) const PAGE_LINK_NAMED_ICON_GRID_TOP: f32 = 90.0;
const PAGE_LINK_NAMED_ICON_ROW_HEIGHT: f32 = 32.0;
const PAGE_LINK_NAMED_ICON_SECTION_HEADER_HEIGHT: f32 = 34.0;

pub(super) fn page_link_picker_named_icon_item_count(
    match_count: usize,
    recent_count: usize,
    query_is_empty: bool,
) -> usize {
    catalog::named_icon_item_count(match_count, recent_count, query_is_empty)
}

pub(super) fn page_link_picker_emoji_item_count(
    query: &str,
    custom_emojis: &[crate::model::NotionCustomEmoji],
) -> usize {
    catalog::emoji_item_count(query, custom_emojis)
}

pub(super) fn page_link_picker_emoji_section_item_index(section_index: usize) -> Option<usize> {
    catalog::emoji_section_item_index(section_index)
}

pub(super) fn page_link_picker_category_index_for_item(item_index: usize) -> Option<usize> {
    catalog::category_index_for_item(item_index)
}
