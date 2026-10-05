use gpui::img;

use crate::model::{CardPageFont, CardPageFormat};

use super::{
    alpha, div, page_block_foreground, px, rgb, AppearanceMode, Arc, CardPageBlockColor,
    CardPageBlockKind, CardPageEditableBlock, CardPageQuoteSize, Div, FontWeight, Hsla,
    LoadedCardPageData, ParentElement, Styled, TextInputStyle, Theme,
};

mod hierarchy;

pub(super) use hierarchy::{
    indent_page_block_in_page, page_block_subtree_end, page_parent_subtree_end,
    remove_merged_page_block, reparent_direct_children, replace_loaded_card_page,
};

pub(in crate::ui::board_workspace::page) fn page_block_text_input_style(
    spec: PageBlockInputSpec,
    theme: Theme,
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> TextInputStyle {
    let content_height = spec.line_height + spec.input_padding_y * 2.0;
    TextInputStyle {
        height: px(content_height),
        min_height: px(content_height),
        padding_x: px(spec.input_padding_x),
        padding_y: px(spec.input_padding_y),
        radius: px(0.0),
        background: Hsla::from(rgb(0x000000)).opacity(0.0),
        border: Hsla::from(rgb(0x000000)).opacity(0.0),
        focused_border: Hsla::from(rgb(0x000000)).opacity(0.0),
        text: page_block_foreground(color, appearance_mode),
        placeholder: rgb(theme.text_hint).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(spec.font_size),
        line_height: px(spec.line_height),
        font_family: spec.font_family.map(Into::into),
    }
}

pub(super) fn page_title_text_input_style(
    font_size: f32,
    format: CardPageFormat,
    theme: Theme,
) -> TextInputStyle {
    let line_height = font_size * 1.2;
    TextInputStyle {
        height: px(line_height),
        min_height: px(line_height),
        padding_x: px(0.0),
        padding_y: px(0.0),
        radius: px(0.0),
        background: Hsla::from(rgb(0x000000)).opacity(0.0),
        border: Hsla::from(rgb(0x000000)).opacity(0.0),
        focused_border: Hsla::from(rgb(0x000000)).opacity(0.0),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_hint).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(font_size),
        line_height: px(line_height),
        font_family: page_font_family(format).map(Into::into),
    }
}

/// The installed faces of Notion's serif and mono page fonts, whose stacks
/// lead with web fonts notnotion does not ship.
fn page_font_family(format: CardPageFormat) -> Option<&'static str> {
    match format.font {
        CardPageFont::Default => None,
        CardPageFont::Serif => Some("Georgia"),
        CardPageFont::Mono => Some("Menlo"),
    }
}

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page) struct PageBlockInputSpec {
    pub(in crate::ui::board_workspace::page) font_size: f32,
    pub(in crate::ui::board_workspace::page) line_height: f32,
    pub(in crate::ui::board_workspace::page) font_weight: FontWeight,
    pub(in crate::ui::board_workspace::page) input_padding_x: f32,
    pub(in crate::ui::board_workspace::page) input_padding_y: f32,
    pub(in crate::ui::board_workspace::page) top_padding: f32,
    font_family: Option<&'static str>,
}

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page) struct PageBlockRowSpacing {
    pub(in crate::ui::board_workspace::page) top: f32,
    pub(in crate::ui::board_workspace::page) bottom: f32,
}

pub(super) const PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT: f32 = 28.0;

/// A block's type in a page with `format`. Notion sizes block text in `em`
/// from the page's 16px, or 14px with Small text, and sets the page's typeface
/// on everything but code.
pub(in crate::ui::board_workspace::page) fn page_block_input_spec(
    kind: CardPageBlockKind,
    format: CardPageFormat,
) -> PageBlockInputSpec {
    let spec = page_block_default_input_spec(kind);
    let scale = if format.small_text { 14.0 / 16.0 } else { 1.0 };
    PageBlockInputSpec {
        font_size: spec.font_size * scale,
        line_height: spec.line_height * scale,
        font_family: spec.font_family.or(page_font_family(format)),
        ..spec
    }
}

fn page_block_default_input_spec(kind: CardPageBlockKind) -> PageBlockInputSpec {
    let spacing = page_block_default_row_spacing(kind);
    let input_spec = |font_size, line_height, font_weight, input_padding_x, input_padding_y| {
        PageBlockInputSpec {
            font_size,
            line_height,
            font_weight,
            input_padding_x,
            input_padding_y,
            top_padding: spacing.top,
            font_family: None,
        }
    };
    match kind {
        CardPageBlockKind::Text => input_spec(16.0, 24.0, FontWeight::NORMAL, 2.0, 2.0),
        CardPageBlockKind::SubHeader => input_spec(30.0, 39.0, FontWeight::SEMIBOLD, 2.0, 2.0),
        CardPageBlockKind::SubSubHeader => input_spec(24.0, 31.2, FontWeight::SEMIBOLD, 2.0, 2.0),
        CardPageBlockKind::Heading3 => input_spec(20.0, 26.0, FontWeight::SEMIBOLD, 2.0, 2.0),
        CardPageBlockKind::Heading4 => input_spec(18.0, 23.4, FontWeight::SEMIBOLD, 2.0, 2.0),
        CardPageBlockKind::PageLink => input_spec(16.0, 24.0, FontWeight::MEDIUM, 2.0, 2.0),
        CardPageBlockKind::Callout => input_spec(16.0, 24.0, FontWeight::NORMAL, 2.0, 2.0),
        CardPageBlockKind::Quote => input_spec(16.0, 24.0, FontWeight::NORMAL, 8.0, 0.0),
        CardPageBlockKind::Code => {
            input_spec(13.6, 20.4, FontWeight::NORMAL, 0.0, 12.0).with_font_family("SF Mono")
        }
        CardPageBlockKind::BulletedList
        | CardPageBlockKind::NumberedList
        | CardPageBlockKind::ToDoList
        | CardPageBlockKind::ToggleList => input_spec(16.0, 24.0, FontWeight::NORMAL, 2.0, 2.0),
    }
}

pub(super) fn page_block_editable_input_spec(
    editable: &CardPageEditableBlock,
    format: CardPageFormat,
) -> PageBlockInputSpec {
    let spec = page_block_input_spec(editable.kind, format);
    if editable.quote_size() == Some(CardPageQuoteSize::Large) {
        return PageBlockInputSpec {
            font_size: spec.font_size * 1.2,
            line_height: spec.line_height * 1.2,
            ..spec
        };
    }
    spec
}

pub(super) fn page_block_first_line_center(
    row_spacing: PageBlockRowSpacing,
    editable_spec: PageBlockInputSpec,
) -> f32 {
    row_spacing.top + editable_spec.input_padding_y + editable_spec.line_height / 2.0
}

pub(in crate::ui::board_workspace::page) fn page_block_default_row_spacing(
    kind: CardPageBlockKind,
) -> PageBlockRowSpacing {
    page_block_row_spacing_from_projection(crate::ui::PageVisibleRowSpacing::for_kind(kind))
}

pub(super) fn page_block_visible_row_spacing(
    data: &LoadedCardPageData,
    visible_row_index: usize,
) -> PageBlockRowSpacing {
    page_block_row_spacing_from_projection(
        data.visible_row_spacing(visible_row_index)
            .expect("visible editable Notion rows must retain layout spacing"),
    )
}

pub(super) fn page_block_flow_row_spacing(
    data: &LoadedCardPageData,
    visible_row_index: usize,
    adjacency: crate::ui::PageFlowUnitAdjacency,
) -> PageBlockRowSpacing {
    let spacing = crate::ui::visible_row_spacing(
        &data.page,
        &data.visible_rows,
        visible_row_index,
        adjacency.next_owner_visible_row_index,
    )
    .expect("flow-rendered Notion rows must retain editable layout spacing");
    page_block_row_spacing_from_projection(spacing)
}

fn page_block_row_spacing_from_projection(
    spacing: crate::ui::PageVisibleRowSpacing,
) -> PageBlockRowSpacing {
    PageBlockRowSpacing {
        top: spacing.top(),
        bottom: spacing.bottom(),
    }
}

pub(super) fn page_block_visible_nesting_offsets(data: &LoadedCardPageData) -> Arc<[f32]> {
    data.visible_nesting_offsets()
}

impl PageBlockInputSpec {
    const fn with_font_family(mut self, font_family: &'static str) -> Self {
        self.font_family = Some(font_family);
        self
    }
}

pub(super) fn render_six_dot_handle(marker: Arc<gpui::RenderImage>) -> Div {
    div()
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .child(img(marker).size(px(20.0)))
}
