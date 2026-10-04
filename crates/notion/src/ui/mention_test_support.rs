//! Drivers for the `@` mention menu used by the Notion cucumber specs.

use crate::ui::board_workspace::PageEditSession;

use crate::ui::board_workspace::PageMentionClock;
use chrono::NaiveDate;
use gpui::{px, size, Pixels, TestAppContext, WindowHandle};
use gpui_components::text_input::TextInputSnapshot;

use crate::model::{
    CardPage, CardPageBlock, CardPageBlockKind, CardPageEditableBlock, PageMention,
    PageTextAnnotation, PAGE_MENTION_TOKEN_STR,
};

use super::regression_test_support::{
    regression_board, regression_surface_with, regression_viewport,
};
use super::{CardPeekState, NotionPageScrollSurface, SurfaceState};

const MENTION_PAGE_ID: &str = "notion-mention-spec-page";
const MENTION_BLOCK_ID: &str = "notion-mention-spec-block";

/// What a scripted `@today` interaction produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionDateMentionOutcome {
    /// Rows the menu offered right after `@`, top to bottom.
    pub initial_row_labels: Vec<String>,
    /// The first row's label once `today` was typed.
    pub filtered_first_row: String,
    /// The block text after Enter.
    pub committed_text: String,
    /// The start date stored on the inserted mention.
    pub committed_start_date: Option<NaiveDate>,
    /// Today as the surface's own clock resolves it, which is what `@today`
    /// must land on regardless of the host's time zone.
    pub today: NaiveDate,
    /// Whether the menu closed after the commit.
    pub menu_closed_after_commit: bool,
}

/// What Escape leaves behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionMentionEscapeOutcome {
    pub text_after_escape: String,
    pub menu_open_after_escape: bool,
    pub menu_reopened_while_typing: bool,
}

pub fn drive_today_mention_commit(cx: &mut TestAppContext) -> NotionDateMentionOutcome {
    let window = cx.open_window(spec_window_size(), |_, _| spec_surface());
    cx.run_until_parked();
    let root = window.root(cx).expect("access Notion mention spec root");
    let outcome = root.update(cx, |surface, cx| {
        type_text(surface, "@", cx);
        let initial_row_labels = surface
            .page_editor
            .mention
            .menu_rows(&PageMentionClock::from(&surface.board))
            .iter()
            .map(|row| row.label())
            .collect::<Vec<_>>();
        type_text(surface, "@today", cx);
        let filtered_first_row = surface
            .page_editor
            .mention
            .menu_rows(&PageMentionClock::from(&surface.board))
            .first()
            .map(|row| row.label())
            .unwrap_or_default();
        surface.submit_page_block(MENTION_BLOCK_ID, spec_snapshot("@today"), cx);
        let editable = spec_editable(surface);
        let committed_start_date =
            editable
                .annotations
                .iter()
                .find_map(|span| match &span.annotation {
                    PageTextAnnotation::Mention(PageMention::Date(date))
                        if span.start_utf8 == 0 =>
                    {
                        Some(date.start_date)
                    }
                    _ => None,
                });
        NotionDateMentionOutcome {
            initial_row_labels,
            filtered_first_row,
            committed_text: editable.text,
            committed_start_date,
            today: PageMentionClock::from(&surface.board).today,
            menu_closed_after_commit: surface.page_editor.mention.menu().is_none(),
        }
    });
    close_spec_window(window, cx);
    outcome
}

pub fn drive_mention_escape(cx: &mut TestAppContext) -> NotionMentionEscapeOutcome {
    let window = cx.open_window(spec_window_size(), |_, _| spec_surface());
    cx.run_until_parked();
    let root = window.root(cx).expect("access Notion mention spec root");
    let outcome = root.update(cx, |surface, cx| {
        type_text(surface, "@tod", cx);
        assert!(
            surface.page_editor.mention.menu_is_open(),
            "typing @tod opens the mention menu"
        );
        if surface.page_editor.mention.close_menu(MENTION_BLOCK_ID) {
            cx.notify();
        }
        let menu_open_after_escape = surface.page_editor.mention.menu_is_open();
        let text_after_escape = spec_editable(surface).text;
        type_text(surface, "@toda", cx);
        NotionMentionEscapeOutcome {
            text_after_escape,
            menu_open_after_escape,
            menu_reopened_while_typing: surface.page_editor.mention.menu_is_open(),
        }
    });
    close_spec_window(window, cx);
    outcome
}

/// Close the scenario's window so the mention menu's pending searches and the
/// page write queue cannot outlive it and run inside a later scenario.
fn close_spec_window(window: WindowHandle<SurfaceState>, cx: &mut TestAppContext) {
    window
        .update(cx, |_, window, _| window.remove_window())
        .expect("close the Notion mention spec window");
    cx.run_until_parked();
}

/// The token character Notion stores for a mention, for specs that assert the
/// text a commit leaves in the block.
pub fn mention_token_text() -> &'static str {
    PAGE_MENTION_TOKEN_STR
}

fn spec_snapshot(text: &str) -> TextInputSnapshot {
    TextInputSnapshot {
        text: text.to_string(),
        selection: text.len()..text.len(),
        cursor: text.len(),
        is_composing: false,
    }
}

fn spec_editable(surface: &SurfaceState) -> CardPageEditableBlock {
    let Some(CardPeekState::Loaded(page)) = surface.page_documents.selected_page.as_ref() else {
        panic!("the mention spec page stays loaded");
    };
    page.data
        .page
        .blocks
        .iter()
        .find(|block| block.block_id == MENTION_BLOCK_ID)
        .and_then(|block| block.editable_content().cloned())
        .expect("the mention spec block stays editable")
}

fn spec_surface() -> SurfaceState {
    regression_surface_with(
        NotionPageScrollSurface::SelectedPageOverlay,
        regression_board(),
        spec_page(),
        regression_viewport(),
    )
}

fn spec_page() -> CardPage {
    CardPage {
        block_id: MENTION_PAGE_ID.to_string(),
        title: "Mention spec".to_string(),
        status: None,
        properties: Vec::new(),
        blocks: vec![CardPageBlock::editable(
            MENTION_BLOCK_ID,
            MENTION_PAGE_ID,
            0,
            CardPageBlockKind::Text,
            "",
        )],
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    }
}

fn spec_window_size() -> gpui::Size<Pixels> {
    let viewport = regression_viewport();
    size(
        px(viewport.logical_width as f32),
        px(viewport.logical_height as f32),
    )
}

fn type_text(surface: &mut SurfaceState, text: &str, cx: &mut gpui::Context<SurfaceState>) {
    {
        let transition = {
            let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
            edit.apply_page_block_text_change(MENTION_BLOCK_ID, spec_snapshot(text));
            edit.finish(())
        };
        surface.apply_page_edit_transition(transition, cx)
    };
}

#[cfg(test)]
mod tests;
