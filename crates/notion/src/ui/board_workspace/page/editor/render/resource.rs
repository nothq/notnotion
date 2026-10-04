use crate::ui::{
    alpha, div, img, px, rgb, CachedNotionBlockImage, CardPageImageBlock, CardPageImageSource,
    CardPageUnsupportedLeafBlock, Div, FluentBuilder, FontWeight, NotionBlockImageRequester,
    NotionBlockImageState, ObjectFit, ParentElement, Styled, StyledImage, Theme,
};
use gpui::App;

use super::row::PageBlockRenderContext;
use super::PageBlockRenderer;

const IMAGE_PADDING: f32 = 8.0;
const EMPTY_IMAGE_HEIGHT: f32 = 49.0;
const DRAG_PLACEHOLDER_HEIGHT: f32 = 120.0;

impl PageBlockRenderer {
    pub(super) fn render_page_block_image(
        &self,
        context: &PageBlockRenderContext<'_>,
        block_image: &CardPageImageBlock,
        cx: &mut App,
    ) -> Div {
        let content = match block_image.source() {
            CardPageImageSource::Empty => render_empty_page_block_image(self.theme),
            source => {
                let key = source
                    .fetch_key()
                    .expect("configured image sources must retain a fetch key");
                match self.block_images.image(
                    key,
                    NotionBlockImageRequester {
                        data: context.data,
                        observation: context.observation,
                        block_id: &context.block.block_id,
                    },
                    cx,
                ) {
                    NotionBlockImageState::Ready(cached) => render_decoded_page_block_image(cached),
                    NotionBlockImageState::Pending => {
                        render_pending_page_block_image(block_image, self.theme)
                    }
                    NotionBlockImageState::Failed => render_failed_page_block_image(
                        source.display_host().map(|host| host.as_str()),
                        self.theme,
                    ),
                }
            }
        };
        let selected = self
            .interaction
            .block_selected_without_drag(&context.block.block_id);
        image_wrapper(content, selected)
    }
}

pub(in crate::ui::board_workspace::page::editor) fn render_page_block_image_preview(
    block_image: &CardPageImageBlock,
    cached: Option<CachedNotionBlockImage>,
    theme: Theme,
) -> Div {
    let content = match (block_image.source(), cached) {
        (CardPageImageSource::Empty, _) => render_empty_page_block_image(theme),
        (_, Some(cached)) => render_decoded_page_block_image(cached),
        (_, None) => render_page_block_image_drag_placeholder(theme),
    };
    image_wrapper(content, false)
}

pub(in crate::ui::board_workspace::page::editor) fn render_unsupported_page_leaf(
    _leaf: &CardPageUnsupportedLeafBlock,
    theme: Theme,
) -> Div {
    div().w_full().py(px(4.0)).child(
        div()
            .w_full()
            .min_h(px(32.0))
            .px(px(10.0))
            .rounded(px(3.0))
            .border_1()
            .border_color(alpha(theme.text_primary, 0.1))
            .bg(alpha(theme.text_primary, 0.025))
            .flex()
            .items_center()
            .child(
                div()
                    .ml(px(10.0))
                    .w(px(96.0))
                    .h(px(8.0))
                    .rounded(px(4.0))
                    .bg(alpha(theme.text_primary, 0.08)),
            ),
    )
}

fn image_wrapper(content: Div, selected: bool) -> Div {
    div()
        .relative()
        .w_full()
        .p(px(IMAGE_PADDING))
        .child(content)
        .when(selected, |wrapper| {
            wrapper.child(div().absolute().inset_0().bg(alpha(0x2383e2, 0.14)))
        })
}

fn render_empty_page_block_image(theme: Theme) -> Div {
    div()
        .w_full()
        .h(px(EMPTY_IMAGE_HEIGHT))
        .rounded(px(3.0))
        .border_1()
        .border_color(alpha(theme.text_primary, 0.12))
        .bg(alpha(theme.text_primary, 0.025))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .text_size(px(14.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme.text_secondary))
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(theme.text_muted))
                .child("▧"),
        )
        .child("Add an image")
}

fn render_decoded_page_block_image(cached: CachedNotionBlockImage) -> Div {
    let width = cached.width as f32;
    let height = cached.height as f32;
    div()
        .w(px(width))
        .max_w_full()
        .aspect_ratio(width / height)
        .overflow_hidden()
        .child(
            img(cached.rendered)
                .size_full()
                .object_fit(ObjectFit::Contain),
        )
}

fn render_pending_page_block_image(block_image: &CardPageImageBlock, theme: Theme) -> Div {
    let skeleton = div().rounded(px(3.0)).bg(alpha(theme.text_primary, 0.08));
    match block_image.size_hint() {
        Some(size_hint) => {
            let width = size_hint.width() as f32;
            let height = size_hint.height() as f32;
            skeleton
                .w(px(width))
                .max_w_full()
                .aspect_ratio(width / height)
        }
        None => skeleton.w_full().h(px(EMPTY_IMAGE_HEIGHT)),
    }
}

fn render_failed_page_block_image(display_host: Option<&str>, theme: Theme) -> Div {
    div()
        .w_full()
        .aspect_ratio(1.0)
        .rounded(px(3.0))
        .border_1()
        .border_color(alpha(theme.text_primary, 0.1))
        .bg(alpha(theme.text_primary, 0.025))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .child(
            div()
                .text_size(px(28.0))
                .text_color(rgb(theme.text_muted))
                .child("▧"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme.text_secondary))
                .child("This image couldn’t be found."),
        )
        .when_some(display_host, |failure, display_host| {
            failure.child(
                div()
                    .max_w_full()
                    .px(px(24.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(12.0))
                    .text_color(rgb(theme.text_muted))
                    .child(display_host.to_string()),
            )
        })
}

fn render_page_block_image_drag_placeholder(theme: Theme) -> Div {
    div()
        .w_full()
        .h(px(DRAG_PLACEHOLDER_HEIGHT))
        .rounded(px(3.0))
        .border_1()
        .border_color(alpha(theme.text_primary, 0.1))
        .bg(alpha(theme.text_primary, 0.04))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(22.0))
        .text_color(rgb(theme.text_muted))
        .child("▧")
}
