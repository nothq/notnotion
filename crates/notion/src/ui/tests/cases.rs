use crate::model::BoardItemProperty;
use crate::ui::tests::*;

mod interactions;
mod notion;
mod quick_find;
mod support;

pub(crate) use support::*;

#[gpui::test]
fn card_urls_preserve_board_context() {
    assert_eq!(
        test_notion_card_url(TEST_BOARD_URL, "c3d4e5f6-a7b8-493a-a415-c6d7e8f90a12"),
        "https://www.notion.so/acme/a1b2c3d4e5f647188293a4b5c6d7e8f0?v=b2c3d4e5f6a748299304b5c6d7e8f901&p=c3d4e5f6a7b8493aa415c6d7e8f90a12&pm=s"
    );
}

#[gpui::test]
fn app_size_matches_reference_capture() {
    let viewport = Viewport::default();
    assert_eq!(
        app_size(),
        size(px(viewport.app_width()), px(viewport.app_height()))
    );
}
#[gpui::test]
fn timeline_visible_start_uses_month_center_timestamp_without_items() {
    let board = interaction_test_board();
    let timeline_view = board
        .timeline_view
        .as_ref()
        .expect("interaction board should include timeline config");
    let today_marker_date = civil_date_for_timestamp_ms(
        timeline_view
            .today_marker_timestamp_ms
            .expect("timeline config should include today marker"),
    );

    let actual = timeline_visible_start(timeline_view, today_marker_date, false);
    let expected = add_days(
        civil_date_for_timestamp_ms(
            timeline_view
                .center_timestamp_ms
                .expect("timeline config should include center timestamp"),
        ),
        -TIMELINE_MONTH_CENTER_LEAD_DAYS,
    );

    assert_eq!(actual.year, expected.year);
    assert_eq!(actual.month, expected.month);
    assert_eq!(actual.day, expected.day);
}

#[gpui::test]
fn timeline_header_month_labels_cover_full_content_range() {
    let labels = timeline_header_month_labels(
        CivilDate {
            year: 2025,
            month: 11,
            day: 1,
        },
        CivilDate {
            year: 2026,
            month: 5,
            day: 1,
        },
        39.0,
    );

    assert_eq!(
        labels,
        vec![
            ("November 2025".to_string(), 0.0),
            (
                "December".to_string(),
                TIMELINE_DAY_ROW_LEFT_INSET + 30.0 * 39.0
            ),
            (
                "January 2026".to_string(),
                TIMELINE_DAY_ROW_LEFT_INSET + 61.0 * 39.0,
            ),
            (
                "February 2026".to_string(),
                TIMELINE_DAY_ROW_LEFT_INSET + 92.0 * 39.0,
            ),
            (
                "March 2026".to_string(),
                TIMELINE_DAY_ROW_LEFT_INSET + 120.0 * 39.0,
            ),
            (
                "April 2026".to_string(),
                TIMELINE_DAY_ROW_LEFT_INSET + 151.0 * 39.0,
            ),
        ]
    );
}

#[gpui::test]
fn timeline_initial_scroll_x_opens_centered_focus_window() {
    let mut board = interaction_test_board();
    board.items = vec![
        BoardItem {
            block_id: "card-1".to_string(),
            title: "Start".to_string(),
            status: Some("Todo".to_string()),
            icon: None,
            properties: vec![BoardItemProperty {
                property_id: "dates".to_string(),
                label: "Dates".to_string(),
                property_type: "date".to_string(),
                value: "2025-11-10".to_string(),
                date: Some(BoardDateValue {
                    start_date: "2025-11-10".to_string(),
                    end_date: None,
                    start_time: None,
                    end_time: None,
                }),
            }],
        },
        BoardItem {
            block_id: "card-2".to_string(),
            title: "End".to_string(),
            status: Some("Todo".to_string()),
            icon: None,
            properties: vec![BoardItemProperty {
                property_id: "dates".to_string(),
                label: "Dates".to_string(),
                property_type: "date".to_string(),
                value: "2026-07-08".to_string(),
                date: Some(BoardDateValue {
                    start_date: "2026-07-08".to_string(),
                    end_date: None,
                    start_time: None,
                    end_time: None,
                }),
            }],
        },
    ];
    board.timeline_view = Some(TimelineViewConfig {
        date_property_id: "dates".to_string(),
        relation_property_id: None,
        zoom_level: Some("month".to_string()),
        center_timestamp_ms: Some(1_773_225_600_000),
        today_marker_timestamp_ms: Some(1_773_225_600_000),
    });

    let scroll_x = timeline_initial_scroll_x(&board, Viewport::default());

    assert!(scroll_x > 0.0);
}
