use crate::ui::CardPage;
use crate::ui::CardPageBlock;
use crate::ui::CardPageBlockColor;
use crate::ui::CardPageBlockColorValue;
use crate::ui::CardPageBlockContent;
use crate::ui::CardPageBlockKind;
use crate::ui::CardPageEditableBlock;
use crate::ui::CardPageProperty;
use crate::ui::CardPageStructuralBlock;
use crate::ui::CardPageToDoState;
use crate::ui::ViewTab;
use crate::ui::ViewTabKind;
use crate::ui::{
    add_days, alpha, civil_date_is_weekend, column_style, command_menu_close_row_body,
    command_menu_divider, command_menu_empty_state, command_menu_option_body,
    command_menu_panel_shell, command_menu_row_shell, command_menu_section_title, div,
    format_page_property_value, generated_notion_record_id, img, lightning_icon,
    notion_ai_button_image, notion_ai_face_image, page_block_background, page_block_foreground,
    page_block_visual_color, page_callout_background, page_callout_border,
    page_command_placeholder, page_composer_numbered_index, page_to_do_border_color, point, px,
    relative, render_page_callout_marker, render_page_command_bulleted_list_icon,
    render_page_command_callout_icon, render_page_command_glyph_icon,
    render_page_command_numbered_list_icon, render_page_command_todo_list_icon,
    render_page_command_toggle_list_icon, render_page_link_marker, render_page_toggle_marker,
    render_svg_image, render_top_bar_chip_impl, render_top_bar_locked_chip_impl,
    render_top_bar_menu_button_impl, render_top_bar_private_chip_impl, rgb, rgba, svg_from_body,
    timeline_bar, timeline_content_range, timeline_date_range_for_item, timeline_day_cell_width,
    timeline_grid_rule_color, timeline_grid_width, timeline_header_month_labels,
    timeline_marker_date, timeline_today_line_color, timeline_weekend_band_color, AnyElement,
    AppearanceMode, Arc, BoxShadow, Card, CardLocation, CardPeekState, CardPlacement, CivilDate,
    ClipboardItem, ColumnState, Context, Div, DragState, DropTarget, Element, FluentBuilder,
    FontWeight, HoveredCard, HoveredCardAction, IconAsset, InteractiveElement, IntoElement,
    KeyDownEvent, LoadedCardPage, LoadedCardPageData, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, MoveCardRequest, NotionAiMode, ObjectFit, PageCommand, PageCommandTarget,
    PageComposerState, PageSlashMenuState, ParentElement, Pixels, Point, PropertyPickerIconKind,
    ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement, Styled, StyledImage, SurfaceState,
    Theme, Tone, ToolbarDialogKind, TopBarPrivateChipOpacity, Window, ACTIVE_VIEW_HEIGHT,
    BOARD_SCROLL_LINE_MULTIPLIER, BOARD_SCROLL_PIXEL_MULTIPLIER, BOARD_VIEWPORT_RIGHT_GUTTER,
    BOARD_VIEWPORT_X, BOARD_VIEWPORT_Y, BOARD_WIDTH, CARD_GAP, CARD_PEEK_WIDTH, CARD_TOP,
    CARD_WIDTH, CARD_X_OFFSET, COLUMN_STRIDE, COLUMN_WIDTH, DRAG_AUTO_SCROLL_EDGE,
    DRAG_AUTO_SCROLL_STEP, DRAG_THRESHOLD, PAGE_COMMANDS, TIMELINE_CONTROLS_RIGHT_INSET,
    TIMELINE_CONTROLS_TOP_INSET, TIMELINE_DAY_LABEL_TOP_INSET, TIMELINE_DAY_MARKER_SIZE,
    TIMELINE_DAY_MARKER_TOP_INSET, TIMELINE_DAY_ROW_LEFT_INSET, TIMELINE_DAY_ROW_RIGHT_INSET,
    TIMELINE_EMPTY_BODY_HEIGHT, TIMELINE_HEADER_HEIGHT, TIMELINE_MONTH_LABEL_HEIGHT,
    TIMELINE_MONTH_LABEL_TOP_INSET, TIMELINE_ROW_HEIGHT, TIMELINE_TODAY_COLOR, VIEW_TAB_ICON_GAP,
    VIEW_TAB_ICON_SIZE,
};

mod board {

    pub(crate) mod actions;
    pub(crate) mod columns;
    pub(crate) mod drag;
    pub(crate) mod layout;
    pub(crate) mod toolbar;
}
pub(crate) use board::actions::PageDocumentAction;
pub(crate) use board::toolbar::InlineDatabaseToolbarTarget;
mod calendar_view;
mod card_chrome;
mod comments;
mod database_view_controls;
mod dialogs {

    pub(crate) mod ai_autofill;
    pub(crate) mod date_undated;
    pub(crate) mod filter;
    pub(crate) mod inline_database_views;
    pub(crate) mod inline_toolbar;
    pub(crate) mod primary;
    pub(crate) mod property_icons;
    pub(crate) mod secondary;
}
pub(crate) mod inline_database;
mod interaction_state;
mod list_gallery_view;
mod page {

    pub(crate) mod composer;
    pub(crate) mod editor;
    pub(crate) mod properties;

    pub(crate) mod commands {

        pub(crate) mod render;
        pub(crate) mod state;
    }
}
#[cfg(any(test, feature = "test-support"))]
pub(crate) use page::editor::PageMentionClock;
#[cfg(test)]
pub(crate) use page::editor::PageMentionMenuRow;
pub(crate) use page::editor::{
    PageBlockContextMenuState, PageBlockDragLayouts, PageBlockDragTarget, PageBlockFocusRequest,
    PageCodeSyntaxCache, PageDocumentColumn, PageDocumentLayout, PageEditSession, PageFocusSession,
    PageForcedTextAnnotations, PageLinkIconController, PageMentionController, PageMermaidDiagrams,
    PageMutationAction, PageMutationPlan, PagePendingCrossBlockComposition,
    PagePendingRichTextComposition, PagePendingRichTextTyping, PageProjectedText,
    PageRichTextDialog, PageTextProjectionTarget, PageWrite, PageWriteOperation,
    PageWriteTextProjection, SupportedPageBlockAction,
};
mod page_shell;
pub(crate) use page_shell::{
    activate_sidebar_tab, PageShellBodyRenderer, PageShellIconRenderer, SidebarView,
};
mod selected_page_overlay;
mod sharing;
pub(crate) use sharing::ShareEvent;
mod status_property_picker;
pub(crate) use status_property_picker::StatusPropertyPickerAction;
mod table_view;
mod timeline {
    use super::{
        add_days, alpha, civil_date_is_weekend, column_style, div, img, px, relative, rgb, rgba,
        timeline_bar, timeline_content_range, timeline_date_range_for_item,
        timeline_day_cell_width, timeline_grid_rule_color, timeline_grid_width,
        timeline_header_month_labels, timeline_marker_date, timeline_today_line_color,
        timeline_weekend_band_color, AnyElement, AppearanceMode, CivilDate, Context, Div,
        FontWeight, InteractiveElement, IntoElement, ParentElement, ScrollDelta, ScrollWheelEvent,
        StatefulInteractiveElement, Styled, SurfaceState, Window, ACTIVE_VIEW_HEIGHT,
        BOARD_SCROLL_LINE_MULTIPLIER, BOARD_SCROLL_PIXEL_MULTIPLIER, TIMELINE_CONTROLS_RIGHT_INSET,
        TIMELINE_CONTROLS_TOP_INSET, TIMELINE_DAY_LABEL_TOP_INSET, TIMELINE_DAY_MARKER_SIZE,
        TIMELINE_DAY_MARKER_TOP_INSET, TIMELINE_DAY_ROW_LEFT_INSET, TIMELINE_DAY_ROW_RIGHT_INSET,
        TIMELINE_EMPTY_BODY_HEIGHT, TIMELINE_HEADER_HEIGHT, TIMELINE_MONTH_LABEL_HEIGHT,
        TIMELINE_MONTH_LABEL_TOP_INSET, TIMELINE_ROW_HEIGHT, TIMELINE_TODAY_COLOR,
    };

    pub(crate) mod header;
    mod scroll;
    pub(crate) use header as timeline_header;
    pub(crate) mod view;
}
mod top_bar;

pub(in crate::ui) use page::editor::PageSimpleTableShortcut;
