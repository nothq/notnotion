use super::super::{
    alpha, div, page_block_foreground, page_callout_background, page_callout_border, px,
    render_page_callout_marker, rgb, rgba, AnyElement, AppearanceMode, CardPageBlockColor,
    CardPageBlockKind, CardPageStructuralBlock, Div, ElementId, FluentBuilder, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    Theme,
};
use crate::ui::{PageFlowCalloutPresentationSpec, PageFlowCalloutSegment};

mod editable;

pub(in crate::ui::board_workspace::page) use editable::{
    render_page_block_editable_content, NumberedEditableBlock,
};

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page) struct PageBlockVisualStyle {
    theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    color: CardPageBlockColor,
    pub(super) selected: bool,
}

impl PageBlockVisualStyle {
    pub(in crate::ui::board_workspace::page) const fn new(
        theme: Theme,
        appearance_mode: AppearanceMode,
        color: CardPageBlockColor,
    ) -> Self {
        Self {
            theme,
            appearance_mode,
            color,
            selected: false,
        }
    }

    pub(in crate::ui::board_workspace::page) const fn with_selection(
        mut self,
        selected: bool,
    ) -> Self {
        self.selected = selected;
        self
    }
}

pub(in crate::ui::board_workspace::page::editor) fn render_page_callout_container_content(
    visual: PageBlockVisualStyle,
    content: AnyElement,
    marker: Option<AnyElement>,
    accessibility_id: Option<ElementId>,
) -> Div {
    render_page_callout_segment_content(
        visual,
        PageFlowCalloutPresentationSpec::notion(PageFlowCalloutSegment::Single),
        content,
        marker,
        accessibility_id,
    )
}

pub(in crate::ui::board_workspace::page::editor) fn render_page_callout_segment_content(
    visual: PageBlockVisualStyle,
    presentation: PageFlowCalloutPresentationSpec,
    content: AnyElement,
    marker: Option<AnyElement>,
    accessibility_id: Option<ElementId>,
) -> Div {
    let is_first = presentation.segment.is_first();
    let is_last = presentation.segment.is_last();
    let frame = page_callout_segment_frame(visual, presentation, content, marker);
    let frame = match accessibility_id {
        Some(id) => frame.id(id).role(Role::Note).into_any_element(),
        None => frame.into_any_element(),
    };
    div()
        .px(px(presentation.outer_horizontal_inset))
        .when(is_first, |container| {
            container.pt(px(presentation.row_spacing))
        })
        .when(is_last, |container| {
            container.pb(px(presentation.row_spacing))
        })
        .w_full()
        .child(frame)
}

fn page_callout_segment_frame(
    visual: PageBlockVisualStyle,
    presentation: PageFlowCalloutPresentationSpec,
    content: AnyElement,
    marker: Option<AnyElement>,
) -> Div {
    let border = page_callout_border(visual.color, visual.appearance_mode);
    let foreground = page_block_foreground(visual.color, visual.appearance_mode);
    let is_first = presentation.segment.is_first();
    let is_last = presentation.segment.is_last();
    let mut frame = div()
        .relative()
        .w_full()
        .when(
            presentation.segment == PageFlowCalloutSegment::Single,
            |frame| frame.min_h(px(presentation.single_minimum_extent)),
        )
        .border_l(px(presentation.border_extent))
        .border_r(px(presentation.border_extent))
        .when(is_first, |frame| {
            frame
                .border_t(px(presentation.border_extent))
                .rounded_t(px(presentation.corner_radius))
        })
        .when(is_last, |frame| {
            frame
                .border_b(px(presentation.border_extent))
                .rounded_b(px(presentation.corner_radius))
        })
        .border_color(border)
        .when_some(
            page_callout_background(visual.color, visual.appearance_mode),
            |frame, background| frame.bg(background),
        )
        .text_color(foreground)
        .px(px(presentation.frame_horizontal_inset))
        .when(is_first, |frame| {
            frame.pt(px(presentation.frame_vertical_inset))
        })
        .when(is_last, |frame| {
            frame.pb(px(presentation.frame_vertical_inset))
        })
        .flex()
        .items_start();
    if visual.selected {
        frame = frame.child(div().absolute().inset_0().bg(alpha(0x2383e2, 0.14)));
    }
    frame
        .child(page_callout_marker_column(visual, presentation, marker))
        .child(div().min_w(px(0.0)).flex_grow(1.0).child(content))
}

fn page_callout_marker_column(
    visual: PageBlockVisualStyle,
    presentation: PageFlowCalloutPresentationSpec,
    marker: Option<AnyElement>,
) -> Div {
    div().w(px(presentation.marker_extent)).flex_none().when(
        presentation.segment.is_first(),
        |icon| {
            icon.mt(px(presentation.marker_top_inset)).child(
                marker
                    .unwrap_or_else(|| render_page_callout_marker(visual.theme).into_any_element()),
            )
        },
    )
}

fn page_block_color_frame(frame: Div, background: Option<Hsla>, selected: bool) -> Div {
    let has_background = background.is_some();
    let mut container = div()
        .relative()
        .w_full()
        .when_some(background, |container, color| {
            container.rounded(px(6.0)).bg(color)
        });
    if selected {
        container = container.child(
            div()
                .absolute()
                .inset_0()
                .when(has_background, |wash| wash.rounded(px(6.0)))
                .bg(alpha(0x2383e2, 0.14)),
        );
    }
    container.child(frame)
}

pub(in crate::ui::board_workspace::page::editor) fn render_page_block_structural_content(
    structural: &CardPageStructuralBlock,
    theme: Theme,
) -> Div {
    match structural {
        CardPageStructuralBlock::Divider => div()
            .w_full()
            .h(px(13.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .child(div().w_full().h(px(1.0)).bg(rgba(theme.surface_border))),
        CardPageStructuralBlock::CollectionView { title } => {
            render_page_collection_placeholder(title.as_deref(), "Inline database", theme)
        }
        CardPageStructuralBlock::CollectionViewPage { title } => {
            render_page_collection_placeholder(title.as_deref(), "Database page", theme)
        }
    }
}

fn render_page_collection_placeholder(
    title: Option<&str>,
    label: &'static str,
    theme: Theme,
) -> Div {
    div().w_full().py(px(5.0)).child(
        div()
            .w_full()
            .min_h(px(64.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgba(theme.surface_border))
            .bg(rgba(theme.page_icon_bg))
            .px(px(12.0))
            .py(px(10.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .size(px(24.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgba(theme.surface_border))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(rgb(theme.text_muted))
                    .child("▦"),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(theme.text_primary))
                            .child(title.unwrap_or("Untitled database").to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(theme.text_muted))
                            .child(label),
                    ),
            ),
    )
}

fn render_page_block_marker(
    kind: CardPageBlockKind,
    numbered_index: usize,
    foreground: Hsla,
    marker_override: Option<AnyElement>,
) -> AnyElement {
    match kind {
        CardPageBlockKind::BulletedList => div()
            .w(px(24.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(5.0)).rounded_full().bg(foreground))
            .into_any_element(),
        CardPageBlockKind::NumberedList => div()
            .w(px(24.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_end()
            .text_size(px(16.0))
            .text_color(foreground)
            .child(format!("{numbered_index}."))
            .into_any_element(),
        CardPageBlockKind::ToDoList => div()
            .w(px(24.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .child(marker_override.unwrap_or_else(|| {
                div()
                    .size(px(16.0))
                    .rounded(px(1.5))
                    .border(px(1.5))
                    .border_color(foreground)
                    .into_any_element()
            }))
            .into_any_element(),
        CardPageBlockKind::ToggleList => div()
            .w(px(24.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .child(marker_override.expect("toggle rows require a cached disclosure marker"))
            .into_any_element(),
        CardPageBlockKind::PageLink => div()
            .w(px(24.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .child(marker_override.expect("page-link rows require a cached page marker"))
            .into_any_element(),
        _ => div().into_any_element(),
    }
}
