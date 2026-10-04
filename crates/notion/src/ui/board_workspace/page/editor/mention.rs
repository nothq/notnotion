//! The `@` mention menu: Notion's inline picker for dates, people and pages.
//!
//! Geometry and behaviour follow the live Notion desktop capture: a 330 px
//! panel anchored 8 px left of the `@` glyph and 9.5 px under its line box,
//! 28 px rows, a query that may contain spaces, Escape leaving the typed text
//! in place, and Enter committing the selected row as one `‣` token.

use chrono::{FixedOffset, NaiveDate, NaiveTime, Utc};

mod actions;
mod atoms;
mod callbacks;
mod commit;
mod data;
mod ghost;
mod host;
mod picker;
mod render;
mod rows;
mod session;
mod state;
mod support;

#[cfg(test)]
mod parity;

pub(in crate::ui::board_workspace::page) use actions::PageMentionAction;
pub(super) use atoms::page_text_input_atoms;
pub(super) use callbacks::page_mention_action_sink;
pub(super) use ghost::page_mention_input_pill_highlights;
pub(in crate::ui::board_workspace::page::editor) use render::{
    PageMentionMenuPresentationSeed, PageMentionRenderer, PageMentionRendererResources,
};
pub(crate) use rows::{PageMentionMenuRow, PageMentionMenuSection};
pub(crate) use state::PageMentionController;
pub(in crate::ui::board_workspace::page) use support::mention_menu_token;

pub(crate) const PAGE_MENTION_MENU_ROW_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_MENTION_MENU_PAGE_ROW_HEIGHT: f32 = 45.0;
pub(crate) const PAGE_MENTION_MENU_ANCHOR_X_INSET: f32 = 8.0;
pub(crate) const PAGE_MENTION_MENU_ANCHOR_Y_GAP: f32 = 9.5;
pub(crate) const PAGE_MENTION_MENU_MAX_HEIGHT_RATIO: f32 = 0.4;
pub(crate) const PAGE_MENTION_MENU_VIEWPORT_MARGIN: f32 = 8.0;

/// The local date, time and zone the menu resolves relative dates against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageMentionClock {
    pub(crate) today: NaiveDate,
    pub(crate) now: NaiveTime,
    pub(crate) time_zone: String,
}

impl PageMentionClock {
    pub(crate) fn local(time_zone: &str, utc_offset_seconds: i32) -> Self {
        let utc_offset = FixedOffset::east_opt(utc_offset_seconds)
            .expect("board snapshot UTC offset must be resolved during live-data parsing");
        let now = Utc::now().with_timezone(&utc_offset).naive_local();
        Self {
            today: now.date(),
            now: now.time(),
            time_zone: time_zone.to_string(),
        }
    }
}

impl From<&crate::model::BoardSnapshot> for PageMentionClock {
    fn from(board: &crate::model::BoardSnapshot) -> Self {
        Self::local(&board.user_time_zone, board.user_utc_offset_seconds)
    }
}
