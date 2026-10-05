use gpui::prelude::FluentBuilder;
use gpui::App;
use gpui::{
    anchored, deferred, Anchor, AnyElement, Div, ElementId, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Role, Stateful,
    StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::{
    PageLinkIconAction, PageLinkIconPickerAction, PageLinkIconTarget, PageLinkIconUploadAction,
};
use super::menus::PageLinkNamedIconChoiceMenu;
use crate::ui::board_workspace::{
    div,
    page::editor::{
        context_menu::page_block_menu_shadow, PageLinkIconPickerControlMenu,
        PageLinkIconPickerState, PageLinkIconPickerTab, PageLinkIconView,
    },
    px, rgb,
};

impl PageLinkIconView {
    pub(in crate::ui::board_workspace::page::editor) fn render_page_link_icon_picker(
        &self,
        state: &PageLinkIconPickerState,
        page_id: &str,
        block_id: &str,
        cx: &mut App,
    ) -> AnyElement {
        let click_away = ClickAwayBoundary::new();
        let panel = self.render_page_link_icon_picker_body(state, page_id, block_id, cx);
        let panel = self.render_page_link_icon_picker_control_layers(panel, state, &click_away, cx);
        self.render_page_link_icon_picker_anchor(
            panel,
            &click_away,
            state.tab == PageLinkIconPickerTab::Upload,
            cx,
        )
    }

    fn render_page_link_icon_picker_body(
        &self,
        state: &PageLinkIconPickerState,
        page_id: &str,
        block_id: &str,
        cx: &mut App,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-page-link-icon-picker-{block_id}").into(),
            ))
            .role(Role::Dialog)
            .aria_label("Page icon")
            .w(px(408.0))
            .h(px(page_link_icon_picker_panel_height(state)))
            .relative()
            .p(px(0.0))
            .rounded(px(10.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(page_block_menu_shadow(self.appearance_mode))
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                self.listener(|_, _: &MouseDownEvent, _, cx| cx.stop_propagation()),
            )
            .on_scroll_wheel(
                self.listener(|_, _: &gpui::ScrollWheelEvent, _, cx| cx.stop_propagation()),
            )
            .child(self.render_page_link_icon_picker_tabs(state))
            .when(state.tab != PageLinkIconPickerTab::Upload, |panel| {
                panel.child(self.render_page_link_icon_picker_search(state, cx))
            })
            .child(self.render_page_link_icon_picker_tab_content(state, page_id, block_id, cx))
    }

    fn render_page_link_icon_picker_tab_content(
        &self,
        state: &PageLinkIconPickerState,
        page_id: &str,
        block_id: &str,
        cx: &mut App,
    ) -> AnyElement {
        match state.tab {
            PageLinkIconPickerTab::Emoji => {
                self.render_page_link_emoji_grid(state, page_id, block_id, cx)
            }
            PageLinkIconPickerTab::Icons => {
                self.render_page_link_named_icon_grid(state, page_id, block_id)
            }
            PageLinkIconPickerTab::Upload => self.render_page_link_icon_upload_tab(state),
        }
    }

    // These menus must remain the panel's final children. Nested deferred
    // subtrees break GPUI reuse when virtualized named-icon rows are recycled.
    // The picker state's page and block ids match the rendered icon's ids.
    fn render_page_link_icon_picker_control_layers(
        &self,
        panel: Stateful<Div>,
        state: &PageLinkIconPickerState,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Stateful<Div> {
        panel
            .when(
                state.control_menu == Some(PageLinkIconPickerControlMenu::SkinTone),
                |panel| {
                    panel.child(self.render_page_link_icon_picker_skin_tone_menu(click_away, cx))
                },
            )
            .when(
                state.control_menu == Some(PageLinkIconPickerControlMenu::NamedIconColor),
                |panel| {
                    panel.child(self.render_page_link_icon_picker_named_color_menu(
                        state.named_icon_preference,
                        click_away,
                        cx,
                    ))
                },
            )
            .when_some(
                page_link_named_icon_choice(state),
                |panel, (slug, anchor)| {
                    panel.child(self.render_page_link_named_icon_choice_menu(
                        PageLinkNamedIconChoiceMenu {
                            target: PageLinkIconTarget {
                                page_id: state.page_id.clone(),
                                block_id: state.block_id.clone(),
                            },
                            slug,
                        },
                        anchor,
                        click_away,
                        cx,
                    ))
                },
            )
    }

    // The zero-sized anchor is four pixels above the icon's bottom center;
    // `Anchored` handles overflow flipping and edge snapping from there.
    fn render_page_link_icon_picker_anchor(
        &self,
        panel: Stateful<Div>,
        click_away: &ClickAwayBoundary,
        upload_tab_is_open: bool,
        cx: &mut App,
    ) -> AnyElement {
        let actions = self.actions.clone();
        let panel = click_away.dismissible_with_handler(
            div().child(panel),
            move |_, window, cx| {
                actions.emit(PageLinkIconAction::DismissPageBlockInteraction, window, cx);
            },
            cx,
        );
        div()
            .absolute()
            .left(px(12.0))
            .top(px(-4.0))
            .size_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                self.listener(|_, _: &MouseDownEvent, _, cx| cx.stop_propagation()),
            )
            .on_key_down(
                self.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key.as_str() == "v"
                        && event.keystroke.modifiers.platform
                        && upload_tab_is_open
                    {
                        window.prevent_default();
                        cx.stop_propagation();
                        this.emit(
                            PageLinkIconAction::Upload(PageLinkIconUploadAction::PasteUpload),
                            window,
                            cx,
                        );
                        return;
                    }
                    if event.keystroke.key.as_str() != "escape" {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Picker(PageLinkIconPickerAction::DismissLayer),
                        window,
                        cx,
                    );
                }),
            )
            .child(
                deferred(anchored().anchor(Anchor::BottomCenter).child(panel)).with_priority(121),
            )
            .into_any_element()
    }
}

fn page_link_icon_picker_panel_height(state: &PageLinkIconPickerState) -> f32 {
    match state.tab {
        PageLinkIconPickerTab::Upload if state.upload_add_to_library => 359.0,
        PageLinkIconPickerTab::Upload if state.upload_preview.is_some() || state.upload_pending => {
            287.0
        }
        PageLinkIconPickerTab::Upload => 192.0,
        PageLinkIconPickerTab::Emoji | PageLinkIconPickerTab::Icons => 390.0,
    }
}

type PageLinkNamedIconChoiceAnchor = (crate::ui::NotionNamedIconSlug, gpui::Point<gpui::Pixels>);

fn page_link_named_icon_choice(
    state: &PageLinkIconPickerState,
) -> Option<PageLinkNamedIconChoiceAnchor> {
    match state.control_menu {
        Some(PageLinkIconPickerControlMenu::NamedIconChoice { slug, anchor, .. }) => {
            Some((slug, anchor))
        }
        _ => None,
    }
}
