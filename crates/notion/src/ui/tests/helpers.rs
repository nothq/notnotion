use crate::model::{BoardColumn, BoardItemProperty};
use crate::ui::tests::*;

mod fixtures;

pub(crate) use fixtures::*;

pub(crate) fn test_notion_card_url(board_url: &str, block_id: &str) -> String {
    let cleaned_id = block_id.replace('-', "");
    format!("{board_url}&p={cleaned_id}&pm=s")
}

#[gpui::test]
fn app_size_matches_reference_capture() {
    let viewport = Viewport::default();
    assert_eq!(
        app_size(),
        size(px(viewport.app_width()), px(viewport.app_height()))
    );
}

pub(crate) fn interaction_test_board() -> BoardSnapshot {
    BoardSnapshot {
        share_target_id: None,
        page_title: "Acme Playground".to_string(),
        database_title: "Acme Playground".to_string(),
        edited_label: "Edited just now".to_string(),
        user_time_zone: "America/Toronto".to_string(),
        user_utc_offset_seconds: -14_400,
        is_private: false,
        is_locked: false,
        is_favorited: false,
        presence: None,
        columns: vec![BoardColumn {
            title: "Todo".to_string(),
            option_color: None,
            cards: vec![CardSummary {
                block_id: "card-1".to_string(),
                title: "Card One".to_string(),
                height: 72.0,
                has_content: true,
                icon: None,
            }],
        }],
        view_tabs: interaction_test_view_tabs(),
        items: Vec::new(),
        table_view_columns: Vec::new(),
        active_view_property_layout: Default::default(),
        active_view_sorts: Default::default(),
        active_view_group: Default::default(),
        database_properties: Vec::new(),
        timeline_view: Some(TimelineViewConfig {
            date_property_id: "dates".to_string(),
            relation_property_id: None,
            zoom_level: Some("month".to_string()),
            center_timestamp_ms: Some(1_709_596_800_000),
            today_marker_timestamp_ms: Some(1_709_596_800_000),
        }),
        calendar_view: None,
        date_undated_count: None,
        page_content: None,
        page_shell: None,
    }
}

fn interaction_test_view_tabs() -> Vec<ViewTab> {
    vec![
        ViewTab {
            provider_view_id: "test-board-view"
                .parse()
                .expect("test board view ID must be valid"),
            label: "Board".to_string(),
            kind: ViewTabKind::Board,
            active: true,
            filters: None,
        },
        ViewTab {
            provider_view_id: "test-timeline-view"
                .parse()
                .expect("test timeline view ID must be valid"),
            label: "Timeline".to_string(),
            kind: ViewTabKind::Timeline,
            active: false,
            filters: None,
        },
    ]
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
use crate::model::CardSummary;
