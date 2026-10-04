use super::super::super::support::PageBlockRowSpacing;
use super::super::super::{
    div, page_block_background, page_block_foreground, px, AnyElement, CardPageBlockKind,
    CardPageEditableBlock, Div, Hsla, IntoElement, ParentElement, Styled,
};
use super::{
    page_block_color_frame, render_page_block_marker, render_page_callout_container_content,
    PageBlockVisualStyle,
};

struct PageListContentRender {
    kind: CardPageBlockKind,
    numbered_index: usize,
    foreground: Hsla,
    background: Option<Hsla>,
    selected: bool,
    spacing: PageBlockRowSpacing,
    marker_override: Option<AnyElement>,
    body: AnyElement,
}

/// An editable block with its number in a numbered-list run.
#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page) struct NumberedEditableBlock<'a> {
    pub(in crate::ui::board_workspace::page) editable: &'a CardPageEditableBlock,
    pub(in crate::ui::board_workspace::page) numbered_index: usize,
}

pub(in crate::ui::board_workspace::page) fn render_page_block_editable_content(
    block: NumberedEditableBlock<'_>,
    visual: PageBlockVisualStyle,
    spacing: PageBlockRowSpacing,
    marker_override: Option<AnyElement>,
    body: AnyElement,
) -> Div {
    let NumberedEditableBlock {
        editable,
        numbered_index,
    } = block;
    let foreground = page_block_foreground(visual.color, visual.appearance_mode);
    let background = page_block_background(visual.color, visual.appearance_mode);
    match editable.kind {
        CardPageBlockKind::Text
        | CardPageBlockKind::SubHeader
        | CardPageBlockKind::SubSubHeader
        | CardPageBlockKind::Heading3
        | CardPageBlockKind::Heading4 => {
            render_page_text_content(background, visual.selected, spacing, body)
        }
        CardPageBlockKind::BulletedList
        | CardPageBlockKind::NumberedList
        | CardPageBlockKind::ToDoList
        | CardPageBlockKind::ToggleList
        | CardPageBlockKind::PageLink => render_page_list_content(PageListContentRender {
            kind: editable.kind,
            numbered_index,
            foreground,
            background,
            selected: visual.selected,
            spacing,
            marker_override,
            body,
        }),
        CardPageBlockKind::Callout => render_page_callout_container_content(
            visual,
            div()
                .w_full()
                .px(px(6.0))
                .py(px(6.0))
                .child(body)
                .into_any_element(),
            None,
            None,
        ),
        CardPageBlockKind::Quote => {
            render_page_quote_content(foreground, background, visual.selected, spacing, body)
        }
        CardPageBlockKind::Code => {
            super::super::code::render_page_code_block_content(visual, spacing, body)
        }
    }
}

fn render_page_text_content(
    background: Option<Hsla>,
    selected: bool,
    spacing: PageBlockRowSpacing,
    body: AnyElement,
) -> Div {
    div()
        .px(px(6.0))
        .pt(px(spacing.top))
        .pb(px(spacing.bottom))
        .w_full()
        .child(page_block_color_frame(
            div().w_full().child(body),
            background,
            selected,
        ))
}

fn render_page_list_content(render: PageListContentRender) -> Div {
    let marker = render_page_block_marker(
        render.kind,
        render.numbered_index,
        render.foreground,
        render.marker_override,
    );
    let content = div()
        .w_full()
        .flex()
        .items_start()
        .pl(px(2.0))
        .child(marker)
        .child(div().min_w(px(0.0)).flex_grow(1.0).child(render.body));
    div()
        .px(px(6.0))
        .pt(px(render.spacing.top))
        .pb(px(render.spacing.bottom))
        .w_full()
        .child(page_block_color_frame(
            content,
            render.background,
            render.selected,
        ))
}

fn render_page_quote_content(
    foreground: Hsla,
    background: Option<Hsla>,
    selected: bool,
    spacing: PageBlockRowSpacing,
    body: AnyElement,
) -> Div {
    let content = div()
        .w_full()
        .border_l_3()
        .border_color(foreground)
        .px(px(14.0))
        .child(body);
    div()
        .px(px(8.0))
        .pt(px(spacing.top))
        .pb(px(spacing.bottom))
        .w_full()
        .child(page_block_color_frame(content, background, selected))
}
