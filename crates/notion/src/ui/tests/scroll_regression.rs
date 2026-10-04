use crate::ui::regression_test_support::{
    drive_selected_page_overlay_wheel_scroll, drive_standalone_document_wheel_scroll,
    NotionPageScrollOutcome, NotionPageScrollSurface,
};

#[gpui::test]
fn long_standalone_document_scrolls_from_a_wheel_event(cx: &mut gpui::TestAppContext) {
    let outcome = drive_standalone_document_wheel_scroll(cx);

    assert_scroll_outcome(
        &outcome,
        NotionPageScrollSurface::StandaloneDocument,
        "notion-page-scroll",
    );
}

#[gpui::test]
fn long_selected_page_overlay_scrolls_from_a_wheel_event(cx: &mut gpui::TestAppContext) {
    let outcome = drive_selected_page_overlay_wheel_scroll(cx);

    assert_scroll_outcome(
        &outcome,
        NotionPageScrollSurface::SelectedPageOverlay,
        "notion-selected-page-scroll",
    );
}

fn assert_scroll_outcome(
    outcome: &NotionPageScrollOutcome,
    expected_surface: NotionPageScrollSurface,
    expected_host_id: &str,
) {
    assert_eq!(outcome.surface, expected_surface);
    assert_eq!(outcome.scroll_host_id, expected_host_id);
    assert!(outcome.document_block_count >= 100);
    assert!(outcome.virtualized_item_count > 2);
    assert!(outcome.scroll_host_width > 0.0);
    assert!(outcome.scroll_host_height > 0.0);
    assert!(outcome.list_viewport_height > 0.0);
    assert_ne!(
        outcome.final_offset, outcome.initial_offset,
        "wheel event did not move the Notion document"
    );
    assert!(
        outcome.final_offset.item_index > outcome.initial_offset.item_index
            || (outcome.final_offset.item_index == outcome.initial_offset.item_index
                && outcome.final_offset.offset_in_item > outcome.initial_offset.offset_in_item),
        "wheel event did not move the Notion document downward"
    );
    assert!(outcome.wheel_delta_y < 0.0);
}
