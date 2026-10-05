use crate::model::{CardPageBlock, CardPageBlockContent, CardPageEditableBlock};
use crate::ui::{
    alpha, div, px, rgb, AnyElement, Div, FluentBuilder, InteractiveElement, IntoElement,
    ParentElement, Styled,
};

use super::{
    controls::{notion_search_preview_body_text_color, notion_search_preview_heading_text_color},
    preview_style::{
        notion_search_preview_block_role, notion_search_preview_header_fill,
        notion_search_preview_styled_text, notion_search_preview_text_marker,
        notion_search_preview_text_metrics, NotionSearchPreviewBlockRole,
    },
    QuickFindView,
};

impl QuickFindView {
    pub(super) fn render_notion_search_preview_virtual_block(
        &self,
        visible_row_index: usize,
        row_count: usize,
        data: &crate::ui::LoadedCardPageData,
    ) -> AnyElement {
        let row = &data.visible_rows[visible_row_index];
        let block_index = row.block_index;
        let block = &data.page.blocks[block_index];
        let block = self.render_notion_search_preview_block(
            block_index,
            block,
            PreviewListPlacement {
                depth: row.visual_depth,
                numbered_index: data.numbered_indices[block_index],
                continues_list: row.joins_previous_list_sibling,
            },
        );
        div()
            .w_full()
            .px(px(24.0))
            .when(visible_row_index + 1 == row_count, |this| this.pb(px(24.0)))
            .when_some(block, |this, block| this.child(block))
            .into_any_element()
    }

    fn render_notion_search_preview_block(
        &self,
        block_index: usize,
        block: &CardPageBlock,
        placement: PreviewListPlacement,
    ) -> Option<AnyElement> {
        let visual_depth = placement.depth;
        match &block.content {
            CardPageBlockContent::Editable(editable) if !editable.text.trim().is_empty() => Some(
                self.render_notion_search_preview_editable_block(block_index, editable, placement),
            ),
            CardPageBlockContent::Alias(alias) if !alias.title.trim().is_empty() => Some(
                div()
                    .debug_selector(move || {
                        format!("notion-search-preview-block-{block_index}-page-link")
                    })
                    .w_full()
                    .pl(px(visual_depth.min(4) as f32 * 14.0))
                    .mb(px(4.0))
                    .flex()
                    .items_start()
                    .text_size(px(13.0))
                    .line_height(px(18.0))
                    .text_color(notion_search_preview_body_text_color(self.appearance_mode))
                    .child(
                        div()
                            .w(px(16.0))
                            .flex_none()
                            .text_color(rgb(self.theme.text_muted))
                            .child("↗"),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex_grow(1.0)
                            .child(alias.title.clone()),
                    )
                    .into_any_element(),
            ),
            _ => None,
        }
    }

    fn render_notion_search_preview_editable_block(
        &self,
        block_index: usize,
        editable: &CardPageEditableBlock,
        placement: PreviewListPlacement,
    ) -> AnyElement {
        let PreviewListPlacement {
            depth,
            numbered_index,
            continues_list,
        } = placement;
        let role = notion_search_preview_block_role(editable.kind);
        let (_, line_height, _) = notion_search_preview_text_metrics(role);
        let text = self.notion_search_preview_editable_text(editable, role);
        let row = self.notion_search_preview_editable_row(block_index, depth, continues_list, role);
        self.render_notion_search_preview_editable_role(
            row,
            text,
            editable,
            EditableBlockPresentation {
                role,
                numbered_index,
                line_height,
                depth,
            },
        )
    }

    fn notion_search_preview_editable_text(
        &self,
        editable: &CardPageEditableBlock,
        role: NotionSearchPreviewBlockRole,
    ) -> Div {
        let (font_size, line_height, font_weight) = notion_search_preview_text_metrics(role);
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .text_size(px(font_size))
            .line_height(px(line_height))
            .font_weight(font_weight)
            .text_color(if role.is_heading() {
                notion_search_preview_heading_text_color(self.appearance_mode)
            } else {
                notion_search_preview_body_text_color(self.appearance_mode)
            })
            .child(notion_search_preview_styled_text(
                editable,
                self.appearance_mode,
            ))
    }

    fn notion_search_preview_editable_row(
        &self,
        block_index: usize,
        depth: usize,
        continues_list: bool,
        role: NotionSearchPreviewBlockRole,
    ) -> Div {
        let selector_segment = role.selector_segment();
        div()
            .debug_selector(move || {
                format!("notion-search-preview-block-{block_index}-{selector_segment}")
            })
            .w_full()
            .pl(px(depth.min(4) as f32 * 14.0))
            .mt(px(if continues_list {
                8.0
            } else if block_index > 0 {
                role.top_margin()
            } else {
                0.0
            }))
            .mb(px(role.bottom_margin()))
            .text_color(rgb(self.theme.text_muted))
    }

    fn render_notion_search_preview_editable_role(
        &self,
        row: Div,
        text: Div,
        editable: &CardPageEditableBlock,
        presentation: EditableBlockPresentation,
    ) -> AnyElement {
        let EditableBlockPresentation {
            role,
            numbered_index,
            line_height,
            depth,
        } = presentation;
        match role {
            NotionSearchPreviewBlockRole::Paragraph
            | NotionSearchPreviewBlockRole::HeadingLarge
            | NotionSearchPreviewBlockRole::HeadingMedium
            | NotionSearchPreviewBlockRole::HeadingSmall => row.child(text).into_any_element(),
            NotionSearchPreviewBlockRole::Bulleted => {
                notion_search_preview_marked_row(row, text, "•", line_height)
            }
            NotionSearchPreviewBlockRole::Numbered => notion_search_preview_marked_row(
                row,
                text,
                format!("{numbered_index}."),
                line_height,
            ),
            NotionSearchPreviewBlockRole::ToDo => {
                self.render_notion_search_preview_to_do(row, text, editable, line_height)
            }
            NotionSearchPreviewBlockRole::Toggle => {
                notion_search_preview_marked_row(row, text, "›", line_height)
            }
            NotionSearchPreviewBlockRole::PageLink => {
                notion_search_preview_marked_row(row, text, "↗", line_height)
            }
            NotionSearchPreviewBlockRole::Quote => row
                .border_l_1()
                .border_color(rgb(self.theme.text_muted))
                .pl(px(10.0 + depth.min(4) as f32 * 14.0))
                .child(text)
                .into_any_element(),
            NotionSearchPreviewBlockRole::Callout | NotionSearchPreviewBlockRole::Code => row
                .rounded(px(5.0))
                .px(px(8.0))
                .py(px(6.0))
                .bg(rgb(notion_search_preview_header_fill(self.appearance_mode)))
                .child(text)
                .into_any_element(),
        }
    }

    fn render_notion_search_preview_to_do(
        &self,
        row: Div,
        text: Div,
        editable: &CardPageEditableBlock,
        line_height: f32,
    ) -> AnyElement {
        let checked = editable
            .to_do_state()
            .is_some_and(|state| state.is_checked());
        row.flex()
            .items_start()
            .child(
                div()
                    .w(px(16.0))
                    .h(px(line_height))
                    .flex_none()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .size(px(12.0))
                            .rounded(px(2.0))
                            .border_1()
                            .border_color(rgb(self.theme.text_muted))
                            .when(checked, |this| {
                                this.bg(rgb(0x2383e2)).border_color(rgb(0x2383e2))
                            }),
                    ),
            )
            .child(text)
            .into_any_element()
    }

    pub(super) fn render_notion_search_preview_skeleton(&self) -> AnyElement {
        div()
            .pt(px(12.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children([220.0, 300.0, 260.0].map(|width| {
                div()
                    .w(px(width))
                    .h(px(10.0))
                    .rounded(px(5.0))
                    .bg(alpha(self.theme.text_primary, 0.10))
            }))
            .into_any_element()
    }
}

/// A preview row's nesting depth and its position in a list run.
#[derive(Clone, Copy)]
struct PreviewListPlacement {
    depth: usize,
    numbered_index: usize,
    continues_list: bool,
}

struct EditableBlockPresentation {
    role: NotionSearchPreviewBlockRole,
    numbered_index: usize,
    line_height: f32,
    depth: usize,
}

fn notion_search_preview_marked_row(
    row: Div,
    text: Div,
    marker: impl Into<gpui::SharedString>,
    line_height: f32,
) -> AnyElement {
    row.flex()
        .items_start()
        .child(notion_search_preview_text_marker(marker, line_height))
        .child(text)
        .into_any_element()
}
