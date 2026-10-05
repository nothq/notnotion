use gpui::ClickEvent;

use super::super::input::PageBlockInputContent;
use super::super::{
    alpha, div, px, AnyElement, App, CardPageBlock, CardPageBlockColor, CardPageEditableBlock, Div,
    ElementId, FluentBuilder, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::content::{render_page_callout_container_content, PageBlockVisualStyle};
use super::row::PageBlockRenderContext;
use super::{PageBlockRenderAction, PageBlockRenderer, PageRenderAction};
use crate::model::PageShellIcon;

struct PageCalloutIconRender {
    page_id: String,
    block_id: String,
    icon: PageShellIcon,
    icon_label: String,
    picker_open: bool,
}

impl PageBlockRenderer {
    pub(super) fn render_page_callout_block_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> Div {
        assert!(
            context.data.block_has_text_input(&context.block.block_id),
            "container-only Callouts must render through segmented document units"
        );
        let input = self.page_block_input_entity(
            context.data,
            PageBlockInputContent {
                block_id: &context.block.block_id,
                editable,
                color,
            },
            cx,
        );
        let content = div().w_full().px(px(6.0)).py(px(6.0)).child(input);
        let marker = self.render_page_callout_icon(&context.data.page.block_id, context.block, cx);
        render_page_callout_container_content(
            PageBlockVisualStyle::new(self.theme, self.appearance_mode, color).with_selection(
                self.interaction
                    .block_selected_without_drag(&context.block.block_id),
            ),
            content.into_any_element(),
            Some(marker),
            Some(ElementId::Name(
                format!("notion-callout-note-{}", context.block.block_id).into(),
            )),
        )
    }

    pub(super) fn render_page_callout_icon(
        &self,
        page_id: &str,
        block: &CardPageBlock,
        cx: &mut App,
    ) -> AnyElement {
        let spec = self.page_callout_icon_render(page_id, block);
        let button = self
            .wire_page_callout_icon_button(self.page_callout_icon_button_base(&spec), &spec, cx)
            .child(self.page_icons.render(&spec.icon, 21.6, cx));
        div()
            .relative()
            .size(px(24.0))
            .child(button)
            .when(spec.picker_open, |marker| {
                marker.when_some(self.page_link_icons.as_ref(), |marker, picker| {
                    marker.child(picker.render(&spec.page_id, &spec.block_id, cx))
                })
            })
            .into_any_element()
    }

    fn page_callout_icon_button_base(&self, spec: &PageCalloutIconRender) -> gpui::Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-callout-icon-{}", spec.block_id).into(),
            ))
            .size(px(24.0))
            .rounded(px(4.0))
            .role(Role::Button)
            .aria_label(spec.icon_label.clone())
            .focusable()
            .tab_stop(true)
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
    }

    fn wire_page_callout_icon_button(
        &self,
        button: gpui::Stateful<Div>,
        spec: &PageCalloutIconRender,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let click_page_id = spec.page_id.clone();
        let click_block_id = spec.block_id.clone();
        let key_page_id = click_page_id.clone();
        let key_block_id = click_block_id.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        button
            .on_click(move |_: &ClickEvent, window, cx| {
                cx.stop_propagation();
                click_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::TogglePageLinkIcon {
                        page_id: click_page_id.clone(),
                        block_id: click_block_id.clone(),
                    }),
                    window,
                    cx,
                );
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    window.prevent_default();
                    cx.stop_propagation();
                    key_actions.emit(
                        PageRenderAction::Block(PageBlockRenderAction::DismissPageLinkIcon),
                        window,
                        cx,
                    );
                    return;
                }
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    PageRenderAction::Block(PageBlockRenderAction::TogglePageLinkIcon {
                        page_id: key_page_id.clone(),
                        block_id: key_block_id.clone(),
                    }),
                    window,
                    cx,
                );
            })
    }

    fn page_callout_icon_render(
        &self,
        page_id: &str,
        block: &CardPageBlock,
    ) -> PageCalloutIconRender {
        let block_id = block.block_id.clone();
        let icon = block
            .icon
            .clone()
            .unwrap_or_else(|| PageShellIcon::emoji("💡"));
        let icon_label = if icon.kind == "emoji" {
            format!("{} Change callout icon", icon.value)
        } else {
            "Change callout icon".to_string()
        };
        PageCalloutIconRender {
            picker_open: self
                .interaction
                .page_link_icon_picker_is_open(page_id, &block_id),
            page_id: page_id.to_string(),
            block_id,
            icon,
            icon_label,
        }
    }
}
