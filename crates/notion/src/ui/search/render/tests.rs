use super::{
    preview_style::{
        notion_search_preview_annotation_highlights, notion_search_preview_block_role,
        notion_search_preview_body_fill, notion_search_preview_code_ranges,
        notion_search_preview_has_explicit_icon, notion_search_preview_header_fill,
        notion_search_preview_text_metrics, NotionSearchPreviewBlockRole,
    },
    SEARCH_BODY_HEIGHT, SEARCH_DARK_BORDER_FILL, SEARCH_FILTERS_HEIGHT, SEARCH_FOOTER_HEIGHT,
    SEARCH_HEADER_HEIGHT, SEARCH_LOADED_HEADER_LEFT_INSET, SEARCH_LOADING_BODY_HEIGHT,
    SEARCH_LOADING_FILTERS_HEIGHT, SEARCH_LOADING_HEADER_HEIGHT, SEARCH_MODAL_CONTENT_WIDTH,
    SEARCH_MODAL_OUTER_HEIGHT, SEARCH_PREVIEW_WIDTH, SEARCH_RESULTS_WIDTH, SEARCH_SEARCH_ICON_SIZE,
};
use crate::model::{
    CardPage, CardPageBlock, CardPageBlockKind, CardPageEditableBlock, CardPageTextAnnotationSpan,
    PageShellIcon, PageTextAnnotation,
};
use crate::ui::{AppearanceMode, FontWeight, LoadedCardPage};

#[test]
fn preview_block_roles_preserve_reference_document_shapes() {
    assert_eq!(
        notion_search_preview_block_role(CardPageBlockKind::Text),
        NotionSearchPreviewBlockRole::Paragraph
    );
    assert_eq!(
        notion_search_preview_block_role(CardPageBlockKind::SubHeader),
        NotionSearchPreviewBlockRole::HeadingLarge
    );
    assert_eq!(
        notion_search_preview_block_role(CardPageBlockKind::Heading3),
        NotionSearchPreviewBlockRole::HeadingSmall
    );
    assert_eq!(
        notion_search_preview_block_role(CardPageBlockKind::BulletedList),
        NotionSearchPreviewBlockRole::Bulleted
    );
    assert_eq!(
        notion_search_preview_text_metrics(NotionSearchPreviewBlockRole::HeadingSmall),
        (13.0, 18.0, FontWeight::SEMIBOLD)
    );
    assert_eq!(
        NotionSearchPreviewBlockRole::HeadingLarge.top_margin(),
        22.0
    );
    assert_eq!(
        NotionSearchPreviewBlockRole::HeadingLarge.bottom_margin(),
        12.0
    );
}

#[test]
fn preview_numbering_uses_canonical_per_parent_sequences() {
    let page_id = "numbered-preview";
    let page = LoadedCardPage::new(CardPage {
        block_id: page_id.to_string(),
        title: "Numbered preview".to_string(),
        status: None,
        properties: Vec::new(),
        blocks: vec![
            CardPageBlock::editable(
                "root-a",
                page_id,
                0,
                CardPageBlockKind::NumberedList,
                "Root A",
            ),
            CardPageBlock::editable(
                "nested-a",
                "root-a",
                1,
                CardPageBlockKind::NumberedList,
                "Nested A",
            ),
            CardPageBlock::editable(
                "root-b",
                page_id,
                0,
                CardPageBlockKind::NumberedList,
                "Root B",
            ),
        ],
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    });

    let rendered_numbering = page
        .data
        .visible_rows
        .iter()
        .map(|row| {
            (
                page.data.page.blocks[row.block_index].block_id.as_str(),
                row.visual_depth,
                page.data.numbered_indices[row.block_index],
                row.joins_previous_list_sibling,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered_numbering,
        vec![
            ("root-a", 0, 1, false),
            ("nested-a", 1, 1, false),
            ("root-b", 0, 2, true),
        ]
    );
}

#[test]
fn preview_annotation_runs_keep_unicode_boundaries_and_inline_code() {
    let text = "Café metric";
    let editable = CardPageEditableBlock::new(
        CardPageBlockKind::Text,
        text,
        vec![
            CardPageTextAnnotationSpan::new(text, 0, text.len(), PageTextAnnotation::Bold)
                .expect("valid bold annotation"),
            CardPageTextAnnotationSpan::new(text, 6, 12, PageTextAnnotation::Code)
                .expect("valid code annotation"),
        ],
    );

    let ranges =
        notion_search_preview_annotation_highlights(&editable.annotations, AppearanceMode::Dark)
            .into_iter()
            .map(|(range, _)| range)
            .collect::<Vec<_>>();
    assert_eq!(ranges, vec![0..6, 6..12]);
    assert_eq!(notion_search_preview_code_ranges(&editable), vec![6..12]);
}

#[test]
fn preview_default_page_icon_keeps_an_empty_slot() {
    assert!(!notion_search_preview_has_explicit_icon(
        &PageShellIcon::named("page")
    ));
    assert!(notion_search_preview_has_explicit_icon(
        &PageShellIcon::emoji("🧠")
    ));
    assert!(notion_search_preview_has_explicit_icon(
        &PageShellIcon::named("microphone")
    ));
}

#[test]
fn preview_dark_fills_match_the_reference_pixels() {
    assert_eq!(
        notion_search_preview_header_fill(AppearanceMode::Dark),
        0x202020
    );
    assert_eq!(
        notion_search_preview_body_fill(AppearanceMode::Dark),
        0x191919
    );
}

#[test]
fn loading_shell_geometry_matches_the_explicit_reference_state() {
    assert_eq!(
        SEARCH_LOADING_HEADER_HEIGHT
            + SEARCH_LOADING_FILTERS_HEIGHT
            + SEARCH_LOADING_BODY_HEIGHT
            + SEARCH_FOOTER_HEIGHT,
        SEARCH_MODAL_OUTER_HEIGHT - 2.0
    );
    assert_eq!(
        SEARCH_HEADER_HEIGHT + SEARCH_FILTERS_HEIGHT + SEARCH_BODY_HEIGHT + SEARCH_FOOTER_HEIGHT,
        SEARCH_MODAL_OUTER_HEIGHT - 2.0
    );
    assert_eq!(
        SEARCH_RESULTS_WIDTH + SEARCH_PREVIEW_WIDTH,
        SEARCH_MODAL_CONTENT_WIDTH
    );
}

#[test]
fn loaded_search_artwork_and_dark_border_match_the_live_reference() {
    assert_eq!(SEARCH_LOADED_HEADER_LEFT_INSET, 17.0);
    assert_eq!(SEARCH_SEARCH_ICON_SIZE, 20.0);
    assert_eq!(SEARCH_DARK_BORDER_FILL, 0x353533);
}
