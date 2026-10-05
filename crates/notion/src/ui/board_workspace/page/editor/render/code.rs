use super::actions::PageBlockEditRenderAction;
use gpui::{anchored, deferred, Anchor, BoxShadow, ClickEvent, FocusHandle, Stateful};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::input::PageBlockInputContent;
use super::super::support::PageBlockRowSpacing;
use super::super::{
    alpha, div, img, px, rgb, rgba, AnyElement, App, AppearanceMode, CardPageBlock,
    CardPageBlockColor, CardPageEditableBlock, Div, ElementId, FluentBuilder, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    PageBlockContextMenuPresentation, ParentElement, Role, StatefulInteractiveElement, Styled,
    PAGE_CODE_LANGUAGE_OPTION_STRIDE,
};
use super::{
    PageBlockRenderAction, PageBlockRenderContext, PageBlockRenderer, PageBlockVisualStyle,
    PageRenderAction,
};

struct PageCodeActionBarSpec {
    block_id: String,
    language: String,
    top: f32,
}

struct PageCodeLanguagePickerAnchor {
    focus_handle: FocusHandle,
    anchor: Anchor,
    offset_y: f32,
}

impl PageBlockRenderer {
    pub(super) fn render_page_code_block(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> Div {
        let spacing = context
            .row_spacing
            .expect("Code rows require precomputed spacing");
        let input = self.page_block_input_entity(
            context.data,
            PageBlockInputContent {
                block_id: &context.block.block_id,
                editable,
                color,
            },
            cx,
        );
        let selected = self
            .interaction
            .block_selected_without_drag(&context.block.block_id);
        let body = self.with_mermaid_diagram(input, &context.block.block_id, editable, cx);
        let content = render_page_code_block_content(
            PageBlockVisualStyle::new(self.theme, self.appearance_mode, color)
                .with_selection(selected),
            spacing,
            body,
        );
        if !self.page_code_actions_visible(&context.block.block_id) {
            return content;
        }
        div()
            .relative()
            .w_full()
            .child(content)
            .child(self.render_page_code_action_bar(context.block, editable, spacing, cx))
    }

    fn render_page_code_action_bar(
        &self,
        block: &CardPageBlock,
        editable: &CardPageEditableBlock,
        spacing: PageBlockRowSpacing,
        cx: &mut App,
    ) -> AnyElement {
        let spec = PageCodeActionBarSpec {
            block_id: block.block_id.clone(),
            language: editable
                .code_language()
                .expect("Code blocks must retain a typed language")
                .as_str()
                .to_string(),
            top: spacing.top + 4.0,
        };
        let picker_anchor = self.page_code_language_picker_anchor(block, cx);
        self.render_page_code_action_surface(&spec, cx)
            .when_some(picker_anchor, |actions, anchor| {
                actions.child(self.render_page_code_language_picker(block, anchor, cx))
            })
            .into_any_element()
    }

    fn render_page_code_action_surface(
        &self,
        spec: &PageCodeActionBarSpec,
        cx: &mut App,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-code-actions-{}", spec.block_id).into(),
            ))
            .absolute()
            .top(px(spec.top))
            .right(px(12.0))
            .h(px(28.0))
            .p(px(2.0))
            .rounded(px(6.0))
            .occlude()
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(page_code_action_shadow())
            .flex()
            .items_center()
            .child(self.render_page_code_language_control(spec, cx))
            .child(
                div()
                    .mx(px(4.0))
                    .w(px(1.0))
                    .h(px(16.0))
                    .bg(rgba(self.theme.surface_border)),
            )
            .child(self.render_page_code_copy_control(&spec.block_id, cx))
            .child(self.render_page_code_more_control(&spec.block_id, cx))
    }

    fn render_page_code_language_control(
        &self,
        spec: &PageCodeActionBarSpec,
        cx: &mut App,
    ) -> Stateful<Div> {
        let block_id = spec.block_id.clone();
        let actions = self.actions.clone();
        div()
            .id(ElementId::Name(
                format!("notion-code-language-{}", spec.block_id).into(),
            ))
            .h(px(24.0))
            .px(px(6.0))
            .role(Role::Button)
            .aria_label("Open language dropdown")
            .rounded(px(4.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .text_size(px(12.0))
            .line_height(px(16.0))
            .text_color(rgb(self.theme.text_secondary))
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_click(move |_: &ClickEvent, window, cx| {
                cx.stop_propagation();
                actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::ToggleCodeLanguagePicker(
                        block_id.clone(),
                    )),
                    window,
                    cx,
                );
            })
            .child(spec.language.clone())
            .child(
                img(self.icons.code_action_language_chevron.render(cx))
                    .w(px(9.875))
                    .h(px(16.0)),
            )
    }

    fn render_page_code_copy_control(&self, block_id: &str, cx: &mut App) -> Stateful<Div> {
        let clicked_block_id = block_id.to_string();
        let actions = self.actions.clone();
        page_code_icon_control(
            format!("notion-code-copy-{block_id}"),
            "Copy code to clipboard",
            self.icons.code_action_copy.render(cx),
            alpha(self.theme.text_primary, 0.08),
        )
        .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            actions.emit(
                PageRenderAction::Block(PageBlockRenderAction::Edit(
                    PageBlockEditRenderAction::CopyCode(clicked_block_id.clone()),
                )),
                window,
                cx,
            );
        })
    }

    fn render_page_code_more_control(&self, block_id: &str, cx: &mut App) -> Stateful<Div> {
        let clicked_block_id = block_id.to_string();
        let actions = self.actions.clone();
        page_code_icon_control(
            format!("notion-code-more-{block_id}"),
            "Open block actions menu",
            self.icons.card_action_ellipsis.render(cx),
            alpha(self.theme.text_primary, 0.08),
        )
        .on_click(move |_: &ClickEvent, window, cx| {
            cx.stop_propagation();
            actions.emit(
                PageRenderAction::Block(PageBlockRenderAction::ToggleContextMenu(
                    clicked_block_id.clone(),
                )),
                window,
                cx,
            );
        })
    }

    fn page_code_language_picker_anchor(
        &self,
        block: &CardPageBlock,
        cx: &mut App,
    ) -> Option<PageCodeLanguagePickerAnchor> {
        let menu = self.interaction.context_menu.as_ref().filter(|menu| {
            menu.block_id == block.block_id
                && menu.presentation == PageBlockContextMenuPresentation::CodeLanguagePicker
        })?;
        let menu_height = (48.0
            + menu.code_language_indices.len().max(1) as f32 * PAGE_CODE_LANGUAGE_OPTION_STRIDE
            - 1.0)
            .min(self.viewport.app_height() * 0.50)
            .max(76.0);
        let open_upward = self.page_code_picker_opens_upward(block, menu_height, cx);
        Some(PageCodeLanguagePickerAnchor {
            focus_handle: menu.focus_handle.clone(),
            anchor: if open_upward {
                Anchor::BottomLeft
            } else {
                Anchor::TopLeft
            },
            offset_y: if open_upward { -6.0 } else { 34.0 },
        })
    }

    fn page_code_picker_opens_upward(
        &self,
        block: &CardPageBlock,
        menu_height: f32,
        cx: &mut App,
    ) -> bool {
        self.input_resources
            .state()
            .block_inputs
            .borrow()
            .get(&block.block_id)
            .and_then(|input| input.read(cx).last_window_bounds())
            .is_some_and(|bounds| {
                let button_top = bounds.top().as_f32() - 18.0;
                let button_bottom = bounds.top().as_f32() + 6.0;
                let below = (self.viewport.app_height() - button_bottom).max(0.0);
                let above = (button_top - self.viewport.chrome_top_inset()).max(0.0);
                below < menu_height + 8.0 && above > below
            })
    }

    fn render_page_code_language_picker(
        &self,
        block: &CardPageBlock,
        anchor: PageCodeLanguagePickerAnchor,
        cx: &mut App,
    ) -> AnyElement {
        let Some(menu_renderer) = self.context_menu.as_ref() else {
            return div().into_any_element();
        };
        let picker = menu_renderer.render_code_language(block, cx);
        let dismiss_actions = self.actions.clone();
        let picker = ClickAwayBoundary::new().dismissible_with_handler(
            div().child(picker),
            move |_, window, cx| {
                dismiss_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::DismissInteraction),
                    window,
                    cx,
                );
            },
            cx,
        );
        let keyboard_menu = self.context_menu.clone();
        div()
            .absolute()
            .left(px(2.0))
            .top(px(anchor.offset_y))
            .size_0()
            .occlude()
            .track_focus(&anchor.focus_handle)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if keyboard_menu
                    .as_ref()
                    .is_some_and(|menu| menu.handle_key_down(event, window, cx))
                {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(deferred(anchored().anchor(anchor.anchor).child(picker)).with_priority(122))
            .into_any_element()
    }
    fn page_code_actions_visible(&self, block_id: &str) -> bool {
        self.interaction.hovered_block.as_deref() == Some(block_id)
            || self.interaction.block_menu_is_open(block_id)
    }
}

pub(super) fn render_page_code_block_content(
    visual: PageBlockVisualStyle,
    spacing: PageBlockRowSpacing,
    body: AnyElement,
) -> Div {
    let background = match visual.appearance_mode {
        AppearanceMode::Light => alpha(0x878378, 0.15),
        AppearanceMode::Dark => alpha(0xffffff, 0.055),
    };
    div()
        .relative()
        .w_full()
        .px(px(8.0))
        .pt(px(spacing.top))
        .pb(px(spacing.bottom))
        .child(
            div()
                .absolute()
                .left(px(8.0))
                .right(px(8.0))
                .top(px(spacing.top))
                .bottom(px(spacing.bottom))
                .rounded(px(10.0))
                .bg(background),
        )
        .when(visual.selected, |block| {
            block.child(
                div()
                    .absolute()
                    .inset(px(2.0))
                    .rounded(px(16.0))
                    .bg(alpha(0x2383e2, 0.14)),
            )
        })
        .child(
            div()
                .relative()
                .w_full()
                .rounded(px(10.0))
                .px(px(22.0))
                .py(px(24.0))
                .child(body),
        )
}

fn page_code_icon_control(
    id: String,
    label: &'static str,
    icon: std::sync::Arc<gpui::RenderImage>,
    hover_color: Hsla,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(id.into()))
        .size(px(24.0))
        .role(Role::Button)
        .aria_label(label)
        .rounded(px(4.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |style| style.bg(hover_color))
        .child(img(icon).size(px(16.0)))
}

fn page_code_action_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: gpui::point(px(0.0), px(8.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: gpui::point(px(0.0), px(2.0)),
            blur_radius: px(6.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x2a1c00, 0.07),
            offset: gpui::point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}
