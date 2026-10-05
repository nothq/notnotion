use gpui::{
    App, AppContext, Context, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputEnterBehavior, TextInputKeyAction, TextInputKeyPreAction,
    TextInputLayoutChange, TextInputMode, TextInputProps, TextInputSnapshot, TextInputStateChange,
    TextInputStyle, TextInputVerticalBoundaryAction,
};

use super::super::{
    alpha, command_menu_empty_state, command_menu_option_body, command_menu_panel_shell,
    command_menu_row_shell, command_menu_section_title, div, generated_notion_record_id, img,
    page_block_background, page_block_foreground, page_block_visual_color, page_callout_background,
    page_callout_border, page_command_placeholder, page_to_do_border_color, px,
    render_page_callout_marker, render_page_link_marker, render_page_toggle_marker, rgb, rgba,
    AnyElement, AppearanceMode, Arc, CardPage, CardPageBlock, CardPageBlockColor,
    CardPageBlockColorValue, CardPageBlockContent, CardPageBlockKind, CardPageEditableBlock,
    CardPageStructuralBlock, CardPageToDoState, CardPeekState, Div, FluentBuilder, LoadedCardPage,
    LoadedCardPageData, PageCommand, PageCommandTarget, PageSlashMenuState, SurfaceState, Theme,
    PAGE_COMMANDS,
};
use crate::ui::{
    CardPageQuoteSize, PageBlockDragScrollTarget, PageFlowNode,
    PAGE_FLOW_BLOCK_INDENT as PAGE_BLOCK_INDENT,
};

const PAGE_BLOCK_GUTTER_WIDTH: f32 = 52.0;
const PAGE_BLOCK_DROP_LINE_HEIGHT: f32 = 4.0;
const SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH: f32 = 120.0;

mod auto_scroll;
mod block_ops;
mod callbacks;
mod code;
mod column_resize;
mod context_menu;
mod disclosure;
mod document_edit;
mod drag;
pub(in crate::ui::board_workspace::page) mod editing;
mod focus;
mod history;
mod indent;
mod input;
mod mention;
mod navigation;
mod overlays;
mod page_link_icon;
mod page_state;
mod persistence;
mod projection;
mod render;
mod rich_text;
mod selection;
mod simple_table;
mod slash;
mod support;
mod text;
mod text_change;

pub(in crate::ui::board_workspace::page) use render::{
    render_page_block_editable_content, NumberedEditableBlock, PageBlockVisualStyle,
};
pub(crate) use render::{PageDocumentColumn, PageDocumentLayout};
pub(in crate::ui::board_workspace::page) use support::{
    page_block_default_row_spacing, page_block_input_spec, page_block_text_input_style,
    PageBlockInputSpec, PageBlockRowSpacing,
};

pub(crate) use rich_text::{
    PageForcedTextAnnotations, PagePendingRichTextComposition, PagePendingRichTextTyping,
    PageProjectedText, PageRichTextDialog, PageTextProjectionTarget, PageWrite, PageWriteOperation,
    PageWriteTextProjection,
};

pub(crate) use code::{PageCodeSyntaxCache, PageMermaidDiagrams};
use code::{PAGE_CODE_LANGUAGES, PAGE_CODE_LANGUAGE_OPTION_STRIDE};
use column_resize::PageColumnResizeDrag;
pub(crate) use context_menu::{
    PageBlockContextMenuPresentation, PageBlockContextMenuState, SupportedPageBlockAction,
};
use drag::{
    PageBlockDragLayout, PageBlockDragObservation, PageBlockDragPreviewRows,
    PageBlockDragSelection, PageBlockDragWash,
};
pub(crate) use drag::{
    PageBlockDragLayouts, PageBlockDragPayload, PageBlockDragPreviewRow, PageBlockDragTarget,
};
pub(crate) use editing::PageEditSession;
pub(crate) use focus::{PageBlockFocusRequest, PageFocusSession};
pub(crate) use page_link_icon::{
    PageLinkIconController, PageLinkIconPickerControlMenu, PageLinkIconPickerState,
    PageLinkIconPickerTab, PageLinkNamedIconPreference,
};
use page_link_icon::{PageLinkIconTarget, PageLinkIconView, PageLinkNamedIconSelection};
pub(in crate::ui) use simple_table::PageSimpleTableShortcut;

pub(crate) use document_edit::PagePendingCrossBlockComposition;
pub(in crate::ui::board_workspace::page) use mention::mention_menu_token;
#[cfg(test)]
pub(crate) use mention::PageMentionMenuRow;

#[cfg(any(test, feature = "test-support"))]
pub(crate) use mention::PageMentionClock;
pub(crate) use mention::PageMentionController;

pub(crate) use persistence::{PageMutationAction, PageMutationPlan};

pub(in crate::ui::board_workspace) use render::{
    handle_page_render_action, PageBlockRenderer, PageComposerRenderAction,
    PageDocumentRenderAction, PageRenderAction, PageTitleRenderer,
};
