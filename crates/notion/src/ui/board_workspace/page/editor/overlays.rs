use gpui::{img, HighlightStyle, SharedString, StyledText, TextStyle};

use super::render::{
    render_page_block_editable_content, render_page_block_image_preview,
    render_page_block_structural_content, render_unsupported_page_leaf, NumberedEditableBlock,
    PageBlockVisualStyle,
};
use super::rich_text::page_text_input_highlights;
use super::support::page_block_editable_input_spec;
use super::{
    alpha, div, page_block_foreground, page_block_visual_color, page_to_do_border_color, px,
    render_page_link_marker, render_page_toggle_marker, rgb, AnyElement, AppearanceMode,
    CardPageBlockColor, CardPageBlockColorValue, CardPageBlockContent, CardPageEditableBlock,
    Context, Div, FluentBuilder, FontWeight, IntoElement, LoadedCardPageData, PageBlockDragPayload,
    PageBlockDragPreviewRow, PageBlockDragScrollTarget, ParentElement, Render, Styled, Theme,
    Window,
};
use crate::ui::surface::PageEditorState;

mod table;

#[derive(Clone, Copy)]
struct PageBlockDragPreviewStyle<'a> {
    theme: Theme,
    appearance_mode: AppearanceMode,
    toggle_markers: &'a [std::sync::Arc<gpui::RenderImage>; 10],
    page_marker: &'a std::sync::Arc<gpui::RenderImage>,
    checked_marker: &'a std::sync::Arc<gpui::RenderImage>,
}

impl Render for PageBlockDragPayload {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance_mode(self.appearance_mode);
        let preview = self.visible_preview();
        div()
            .pl(self.cursor_offset.x + px(19.0))
            .pt(self.cursor_offset.y - px(20.0))
            .child(
                div()
                    .relative()
                    .w(px(self.preview_width.current()))
                    .child(render_page_block_drag_preview_rows(
                        preview.rows_before_anchor,
                        preview.rows_from_anchor,
                        PageBlockDragPreviewStyle {
                            theme,
                            appearance_mode: self.appearance_mode,
                            toggle_markers: &self.toggle_markers,
                            page_marker: &self.page_marker,
                            checked_marker: &self.checked_marker,
                        },
                    ))
                    .when(preview.overflow_count > 0, |container| {
                        container.child(render_page_block_drag_overflow_count(
                            preview.overflow_count,
                        ))
                    }),
            )
    }
}

impl PageBlockDragPreviewRow {
    fn render_row(&self, style: PageBlockDragPreviewStyle<'_>) -> Div {
        div()
            .w_full()
            .pl(px(self.depth as f32 * super::PAGE_BLOCK_INDENT))
            .child(self.render_content(style))
    }

    fn render_content(&self, style: PageBlockDragPreviewStyle<'_>) -> Div {
        let PageBlockDragPreviewStyle {
            theme,
            appearance_mode,
            page_marker,
            ..
        } = style;
        match &self.content {
            CardPageBlockContent::Editable(editable) => {
                self.render_editable_content(editable, style)
            }
            CardPageBlockContent::Alias(alias) => {
                self.render_alias_content(alias, appearance_mode, page_marker)
            }
            CardPageBlockContent::Structural(structural) => {
                render_page_block_structural_content(structural, theme)
            }
            CardPageBlockContent::Resource(resource) => render_page_block_image_preview(
                resource.image(),
                self.cached_block_image.clone(),
                theme,
            ),
            CardPageBlockContent::UnsupportedLeaf(unsupported) => {
                render_unsupported_page_leaf(unsupported, theme)
            }
            CardPageBlockContent::Table { table } => {
                self.render_simple_table_content(table, theme, appearance_mode)
            }
            CardPageBlockContent::TableRow { .. } => {
                unreachable!("internal table rows must not enter drag previews")
            }
            CardPageBlockContent::Layout(_) | CardPageBlockContent::OpaqueUnavailable { .. } => {
                unreachable!("non-visible page blocks must not enter drag previews")
            }
        }
    }

    fn render_editable_content(
        &self,
        editable: &CardPageEditableBlock,
        style: PageBlockDragPreviewStyle<'_>,
    ) -> Div {
        let PageBlockDragPreviewStyle {
            theme,
            appearance_mode,
            toggle_markers,
            page_marker,
            checked_marker,
        } = style;
        let color = page_block_visual_color(editable.kind, self.color);
        let marker = match editable.kind {
            super::CardPageBlockKind::ToggleList => Some(
                render_page_toggle_marker(
                    toggle_markers[page_block_drag_toggle_color_index(color)].clone(),
                )
                .into_any_element(),
            ),
            super::CardPageBlockKind::ToDoList => Some(render_page_block_drag_to_do_marker(
                editable
                    .to_do_state()
                    .expect("drag-preview to-do block must retain typed checked state")
                    .is_checked(),
                color,
                appearance_mode,
                theme.text_secondary,
                checked_marker,
            )),
            super::CardPageBlockKind::PageLink => {
                Some(render_page_link_marker(page_marker.clone()).into_any_element())
            }
            _ => None,
        };
        render_page_block_editable_content(
            NumberedEditableBlock {
                editable,
                numbered_index: self.numbered_index,
            },
            PageBlockVisualStyle::new(theme, appearance_mode, color),
            self.row_spacing
                .expect("editable drag-preview rows require precomputed spacing"),
            marker,
            self.render_text(editable, appearance_mode, color)
                .into_any_element(),
        )
    }

    fn render_alias_content(
        &self,
        alias: &crate::model::CardPageAliasBlock,
        appearance_mode: AppearanceMode,
        page_marker: &std::sync::Arc<gpui::RenderImage>,
    ) -> Div {
        div().w_full().p(px(6.0)).child(
            div()
                .h(px(28.0))
                .p(px(2.0))
                .flex()
                .items_center()
                .child(
                    div()
                        .size(px(24.0))
                        .mr(px(4.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(img(page_marker.clone()).size(px(20.0))),
                )
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::MEDIUM)
                        .line_height(gpui::relative(1.3))
                        .text_color(page_block_foreground(self.color, appearance_mode))
                        .child(alias.title.clone()),
                ),
        )
    }

    fn render_text(
        &self,
        editable: &CardPageEditableBlock,
        appearance_mode: AppearanceMode,
        color: CardPageBlockColor,
    ) -> Div {
        let spec = page_block_editable_input_spec(editable, self.format);
        div()
            .min_w(px(0.0))
            .min_h(px(spec.line_height + spec.input_padding_y * 2.0))
            .px(px(spec.input_padding_x))
            .py(px(spec.input_padding_y))
            .flex_grow(1.0)
            .child(page_block_drag_styled_text(
                editable,
                appearance_mode,
                color,
                spec,
            ))
    }
}

fn render_page_block_drag_preview_rows(
    rows_before_anchor: &[PageBlockDragPreviewRow],
    rows_from_anchor: &[PageBlockDragPreviewRow],
    style: PageBlockDragPreviewStyle<'_>,
) -> Div {
    div()
        .opacity(0.4)
        .flex()
        .flex_col()
        .child(div().relative().w_full().h(px(0.0)).when(
            !rows_before_anchor.is_empty(),
            |anchor| {
                anchor.child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .flex_col()
                        .children(rows_before_anchor.iter().map(|row| row.render_row(style))),
                )
            },
        ))
        .children(rows_from_anchor.iter().map(|row| row.render_row(style)))
}

fn render_page_block_drag_overflow_count(overflow_count: usize) -> Div {
    div()
        .absolute()
        .left(px(4.0))
        .top(px(-24.0))
        .h(px(20.0))
        .min_w(px(20.0))
        .px(px(6.0))
        .rounded(px(10.0))
        .bg(rgb(0x2383e2))
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0xffffff))
        .flex()
        .items_center()
        .justify_center()
        .child(format!("+{overflow_count}"))
}

fn page_block_drag_styled_text(
    editable: &CardPageEditableBlock,
    appearance_mode: AppearanceMode,
    color: CardPageBlockColor,
    spec: super::support::PageBlockInputSpec,
) -> StyledText {
    let highlights = page_text_input_highlights(editable, appearance_mode, color);
    let font_overrides = highlights
        .iter()
        .filter(|highlight| highlight.monospace)
        .map(|highlight| {
            (
                highlight.range.clone(),
                highlight
                    .font_family
                    .clone()
                    .unwrap_or_else(|| SharedString::from("Monaco")),
            )
        })
        .collect::<Vec<_>>();
    let highlight_styles = highlights.into_iter().map(|highlight| {
        (
            highlight.range,
            HighlightStyle {
                color: Some(highlight.color),
                background_color: highlight.background,
                font_weight: highlight.font_weight,
                font_style: highlight.font_style,
                underline: highlight.underline,
                strikethrough: highlight.strikethrough,
                ..Default::default()
            },
        )
    });
    let default_style = TextStyle {
        color: page_block_foreground(color, appearance_mode),
        font_size: px(spec.font_size).into(),
        line_height: px(spec.line_height).into(),
        font_weight: spec.font_weight,
        ..Default::default()
    };
    StyledText::new(editable.text.clone())
        .with_default_highlights(&default_style, highlight_styles)
        .with_font_family_overrides(font_overrides)
}

fn page_block_drag_toggle_color_index(color: CardPageBlockColor) -> usize {
    match color {
        CardPageBlockColor::Text(value) => value.index(),
        CardPageBlockColor::Background(_) => CardPageBlockColorValue::Default.index(),
    }
}

fn render_page_block_drag_to_do_marker(
    checked: bool,
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
    dark_default: u32,
    checked_marker: &std::sync::Arc<gpui::RenderImage>,
) -> AnyElement {
    div()
        .size(px(16.0))
        .rounded(px(1.5))
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |checkbox| {
            checkbox
                .bg(rgb(0x2783de))
                .child(img(checked_marker.clone()).size(px(14.0)))
        })
        .when(!checked, |checkbox| {
            checkbox
                .border(px(1.5))
                .border_color(page_to_do_border_color(
                    color,
                    appearance_mode,
                    dark_default,
                ))
        })
        .into_any_element()
}

impl PageEditorState {
    pub(crate) fn render_page_block_drop_overlay(
        &self,
        data: &LoadedCardPageData,
        scroll_target: PageBlockDragScrollTarget,
    ) -> Option<AnyElement> {
        let drag = self.drag.borrow();
        let target = drag.target.as_ref().filter(|target| {
            target.scroll_target == scroll_target && target.page_id == data.page.block_id
        })?;
        Some(
            div()
                .absolute()
                .inset_0()
                .when_some(target.wash, |overlay, wash| {
                    overlay.child(
                        div()
                            .absolute()
                            .left(px(wash.left))
                            .w(px(wash.width))
                            .top(px(wash.top))
                            .h(px(wash.height))
                            .bg(alpha(0x2383e2, 0.07)),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .left(px(target.indicator_left))
                        .w(px(target.indicator_width))
                        .top(px(target.indicator_top))
                        .h(px(super::PAGE_BLOCK_DROP_LINE_HEIGHT))
                        .bg(alpha(0x2383e2, 0.43)),
                )
                .into_any_element(),
        )
    }
}
