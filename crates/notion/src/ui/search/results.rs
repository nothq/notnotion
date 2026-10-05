pub(super) use super::render::QuickFindView;

mod list;
mod query;
mod query_text;
mod row;

#[cfg(test)]
mod tests;

const NOTION_SEARCH_QUERY_ROW_HEIGHT: f32 = 56.0;
const NOTION_SEARCH_QUERY_SNIPPET_ROW_HEIGHT: f32 = 76.0;
const NOTION_SEARCH_QUERY_TWO_LINE_SNIPPET_ROW_HEIGHT: f32 = 93.0;
const NOTION_SEARCH_QUERY_HEADER_HEIGHT: f32 = 33.0;
const NOTION_SEARCH_RECENCY_HEADER_HEIGHT: f32 = 34.0;
// 600px results pane - 28px list gutters - 16px row padding - 20px icon - 8px gap.
const NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH: f32 = 528.0;
const NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH: f32 = 380.0;
const NOTION_SEARCH_QUERY_TITLE_BADGE_GAP: f32 = 6.0;
const NOTION_SEARCH_QUERY_BADGE_HORIZONTAL_PADDING: f32 = 10.0;
const NOTION_SEARCH_QUERY_MATCH_COLOR: u32 = 0x2783de;
const NOTION_SEARCH_PAGINATION_PREFETCH_ROWS: usize = 3;
const NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT: usize = 6;
const NOTION_SEARCH_LOADING_SKELETON_ROW_HEIGHT: f32 = 12.0;
const NOTION_SEARCH_LOADING_SKELETON_ROW_GAP: f32 = 14.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotionSearchQuerySnippetLayout {
    None,
    OneLine,
    TwoLines,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotionSearchResultIconArtwork {
    Database,
    DefaultPage,
    Explicit,
}
