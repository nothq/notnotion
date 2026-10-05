use gpui::{AppContext, IntoElement, MouseButton};

use crate::ui::{PageColumnFlow, PageFlowColumns};

use super::super::super::super::super::{
    alpha, div, px, AnyElement, App, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled,
};
use super::super::{PageBlockRenderer, PageFlowRenderContext};
use crate::ui::board_workspace::page::editor::render::{PageDragRenderAction, PageRenderAction};

type Divider = gpui::Stateful<gpui::Div>;

/// The divider between two adjacent columns, and its width.
pub(super) struct PageFlowColumnDivider<'a> {
    pub(super) left: &'a PageColumnFlow,
    pub(super) right: &'a PageColumnFlow,
    pub(super) width: f32,
}

pub(super) fn render_page_flow_column_divider(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    columns: &PageFlowColumns,
    divider: PageFlowColumnDivider<'_>,
    cx: &mut App,
) -> AnyElement {
    let PageFlowColumnDivider { left, right, width } = divider;
    let column_list_id = match &columns.key {
        crate::ui::PageFlowNodeKey::Columns {
            column_list_block_id,
        } => column_list_block_id,
        crate::ui::PageFlowNodeKey::Section { .. } => {
            unreachable!("Columns divider must retain a Columns node key")
        }
    };
    let drag = columns.resize_authority_complete.then(|| {
        super::super::super::super::super::PageColumnResizeDrag::new(
            context.data.clone(),
            context.observation.surface(),
            column_list_id.clone(),
            left,
            right,
        )
    });
    let divider = div()
        .id(format!(
            "notion-column-divider-{}-{}",
            left.column_block_id, right.column_block_id
        ))
        .relative()
        .w(px(width))
        .min_w(px(width))
        .max_w(px(width))
        .self_stretch()
        .flex_none();
    let divider = match drag {
        Some(drag) => wire_resizable_divider(renderer, divider, drag, cx),
        None => divider,
    };
    divider
        .child(
            div()
                .absolute()
                .left(px((width - 4.0) / 2.0))
                .top(px(0.0))
                .w(px(4.0))
                .h_full()
                .bg(alpha(0x1c1301, 0.11)),
        )
        .into_any_element()
}

fn wire_resizable_divider(
    renderer: &PageBlockRenderer,
    divider: Divider,
    drag: super::super::super::super::super::PageColumnResizeDrag,
    _cx: &mut App,
) -> Divider {
    let actions = renderer.actions.clone();
    divider
        .cursor_col_resize()
        .occlude()
        .block_mouse_except_scroll()
        .on_drag(drag, move |dragged, _, window, cx| {
            cx.stop_propagation();
            actions.emit(
                PageRenderAction::Drag(PageDragRenderAction::BeginColumnResize {
                    drag: dragged.clone(),
                    pointer_x: window.mouse_position().x.as_f32(),
                }),
                window,
                cx,
            );
            cx.new(|_| dragged.clone())
        })
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}
