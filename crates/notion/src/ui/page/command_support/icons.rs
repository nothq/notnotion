use std::sync::Arc;

use gpui::{img, RenderImage};

use super::{
    alpha, div, px, rgb, Div, FontWeight, PageComposerState, ParentElement, Styled, Theme,
};
use crate::ui::CardPageBlockKind;
use crate::ui::LoadedCardPageData;

pub(crate) fn generated_local_page_id() -> String {
    generated_notion_record_id()
}

pub(crate) fn generated_notion_record_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(crate) fn page_command_placeholder(kind: CardPageBlockKind) -> &'static str {
    match kind {
        CardPageBlockKind::Text => "Type something...",
        CardPageBlockKind::SubHeader => "Heading",
        CardPageBlockKind::SubSubHeader => "Section heading",
        CardPageBlockKind::Heading3 => "Heading 3",
        CardPageBlockKind::Heading4 => "Heading 4",
        CardPageBlockKind::BulletedList
        | CardPageBlockKind::NumberedList
        | CardPageBlockKind::ToDoList => "List item",
        CardPageBlockKind::ToggleList => "Toggle",
        CardPageBlockKind::PageLink => "New page",
        CardPageBlockKind::Callout => "Callout",
        CardPageBlockKind::Quote => "Quote",
        CardPageBlockKind::Code => "",
    }
}

pub(crate) fn page_composer_numbered_index(
    data: &LoadedCardPageData,
    composer: &PageComposerState,
) -> usize {
    if composer.block_kind != CardPageBlockKind::NumberedList {
        return 0;
    }
    data.next_root_numbered_index
}

pub(crate) fn render_page_command_glyph_icon(theme: Theme, glyph: &'static str) -> Div {
    let text_size = if glyph.len() > 1 { 9.5 } else { 11.0 };
    div()
        .w(px(18.0))
        .h(px(14.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme.text_secondary))
                .child(glyph),
        )
}

pub(crate) fn render_page_command_list_row(marker: Div, line_width: f32, color: u32) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(3.0))
        .child(marker)
        .child(div().w(px(line_width)).h(px(1.0)).bg(rgb(color)))
}

pub(crate) fn render_page_command_bulleted_list_icon(theme: Theme) -> Div {
    div()
        .w(px(18.0))
        .h(px(14.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .child(render_page_command_list_row(
            div()
                .size(px(2.5))
                .rounded_full()
                .bg(rgb(theme.text_secondary)),
            9.0,
            theme.text_secondary,
        ))
        .child(render_page_command_list_row(
            div()
                .size(px(2.5))
                .rounded_full()
                .bg(rgb(theme.text_secondary)),
            9.0,
            theme.text_secondary,
        ))
        .child(render_page_command_list_row(
            div()
                .size(px(2.5))
                .rounded_full()
                .bg(rgb(theme.text_secondary)),
            7.0,
            theme.text_secondary,
        ))
}

pub(crate) fn render_page_command_numbered_list_icon(theme: Theme) -> Div {
    div()
        .w(px(18.0))
        .h(px(14.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .child(render_page_command_list_row(
            div()
                .w(px(4.0))
                .text_size(px(6.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme.text_secondary))
                .child("1"),
            9.0,
            theme.text_secondary,
        ))
        .child(render_page_command_list_row(
            div()
                .w(px(4.0))
                .text_size(px(6.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme.text_secondary))
                .child("2"),
            9.0,
            theme.text_secondary,
        ))
}

pub(crate) fn render_page_command_todo_list_icon(theme: Theme) -> Div {
    div()
        .w(px(18.0))
        .h(px(14.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .child(render_page_command_list_row(
            div()
                .size(px(5.0))
                .rounded(px(1.5))
                .border_1()
                .border_color(alpha(theme.text_secondary, 1.0)),
            8.0,
            theme.text_secondary,
        ))
        .child(render_page_command_list_row(
            div()
                .size(px(5.0))
                .rounded(px(1.5))
                .border_1()
                .border_color(alpha(theme.text_secondary, 1.0)),
            6.0,
            theme.text_secondary,
        ))
}

pub(crate) fn render_page_command_toggle_list_icon(theme: Theme, marker: Arc<RenderImage>) -> Div {
    div()
        .w(px(18.0))
        .h(px(14.0))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .child(render_page_command_list_row(
            render_page_command_toggle_marker(marker.clone()),
            8.0,
            theme.text_secondary,
        ))
        .child(render_page_command_list_row(
            render_page_command_toggle_marker(marker),
            6.0,
            theme.text_secondary,
        ))
}

fn render_page_command_toggle_marker(marker: Arc<RenderImage>) -> Div {
    div()
        .w(px(5.0))
        .h(px(7.0))
        .flex()
        .items_center()
        .justify_center()
        .child(img(marker).size(px(5.0)))
}

pub(crate) fn render_page_toggle_marker(marker: Arc<RenderImage>) -> Div {
    div()
        .w(px(14.0))
        .h(px(14.0))
        .flex()
        .items_center()
        .justify_center()
        .child(img(marker).size(px(12.8)))
}

pub(crate) fn render_page_link_marker(marker: Arc<RenderImage>) -> Div {
    div()
        .size(px(18.0))
        .flex()
        .items_center()
        .justify_center()
        .child(img(marker).size(px(18.0)))
}

pub(crate) fn render_page_callout_marker(_theme: Theme) -> Div {
    div()
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(20.0))
                .line_height(gpui::relative(1.2))
                .child("💡"),
        )
}

pub(crate) fn render_page_command_callout_icon(theme: Theme) -> Div {
    div()
        .size(px(14.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(alpha(theme.text_secondary, 1.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(8.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme.text_secondary))
                .child("T"),
        )
}
