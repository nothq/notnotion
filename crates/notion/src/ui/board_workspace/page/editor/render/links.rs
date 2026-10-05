use gpui::{ClickEvent, Hsla};

use super::super::{
    alpha, div, img, page_block_background, page_block_foreground, px, App, AppearanceMode,
    CardPageBlockColor, CardPageEditableBlock, Div, ElementId, FluentBuilder, FontWeight,
    InteractiveElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::row::PageBlockRenderContext;
use super::{PageBlockRenderAction, PageBlockRenderer, PageDocumentRenderAction, PageRenderAction};
use crate::model::{CardPageAliasBlock, NotionBoardUrl, PageShellIcon};

struct PageAliasLinkRender {
    block_id: String,
    label: String,
    target_url: Option<String>,
    border: Hsla,
    foreground: Hsla,
}

struct PageLinkBlockRender {
    block_id: String,
    page_id: String,
    label: String,
    target_url: Option<String>,
    icon: PageShellIcon,
    picker_open: bool,
    foreground: Hsla,
}

impl PageBlockRenderer {
    pub(super) fn render_page_alias_block_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        alias: &CardPageAliasBlock,
        cx: &mut App,
    ) -> Div {
        let spec = PageAliasLinkRender {
            block_id: context.block.block_id.clone(),
            label: alias.title.clone(),
            target_url: self.notion_child_block_url(&alias.target_block_id),
            border: page_link_border(self.appearance_mode),
            foreground: page_block_foreground(context.block.color, self.appearance_mode),
        };
        div()
            .w_full()
            .relative()
            .p(px(6.0))
            .when_some(
                page_block_background(context.block.color, self.appearance_mode),
                |block, background| block.bg(background),
            )
            .child(self.render_page_alias_link(&spec, alias, cx))
    }

    fn render_page_alias_link(
        &self,
        spec: &PageAliasLinkRender,
        alias: &CardPageAliasBlock,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let link = self.page_alias_link_base(spec);
        self.wire_page_alias_navigation(link, spec, cx)
            .child(self.render_page_alias_icon(alias, cx))
            .child(page_alias_label(&spec.label, spec.foreground, spec.border))
    }

    fn page_alias_link_base(&self, spec: &PageAliasLinkRender) -> gpui::Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-page-alias-{}", spec.block_id).into(),
            ))
            .h(px(28.0))
            .w_full()
            .p(px(2.0))
            .role(Role::Link)
            .aria_label(format!("Open {}", spec.label))
            .focusable()
            .tab_stop(true)
            .rounded(px(4.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.04)))
    }

    fn wire_page_alias_navigation(
        &self,
        link: gpui::Stateful<Div>,
        spec: &PageAliasLinkRender,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let Some(target_url) = spec.target_url.clone() else {
            return link;
        };
        let keyboard_url = target_url.clone();
        let keyboard_label = spec.label.clone();
        let mouse_label = spec.label.clone();
        let mouse_actions = self.actions.clone();
        let keyboard_actions = self.actions.clone();
        link.on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            mouse_actions.emit(
                PageRenderAction::Document(PageDocumentRenderAction::OpenWorkspace {
                    url: target_url.clone(),
                    label: mouse_label.clone(),
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
            keyboard_actions.emit(
                PageRenderAction::Document(PageDocumentRenderAction::OpenWorkspace {
                    url: keyboard_url.clone(),
                    label: keyboard_label.clone(),
                }),
                window,
                cx,
            );
        })
    }

    fn render_page_alias_icon(&self, alias: &CardPageAliasBlock, cx: &mut App) -> Div {
        div()
            .relative()
            .size(px(24.0))
            .mr(px(4.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(self.page_icons.render(&alias.icon, 19.8, cx))
            .child(
                div()
                    .absolute()
                    .right(px(-2.0))
                    .bottom(px(-2.0))
                    .size(px(16.0))
                    .child(img(self.icons.ai_autofill_external_link.render(cx)).size(px(16.0))),
            )
    }

    pub(super) fn render_page_link_block_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> Div {
        let spec = self.page_link_block_render(context, editable, color);
        div()
            .w_full()
            .relative()
            .px(px(6.0))
            .pt(px(6.0))
            .pb(px(0.0))
            .child(self.render_page_link(&spec, cx))
    }

    fn page_link_block_render(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        color: CardPageBlockColor,
    ) -> PageLinkBlockRender {
        let block_id = context.block.block_id.clone();
        let page_id = context.data.page.block_id.clone();
        PageLinkBlockRender {
            label: if editable.text.is_empty() {
                "New page".to_string()
            } else {
                editable.text.clone()
            },
            target_url: self.notion_child_block_url(&block_id),
            icon: context
                .block
                .icon
                .clone()
                .unwrap_or_else(|| PageShellIcon::named("page")),
            picker_open: self
                .interaction
                .page_link_icon_picker_is_open(&page_id, &block_id),
            foreground: page_block_foreground(color, self.appearance_mode),
            block_id,
            page_id,
        }
    }

    fn render_page_link(&self, spec: &PageLinkBlockRender, cx: &mut App) -> gpui::Stateful<Div> {
        let link = div()
            .id(ElementId::Name(
                format!("notion-page-link-{}", spec.block_id).into(),
            ))
            .role(Role::Link)
            .aria_label(format!("Open {}", spec.label))
            .focusable()
            .tab_stop(true)
            .cursor_pointer();
        self.wire_page_link_navigation(link, spec, cx)
            .h(px(28.0))
            .w_full()
            .p(px(2.0))
            .flex()
            .items_center()
            .child(self.render_page_link_icon_control(spec, cx))
            .child(page_link_block_label(&spec.label, spec.foreground))
    }

    fn wire_page_link_navigation(
        &self,
        link: gpui::Stateful<Div>,
        spec: &PageLinkBlockRender,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let Some(target_url) = spec.target_url.clone() else {
            return link;
        };
        let keyboard_url = target_url.clone();
        let keyboard_label = spec.label.clone();
        let mouse_label = spec.label.clone();
        let mouse_actions = self.actions.clone();
        let keyboard_actions = self.actions.clone();
        link.on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            mouse_actions.emit(
                PageRenderAction::Document(PageDocumentRenderAction::OpenWorkspace {
                    url: target_url.clone(),
                    label: mouse_label.clone(),
                }),
                window,
                cx,
            );
        })
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if !matches!(event.keystroke.key.as_str(), "enter" | "space")
                || event.keystroke.modifiers.modified()
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            keyboard_actions.emit(
                PageRenderAction::Document(PageDocumentRenderAction::OpenWorkspace {
                    url: keyboard_url.clone(),
                    label: keyboard_label.clone(),
                }),
                window,
                cx,
            );
        })
    }

    fn render_page_link_icon_control(
        &self,
        spec: &PageLinkBlockRender,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let control = div()
            .id(ElementId::Name(
                format!("notion-page-link-icon-{}", spec.block_id).into(),
            ))
            .relative()
            .size(px(24.0))
            .mr(px(4.0))
            .role(Role::Button)
            .aria_label("Change page icon")
            .focusable()
            .tab_stop(true)
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)));
        self.wire_page_link_icon_control(control, spec, cx)
            .child(self.page_icons.render(&spec.icon, 19.8, cx))
            .when(spec.picker_open, |icon| {
                icon.when_some(self.page_link_icons.as_ref(), |icon, picker| {
                    icon.child(picker.render(&spec.page_id, &spec.block_id, cx))
                })
            })
    }

    fn wire_page_link_icon_control(
        &self,
        control: gpui::Stateful<Div>,
        spec: &PageLinkBlockRender,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let click_page_id = spec.page_id.clone();
        let click_block_id = spec.block_id.clone();
        let key_page_id = click_page_id.clone();
        let key_block_id = click_block_id.clone();
        let click_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        control
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

    fn notion_child_block_url(&self, block_id: &str) -> Option<String> {
        self.board_url
            .as_deref()
            .and_then(|board_url| board_url.parse::<NotionBoardUrl>().ok())
            .map(|board_url| board_url.child_block_url(block_id))
    }
}

fn page_alias_label(label: &str, foreground: Hsla, border: Hsla) -> Div {
    div()
        .min_w(px(0.0))
        .max_w_full()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(16.0))
        .font_weight(FontWeight::MEDIUM)
        .line_height(gpui::relative(1.3))
        .border_b_1()
        .border_color(border)
        .text_color(foreground)
        .child(label.to_string())
}

fn page_link_block_label(label: &str, foreground: Hsla) -> Div {
    div()
        .min_w(px(0.0))
        .flex_grow(1.0)
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(16.0))
        .font_weight(FontWeight::MEDIUM)
        .line_height(gpui::relative(1.3))
        .pb(px(1.0))
        .text_color(foreground)
        .child(label.to_string())
}

fn page_link_border(appearance_mode: AppearanceMode) -> Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x1c1301, 0.11),
        AppearanceMode::Dark => alpha(0xffffeb, 0.10),
    }
}
