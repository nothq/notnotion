use super::{
    close_spec_window, drive_mention_escape, drive_today_mention_commit, spec_editable,
    spec_snapshot, spec_surface, spec_window_size, type_text, PageMentionClock, MENTION_BLOCK_ID,
};
use crate::model::{PageMention, PageTextAnnotation, PAGE_MENTION_TOKEN_STR};
use crate::ui::{board_workspace::PageMentionMenuRow, SurfaceState};
use gpui::TestAppContext;

/// Open the spec page and hand the surface to `exercise`.
fn with_mention_surface<R>(
    cx: &mut TestAppContext,
    exercise: impl FnOnce(&mut SurfaceState, &mut gpui::Context<SurfaceState>) -> R,
) -> R {
    let window = cx.open_window(spec_window_size(), |_, _| spec_surface());
    cx.run_until_parked();
    let root = window.root(cx).expect("access the mention test root");
    let result = root.update(cx, exercise);
    close_spec_window(window, cx);
    result
}

#[gpui::test]
fn typing_at_opens_the_menu_anchored_at_the_trigger(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "Ship it @", cx);
        let menu = surface
            .page_editor
            .mention
            .menu()
            .cloned()
            .expect("@ opens the mention menu");
        assert_eq!(menu.block_id, MENTION_BLOCK_ID);
        assert_eq!(menu.trigger_offset, "Ship it ".len());
        assert_eq!(menu.query, "");
        assert_eq!(menu.selected_index, 0);
        let rows = surface
            .page_editor
            .mention
            .menu_rows(&PageMentionClock::from(&surface.board));
        assert_eq!(rows[0].label(), "Today");
        assert!(matches!(rows[1], PageMentionMenuRow::Reminder { .. }));
    });
}

#[gpui::test]
fn an_at_inside_a_word_does_not_open_the_menu(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "mail@", cx);
        assert!(surface.page_editor.mention.menu().is_none());
    });
}

#[gpui::test]
fn arrow_keys_move_the_selection_and_wrap(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@sep 7", cx);
        let row_count = surface
            .page_editor
            .mention
            .menu_rows(&PageMentionClock::from(&surface.board))
            .len();
        if surface.page_editor.mention.move_menu_selection(
            MENTION_BLOCK_ID,
            1,
            &PageMentionClock::from(&surface.board),
        ) {
            cx.notify();
        }
        assert_eq!(
            surface
                .page_editor
                .mention
                .menu()
                .expect("the menu stays open")
                .selected_index,
            1
        );
        if surface.page_editor.mention.move_menu_selection(
            MENTION_BLOCK_ID,
            -2,
            &PageMentionClock::from(&surface.board),
        ) {
            cx.notify();
        }
        assert_eq!(
            surface
                .page_editor
                .mention
                .menu()
                .expect("the menu stays open")
                .selected_index,
            row_count - 1
        );
    });
}

#[gpui::test]
fn a_partial_query_ghosts_the_rest_of_the_selected_label(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@tod", cx);
        let ghost = surface
            .page_editor
            .mention
            .input_ghost(
                MENTION_BLOCK_ID,
                &PageMentionClock::from(&surface.board),
                surface.appearance_mode,
            )
            .expect("a partial date query ghosts its completion");
        assert_eq!(ghost.text.as_ref(), "ay");
        assert_eq!(ghost.offset, "@tod".len());
        assert_eq!(
            surface.page_editor.mention.pill_range(MENTION_BLOCK_ID),
            Some(0.."@tod".len())
        );
    });
}

#[gpui::test]
fn a_bare_trigger_ghosts_the_whole_label(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@", cx);
        let ghost = surface
            .page_editor
            .mention
            .input_ghost(
                MENTION_BLOCK_ID,
                &PageMentionClock::from(&surface.board),
                surface.appearance_mode,
            )
            .expect("a bare trigger ghosts the top label");
        assert_eq!(ghost.text.as_ref(), "Today");
        assert_eq!(ghost.offset, 1);
    });
}

#[gpui::test]
fn a_query_the_label_does_not_continue_has_no_ghost(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@sep 7", cx);
        // Saturday 5 September 2026, when September 7 was next Monday.
        let clock = PageMentionClock {
            today: chrono::NaiveDate::from_ymd_opt(2026, 9, 5).expect("valid fixture date"),
            ..PageMentionClock::from(&surface.board)
        };
        assert_eq!(
            surface.page_editor.mention.menu_rows(&clock)[0].label(),
            "Next Monday",
            "the fixture query resolves to a label that does not start with it"
        );
        assert!(surface
            .page_editor
            .mention
            .input_ghost(MENTION_BLOCK_ID, &clock, surface.appearance_mode)
            .is_none());
        assert_eq!(
            surface.page_editor.mention.pill_range(MENTION_BLOCK_ID),
            Some(0.."@sep 7".len()),
            "the pill still marks the trigger while it is being typed"
        );
    });
}

#[gpui::test]
fn closing_the_menu_clears_the_ghost_and_the_pill(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@tod", cx);
        if surface.page_editor.mention.close_menu(MENTION_BLOCK_ID) {
            cx.notify();
        }
        assert!(surface
            .page_editor
            .mention
            .input_ghost(
                MENTION_BLOCK_ID,
                &PageMentionClock::from(&surface.board),
                surface.appearance_mode
            )
            .is_none());
        assert!(surface
            .page_editor
            .mention
            .pill_range(MENTION_BLOCK_ID)
            .is_none());
    });
}

#[gpui::test]
fn a_query_that_is_not_a_date_drops_the_date_section(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@zzz", cx);
        let rows = surface
            .page_editor
            .mention
            .menu_rows(&PageMentionClock::from(&surface.board));
        assert!(rows
            .iter()
            .all(|row| !matches!(row, PageMentionMenuRow::Date { .. })));
        assert!(rows
            .iter()
            .any(|row| matches!(row, PageMentionMenuRow::NewPage { title } if title == "zzz")));
    });
}

#[gpui::test]
fn committing_a_date_leaves_one_token_and_a_trailing_space(cx: &mut TestAppContext) {
    let outcome = drive_today_mention_commit(cx);
    assert_eq!(outcome.committed_text, format!("{PAGE_MENTION_TOKEN_STR} "));
    assert_eq!(outcome.committed_start_date, Some(outcome.today));
    assert!(outcome.menu_closed_after_commit);
}

#[gpui::test]
fn the_committed_token_carries_a_date_mention_span(cx: &mut TestAppContext) {
    with_mention_surface(cx, |surface, cx| {
        type_text(surface, "@today", cx);
        surface.submit_page_block(MENTION_BLOCK_ID, spec_snapshot("@today"), cx);
        let editable = spec_editable(surface);
        let span = editable
            .annotations
            .iter()
            .find(|span| span.start_utf8 == 0)
            .expect("the token carries a mention span");
        assert_eq!(span.end_utf8, PAGE_MENTION_TOKEN_STR.len());
        assert!(matches!(
            span.annotation,
            PageTextAnnotation::Mention(PageMention::Date(_))
        ));
    });
}

#[gpui::test]
fn escape_keeps_the_typed_text_and_suppresses_the_same_trigger(cx: &mut TestAppContext) {
    let outcome = drive_mention_escape(cx);
    assert_eq!(outcome.text_after_escape, "@tod");
    assert!(!outcome.menu_open_after_escape);
    assert!(!outcome.menu_reopened_while_typing);
}

/// The scroll specs share one app with the mention specs, so a scenario
/// must not leave a window or a pending search behind.
#[gpui::test]
fn a_mention_scenario_leaves_the_app_usable_for_the_next_one(cx: &mut TestAppContext) {
    drive_today_mention_commit(cx);
    super::super::drive_standalone_document_wheel_scroll(cx);
}
