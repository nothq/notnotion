use super::super::{
    alpha, div, img, page_to_do_border_color, px, rgb, App, CardPageBlockColor, CardPageToDoState,
    Div, ElementId, FluentBuilder, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled,
};
use super::actions::PageBlockEditRenderAction;
use super::{PageBlockRenderAction, PageBlockRenderer, PageRenderAction};

impl PageBlockRenderer {
    pub(super) fn render_page_toggle_disclosure_button(
        &self,
        page_id: &str,
        block_id: &str,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> super::super::AnyElement {
        let expanded = self.interaction.expanded_blocks.contains(block_id);
        let marker = self.icons.page_toggle_marker(expanded, color).render(cx);
        let click_page_id = page_id.to_string();
        let click_block_id = block_id.to_string();
        let key_page_id = click_page_id.clone();
        let key_block_id = click_block_id.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        page_toggle_disclosure_button(block_id, expanded, self.theme.text_primary)
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                click_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::ToggleDisclosure {
                        page_id: click_page_id.clone(),
                        block_id: click_block_id.clone(),
                    }),
                    window,
                    cx,
                );
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::ToggleDisclosure {
                        page_id: key_page_id.clone(),
                        block_id: key_block_id.clone(),
                    }),
                    window,
                    cx,
                );
            })
            .child(super::super::render_page_toggle_marker(marker))
            .into_any_element()
    }

    pub(super) fn render_empty_page_toggle_placeholder(
        &self,
        block_id: &str,
        selected: bool,
        _cx: &mut App,
    ) -> super::super::AnyElement {
        let placeholder = div()
            .id(ElementId::Name(
                format!("notion-empty-toggle-{block_id}").into(),
            ))
            .ml(px(32.0))
            .mr(px(6.0))
            .h(px(40.0))
            .px(px(8.0))
            .role(Role::Button)
            .aria_label("Empty toggle. Click or drop blocks inside.")
            .focusable()
            .tab_stop(true)
            .rounded(px(4.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(16.0))
            .line_height(px(24.0))
            .text_color(alpha(self.theme.text_primary, 0.286))
            .when(selected, |placeholder| {
                placeholder.bg(alpha(0x2383e2, 0.14))
            })
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.04)))
            .focus_visible(|style| style.shadow(page_toggle_disclosure_focus_shadow()));
        self.wire_empty_page_toggle_placeholder(placeholder, block_id)
            .child("Empty toggle. Click or drop blocks inside.")
            .into_any_element()
    }

    fn wire_empty_page_toggle_placeholder(
        &self,
        placeholder: gpui::Stateful<Div>,
        block_id: &str,
    ) -> gpui::Stateful<Div> {
        let click_block_id = block_id.to_string();
        let key_block_id = click_block_id.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        placeholder
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                click_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::Edit(
                        PageBlockEditRenderAction::InsertFirstToggleChild(click_block_id.clone()),
                    )),
                    window,
                    cx,
                );
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::Edit(
                        PageBlockEditRenderAction::InsertFirstToggleChild(key_block_id.clone()),
                    )),
                    window,
                    cx,
                );
            })
    }

    pub(super) fn render_page_to_do_checkbox(
        &self,
        block_id: &str,
        state: CardPageToDoState,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> super::super::AnyElement {
        let checked = state.is_checked();
        let click_block_id = block_id.to_string();
        let key_block_id = click_block_id.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        let checkbox = page_to_do_checkbox_base(block_id, checked);
        let checkbox = self.style_page_to_do_checkbox(checkbox, checked, color, cx);
        checkbox
            .focus_visible(|style| style.shadow(page_toggle_disclosure_focus_shadow()))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                click_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::Edit(
                        PageBlockEditRenderAction::ToggleToDo(click_block_id.clone()),
                    )),
                    window,
                    cx,
                );
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::Edit(
                        PageBlockEditRenderAction::ToggleToDo(key_block_id.clone()),
                    )),
                    window,
                    cx,
                );
            })
            .into_any_element()
    }

    fn style_page_to_do_checkbox(
        &self,
        checkbox: gpui::Stateful<Div>,
        checked: bool,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        checkbox
            .when(checked, |checkbox| {
                checkbox
                    .bg(rgb(0x2783de))
                    .child(img(self.icons.page_checkbox_checked.render(cx)).size(px(14.0)))
            })
            .when(!checked, |checkbox| {
                checkbox
                    .border(px(1.5))
                    .border_color(page_to_do_border_color(
                        color,
                        self.appearance_mode,
                        self.theme.text_secondary,
                    ))
            })
    }
}

fn page_toggle_disclosure_button(
    block_id: &str,
    expanded: bool,
    text_primary: u32,
) -> gpui::Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!("notion-toggle-disclosure-{block_id}").into(),
        ))
        .size(px(24.0))
        .flex_none()
        .role(Role::Button)
        .aria_label(if expanded { "Close" } else { "Open" })
        .aria_expanded(expanded)
        .focusable()
        .tab_stop(true)
        .rounded(px(4.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |style| style.bg(alpha(text_primary, 0.08)))
        .focus_visible(|style| style.shadow(page_toggle_disclosure_focus_shadow()))
}

fn page_to_do_checkbox_base(block_id: &str, checked: bool) -> gpui::Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!("notion-to-do-checkbox-{block_id}").into(),
        ))
        .size(px(16.0))
        .flex_none()
        .role(Role::Button)
        .aria_label(if checked {
            "Mark to-do as unchecked"
        } else {
            "Mark to-do as checked"
        })
        .aria_toggled(checked.into())
        .focusable()
        .tab_stop(true)
        .rounded(px(1.5))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
}

pub(in crate::ui::board_workspace::page::editor) fn page_toggle_disclosure_focus_shadow(
) -> Vec<gpui::BoxShadow> {
    vec![
        gpui::BoxShadow {
            color: rgb(0xf8f8f7).into(),
            offset: gpui::point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(2.0),
            inset: false,
        },
        gpui::BoxShadow {
            color: rgb(0x2383e2).into(),
            offset: gpui::point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(4.0),
            inset: false,
        },
        gpui::BoxShadow {
            color: alpha(0xffffff, 0.25),
            offset: gpui::point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(6.0),
            inset: false,
        },
    ]
}
