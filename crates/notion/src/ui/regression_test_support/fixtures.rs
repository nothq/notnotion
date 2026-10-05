use app_model::Viewport;

use crate::model::{
    BoardSnapshot, CardPage, CardPageBlock, CardPageBlockKind, CardPageLayoutBlock,
};

use super::{REGRESSION_PAGE_BLOCK_COUNT, REGRESSION_WINDOW_HEIGHT, REGRESSION_WINDOW_WIDTH};

pub(crate) fn regression_viewport() -> Viewport {
    Viewport {
        logical_width: REGRESSION_WINDOW_WIDTH,
        logical_height: REGRESSION_WINDOW_HEIGHT,
        scale_factor: 1.0,
        ..Viewport::default()
    }
}

pub(crate) fn regression_page() -> CardPage {
    let mut blocks = vec![
        CardPageBlock::layout(
            "notion-scroll-regression-columns",
            "notion-scroll-regression-page",
            0,
            CardPageLayoutBlock::ColumnList,
        ),
        CardPageBlock::layout(
            "notion-scroll-regression-left-column",
            "notion-scroll-regression-columns",
            1,
            CardPageLayoutBlock::Column { ratio: None },
        ),
    ];
    blocks.extend((0..REGRESSION_PAGE_BLOCK_COUNT / 2).map(|index| {
        CardPageBlock::editable(
            format!("notion-scroll-regression-left-block-{index:03}"),
            "notion-scroll-regression-left-column",
            2,
            CardPageBlockKind::Text,
            format!(
                "Left regression paragraph {index:03}: deterministic nested Notion document content."
            ),
        )
    }));
    blocks.push(CardPageBlock::layout(
        "notion-scroll-regression-right-column",
        "notion-scroll-regression-columns",
        1,
        CardPageLayoutBlock::Column { ratio: None },
    ));
    blocks.extend((0..REGRESSION_PAGE_BLOCK_COUNT / 2).map(|index| {
        CardPageBlock::editable(
            format!("notion-scroll-regression-right-block-{index:03}"),
            "notion-scroll-regression-right-column",
            2,
            CardPageBlockKind::Text,
            format!(
                "Right regression paragraph {index:03}: deterministic nested Notion document content."
            ),
        )
    }));
    CardPage {
        block_id: "notion-scroll-regression-page".to_string(),
        title: "Long regression document".to_string(),
        status: None,
        properties: Vec::new(),
        blocks,
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    }
}

pub(crate) fn regression_board() -> BoardSnapshot {
    BoardSnapshot {
        share_target_id: None,
        page_title: "Scroll regression".to_string(),
        database_title: "Scroll regression".to_string(),
        edited_label: String::new(),
        user_time_zone: "America/Toronto".to_string(),
        user_utc_offset_seconds: -14_400,
        is_private: false,
        is_locked: false,
        is_favorited: false,
        presence: None,
        columns: Vec::new(),
        view_tabs: Vec::new(),
        items: Vec::new(),
        table_view_columns: Vec::new(),
        active_view_property_layout: Default::default(),
        active_view_sorts: Default::default(),
        active_view_group: Default::default(),
        database_properties: Vec::new(),
        timeline_view: None,
        calendar_view: None,
        date_undated_count: None,
        page_content: None,
        page_shell: None,
    }
}
