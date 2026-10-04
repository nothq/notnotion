use super::super::drag::{PageBlockDropRequest, PageBlockDropRuntime};
use super::super::support::{
    page_block_editable_input_spec, page_block_first_line_center, render_six_dot_handle,
    PageBlockRowSpacing, PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT,
};
use super::super::{
    alpha, div, img, px, AnyElement, App, Arc, CardPageBlock, CardPageBlockColorValue, Div,
    ElementId, FluentBuilder, InteractiveElement, IntoElement, KeyDownEvent, LoadedCardPageData,
    MouseButton, MouseDownEvent, MouseUpEvent, PageBlockDragScrollTarget, PageBlockDragSelection,
    ParentElement, Role, StatefulInteractiveElement, Styled, PAGE_BLOCK_GUTTER_WIDTH,
    PAGE_BLOCK_INDENT,
};
use super::actions::PageBlockEditRenderAction;
use super::{PageBlockRenderAction, PageBlockRenderer, PageDragRenderAction, PageRenderAction};

mod drag_preview;

use drag_preview::page_block_drag_preview;

impl PageBlockRenderContext<'_> {
    /// The middle of the block's first line, where its gutter and block menu
    /// line up.
    pub(super) fn first_line_center(&self) -> f32 {
        let format = self.data.page.format;
        self.block.editable_content().zip(self.row_spacing).map_or(
            PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT / 2.0,
            |(editable, spacing)| {
                page_block_first_line_center(
                    spacing,
                    page_block_editable_input_spec(editable, format),
                )
            },
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct PageBlockRenderContext<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) index: usize,
    pub(super) block: &'a CardPageBlock,
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) inherited_text_color: Option<CardPageBlockColorValue>,
    pub(super) render_depth: usize,
    pub(super) nesting_offset: f32,
    pub(super) row_spacing: Option<PageBlockRowSpacing>,
    pub(super) render_gutter: bool,
    pub(super) gutter_center: Option<f32>,
    pub(super) observation: &'a crate::ui::surface::PageFlowObservationToken,
}

struct PageBlockRowRenderState {
    block_id: String,
    selected_without_drag: bool,
    show_controls: bool,
    numbered_index: usize,
    show_empty_toggle_placeholder: bool,
}

impl PageBlockRenderer {
    pub(super) fn render_page_block(
        &self,
        context: PageBlockRenderContext<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let state = self.page_block_row_render_state(&context);
        let content = self.render_page_block_body(&context, &state, cx);
        self.render_page_block_frame(&context, &state, content, cx)
    }

    fn page_block_row_render_state(
        &self,
        context: &PageBlockRenderContext<'_>,
    ) -> PageBlockRowRenderState {
        let block_id = context.block.block_id.clone();
        let selected_without_drag = self.interaction.block_selected_without_drag(&block_id);
        PageBlockRowRenderState {
            show_controls: self.interaction.hovered_block.as_deref() == Some(block_id.as_str())
                || self.interaction.block_menu_is_open(&block_id),
            numbered_index: context.data.numbered_indices[context.index],
            show_empty_toggle_placeholder: self.page_block_shows_empty_toggle(context),
            block_id,
            selected_without_drag,
        }
    }

    fn page_block_shows_empty_toggle(&self, context: &PageBlockRenderContext<'_>) -> bool {
        context
            .data
            .block_shows_empty_toggle_placeholder(&context.block.block_id)
    }

    fn render_page_block_body(
        &self,
        context: &PageBlockRenderContext<'_>,
        state: &PageBlockRowRenderState,
        cx: &mut App,
    ) -> Div {
        div()
            .relative()
            .ml(px(PAGE_BLOCK_GUTTER_WIDTH))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .when(
                state.selected_without_drag
                    && context.block.editable_content().is_none_or(|editable| {
                        editable.kind == super::super::CardPageBlockKind::PageLink
                    }),
                |body| body.bg(alpha(0x2383e2, 0.14)),
            )
            .child(self.render_page_block_content(context, state.numbered_index, cx))
            .when(state.show_empty_toggle_placeholder, |content| {
                content.child(self.render_empty_page_toggle_placeholder(
                    &state.block_id,
                    state.selected_without_drag,
                    cx,
                ))
            })
            .when(
                self.interaction
                    .slash_menu
                    .as_ref()
                    .is_some_and(|menu| menu.block_id == context.block.block_id),
                |body| {
                    body.child(self.render_native_page_slash_menu(
                        context.block,
                        context.data.page.format,
                        cx,
                    ))
                },
            )
            .when(
                self.interaction.mention_menu_block.as_deref()
                    == Some(context.block.block_id.as_str()),
                |body| body.child(self.render_native_page_mention_menu(context.block, cx)),
            )
            .when(
                self.interaction
                    .block_actions_menu_is_open(&context.block.block_id),
                |body| {
                    body.child(self.render_page_block_context_menu(
                        context.block,
                        context.first_line_center(),
                        cx,
                    ))
                },
            )
    }

    fn render_page_block_frame(
        &self,
        context: &PageBlockRenderContext<'_>,
        state: &PageBlockRowRenderState,
        content: Div,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(ElementId::Name(
                format!("notion-page-block-{}", state.block_id).into(),
            ))
            .relative()
            .flex()
            .top(px(-context.nesting_offset))
            .ml(px(
                context.render_depth as f32 * PAGE_BLOCK_INDENT - PAGE_BLOCK_GUTTER_WIDTH
            ))
            .when(context.render_gutter, |row| {
                row.child(self.render_page_block_gutter(context, state.show_controls, cx))
            })
            .child(content)
            .into_any_element()
    }

    pub(super) fn render_page_block_gutter(
        &self,
        context: &PageBlockRenderContext<'_>,
        visible: bool,
        cx: &mut App,
    ) -> Div {
        let gutter_center = context
            .gutter_center
            .unwrap_or_else(|| context.first_line_center());
        let mut gutter = div()
            .absolute()
            .left(px(0.0))
            .top(px(gutter_center - PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT / 2.0))
            .h(px(PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT))
            .w(px(PAGE_BLOCK_GUTTER_WIDTH))
            .flex()
            .items_center()
            .justify_end();
        if visible {
            gutter = gutter
                .child(self.render_page_block_add_button(context.block, cx))
                .child(self.render_page_block_drag_handle(context, cx));
        }
        gutter
    }

    fn render_page_block_add_button(&self, block: &CardPageBlock, cx: &mut App) -> AnyElement {
        let block_id = block.block_id.clone();
        let key_block_id = block_id.clone();
        div()
            .id(ElementId::Name(
                format!("notion-block-add-{block_id}").into(),
            ))
            .size(px(24.0))
            .role(Role::Button)
            .aria_label("Click to add below. Option-click to add a block above")
            .focusable()
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    PageRenderAction::Block(PageBlockRenderAction::Edit(
                        PageBlockEditRenderAction::InsertRelative {
                            block_id: block_id.clone(),
                            before: event.modifiers.alt,
                        },
                    ))
                }),
            )
            .on_key_down({
                let actions = self.actions.clone();
                move |event: &KeyDownEvent, window: &mut gpui::Window, cx: &mut App| {
                    if event.keystroke.modifiers.modified()
                        || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    actions.emit(
                        PageRenderAction::Block(PageBlockRenderAction::Edit(
                            PageBlockEditRenderAction::InsertRelative {
                                block_id: key_block_id.clone(),
                                before: false,
                            },
                        )),
                        window,
                        cx,
                    );
                }
            })
            .child(img(self.icons.page_block_add.render(cx)).size(px(20.0)))
            .into_any_element()
    }

    fn render_page_block_drag_handle(
        &self,
        context: &PageBlockRenderContext<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let block_id = context.block.block_id.clone();
        let drag_enabled = !self.interaction.block_is_composing(&context.block.block_id);
        let handle = self.render_page_block_drag_handle_frame(&block_id, drag_enabled, cx);
        self.wire_page_block_drag_handle(handle, context, cx)
            .on_mouse_up(MouseButton::Left, {
                let actions = self.actions.clone();
                let block_id = block_id.clone();
                move |_: &MouseUpEvent, window: &mut gpui::Window, cx: &mut App| {
                    if !cx.has_active_drag() {
                        cx.stop_propagation();
                        actions.emit(
                            PageRenderAction::Drag(PageDragRenderAction::FinishBlockHandle(
                                block_id.clone(),
                            )),
                            window,
                            cx,
                        );
                    }
                }
            })
            .on_mouse_up(MouseButton::Right, {
                let actions = self.actions.clone();
                move |_: &MouseUpEvent, window: &mut gpui::Window, cx: &mut App| {
                    cx.stop_propagation();
                    actions.emit(
                        PageRenderAction::Block(PageBlockRenderAction::ToggleContextMenu(
                            block_id.clone(),
                        )),
                        window,
                        cx,
                    );
                }
            })
            .child(render_six_dot_handle(
                self.icons.page_block_handle.render(cx),
            ))
            .into_any_element()
    }

    fn wire_page_block_drag_handle(
        &self,
        handle: gpui::Stateful<Div>,
        context: &PageBlockRenderContext<'_>,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        if self.interaction.block_is_composing(&context.block.block_id) {
            return handle;
        }
        let payload = self.page_block_drag_payload(context, cx);
        let drag_data = context.data.clone();
        let drop_runtime =
            PageBlockDropRuntime::new(self.drag.clone(), self.flow.clone(), self.notifier.clone());
        let scroll_target = context.scroll_target;
        handle
            .cursor_grab()
            .on_drag(payload, move |dragged, cursor_offset, window, cx| {
                drop_runtime.can_drop(
                    PageBlockDropRequest {
                        scroll_target,
                        data: &drag_data,
                        pointer: window.mouse_position(),
                        payload: dragged,
                    },
                    cx,
                );
                page_block_drag_preview(dragged, cursor_offset, window, cx)
            })
    }

    fn render_page_block_drag_handle_frame(
        &self,
        block_id: &str,
        drag_enabled: bool,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let key_block_id = block_id.to_string();
        div()
            .id(ElementId::Name(
                format!("notion-block-drag-{block_id}").into(),
            ))
            .w(px(18.0))
            .h(px(24.0))
            .role(Role::Button)
            .aria_label(if drag_enabled {
                "Drag to move, click to open menu"
            } else {
                "Click to open menu"
            })
            .focusable()
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    PageRenderAction::Drag(PageDragRenderAction::BeginBlockHandle)
                }),
            )
            .on_key_down({
                let actions = self.actions.clone();
                move |event: &KeyDownEvent, window: &mut gpui::Window, cx: &mut App| {
                    if event.keystroke.modifiers.modified()
                        || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    actions.emit(
                        PageRenderAction::Block(PageBlockRenderAction::ToggleContextMenu(
                            key_block_id.clone(),
                        )),
                        window,
                        cx,
                    );
                }
            })
    }
}
