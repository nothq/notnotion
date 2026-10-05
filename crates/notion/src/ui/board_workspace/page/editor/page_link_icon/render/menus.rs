use gpui::App;
mod buttons;
mod choice;

pub(super) use choice::PageLinkNamedIconChoiceMenu;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::{PageLinkIconAction, PageLinkIconControlAction};
use crate::ui::{
    board_workspace::{
        alpha, div,
        page::editor::{
            context_menu::page_block_menu_shadow, PageLinkIconView, PageLinkNamedIconPreference,
        },
        px, rgb,
    },
    NotionNamedIconColor,
};

use super::catalog::{skin_tone_glyph, skin_tone_label, skin_tones};

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_picker_skin_tone_menu(
        &self,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> AnyElement {
        click_away
            .member(
                div()
                    .absolute()
                    .right(px(8.0))
                    .top(px(76.0))
                    .w(px(200.0))
                    .h(px(40.0))
                    .p(px(4.0))
                    .rounded(px(10.0))
                    .bg(rgb(self.theme.elevated_surface_bg))
                    .shadow(page_block_menu_shadow(self.appearance_mode)),
                cx,
            )
            .id("notion-page-link-icon-picker-skin-tone-menu")
            .role(Role::Menu)
            .aria_label("Emoji skin tone")
            .flex()
            .items_center()
            .gap(px(0.0))
            .children(
                skin_tones()
                    .into_iter()
                    .map(|skin_tone| self.render_page_link_skin_tone_item(skin_tone)),
            )
            .into_any_element()
    }

    fn render_page_link_skin_tone_item(&self, skin_tone: emojis::SkinTone) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-page-link-icon-picker-skin-tone-{skin_tone:?}").into(),
            ))
            .role(Role::MenuItem)
            .aria_label(skin_tone_label(skin_tone))
            .focusable()
            .tab_stop(true)
            .size(px(32.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .text_size(px(20.0))
            .on_click(self.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Controls(PageLinkIconControlAction::SetSkinTone(skin_tone)),
                    window,
                    cx,
                );
            }))
            .on_key_down(
                self.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified()
                        || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Controls(PageLinkIconControlAction::SetSkinTone(
                            skin_tone,
                        )),
                        window,
                        cx,
                    );
                }),
            )
            .child(skin_tone_glyph(skin_tone))
    }

    pub(super) fn render_page_link_icon_picker_named_color_menu(
        &self,
        preference: PageLinkNamedIconPreference,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> AnyElement {
        let ask_every_time = preference == PageLinkNamedIconPreference::AskEveryTime;
        let selected_color = match preference {
            PageLinkNamedIconPreference::AskEveryTime => None,
            PageLinkNamedIconPreference::Color(color) => Some(color),
        };
        click_away
            .member(
                div()
                    .absolute()
                    .right(px(-120.0))
                    .top(px(76.0))
                    .w(px(156.0))
                    .h(px(101.0))
                    .py(px(4.0))
                    .rounded(px(10.0))
                    .bg(rgb(self.theme.elevated_surface_bg))
                    .shadow(page_block_menu_shadow(self.appearance_mode)),
                cx,
            )
            .id("notion-page-link-icon-picker-named-color-menu")
            .role(Role::Dialog)
            .aria_label("Icon color preference")
            .child(self.render_page_link_named_color_rows(ask_every_time, selected_color))
            .child(self.render_page_link_ask_for_color_toggle(ask_every_time))
            .into_any_element()
    }

    fn render_page_link_named_color_rows(
        &self,
        ask_every_time: bool,
        selected_color: Option<NotionNamedIconColor>,
    ) -> Div {
        NotionNamedIconColor::ALL.chunks(5).enumerate().fold(
            div().flex().flex_col(),
            |rows, (row_index, colors)| {
                rows.child(
                    div()
                        .id(ElementId::Name(
                            format!("notion-page-link-icon-picker-named-color-row-{row_index}")
                                .into(),
                        ))
                        .role(Role::Row)
                        .h(px(30.0))
                        .p(px(4.0))
                        .flex()
                        .justify_center()
                        .gap(px(4.0))
                        .opacity(if ask_every_time { 0.5 } else { 1.0 })
                        .children(colors.iter().copied().map(|color| {
                            self.render_page_link_named_color_swatch(
                                color,
                                selected_color == Some(color),
                                !ask_every_time,
                            )
                        })),
                )
            },
        )
    }

    fn render_page_link_ask_for_color_toggle(&self, ask_every_time: bool) -> Stateful<Div> {
        div()
            .id("notion-page-link-icon-picker-ask-color")
            .role(Role::GridCell)
            .aria_label("Ask every time")
            .focusable()
            .tab_stop(true)
            .mx(px(4.0))
            .mt(px(4.0))
            .h(px(25.0))
            .border_t_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .pt(px(4.0))
            .px(px(6.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
            .text_size(px(12.0))
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(MouseButton::Left, self.ask_for_color_click())
            .on_key_down(self.ask_for_color_key_down())
            .child("Ask every time")
            .child(self.render_page_link_ask_for_color_switch(ask_every_time))
    }

    fn render_page_link_ask_for_color_switch(&self, ask_every_time: bool) -> Div {
        div()
            .w(px(30.0))
            .h(px(18.0))
            .p(px(2.0))
            .rounded(px(9.0))
            .bg(if ask_every_time {
                rgb(0x2383e2).into()
            } else {
                alpha(self.theme.text_primary, 0.18)
            })
            .flex()
            .items_center()
            .when(ask_every_time, |toggle| toggle.justify_end())
            .child(div().size(px(14.0)).rounded(px(7.0)).bg(rgb(0xffffff)))
    }

    fn ask_for_color_click(
        &self,
    ) -> impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, _: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleAskForNamedColor),
                window,
                cx,
            );
        })
    }

    fn ask_for_color_key_down(
        &self,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleAskForNamedColor),
                window,
                cx,
            );
        })
    }

    fn named_color_key_down(
        &self,
        color: NotionNamedIconColor,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::SetNamedColor(color)),
                window,
                cx,
            );
        })
    }

    fn render_page_link_named_color_swatch(
        &self,
        color: NotionNamedIconColor,
        selected: bool,
        enabled: bool,
    ) -> AnyElement {
        div()
            .id(ElementId::Name(
                format!("notion-page-link-named-color-{}", color.suffix()).into(),
            ))
            .role(Role::GridCell)
            .aria_label(format!("Set color as {}", color.label()))
            .size(px(22.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .when(selected, |cell| {
                cell.bg(alpha(self.theme.text_primary, 0.08))
            })
            .when(enabled, |cell| {
                cell.focusable()
                    .tab_stop(true)
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_click(self.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.emit(
                            PageLinkIconAction::Controls(PageLinkIconControlAction::SetNamedColor(
                                color,
                            )),
                            window,
                            cx,
                        );
                    }))
                    .on_key_down(self.named_color_key_down(color))
            })
            .child(
                div()
                    .size(px(20.0))
                    .rounded(px(10.0))
                    .bg(rgb(color.picker_color(self.appearance_mode))),
            )
            .into_any_element()
    }
}
