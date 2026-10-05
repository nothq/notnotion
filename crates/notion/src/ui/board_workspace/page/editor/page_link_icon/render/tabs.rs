use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};

use super::super::{
    PageLinkIconAction, PageLinkIconPickerAction, PageLinkIconSelectionAction, PageLinkIconTarget,
};
use crate::ui::board_workspace::{
    alpha, div,
    page::editor::{PageLinkIconPickerState, PageLinkIconPickerTab, PageLinkIconView},
    px, rgb,
};

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_picker_tabs(
        &self,
        state: &PageLinkIconPickerState,
    ) -> impl IntoElement {
        let tabs = [
            (PageLinkIconPickerTab::Emoji, "Emoji"),
            (PageLinkIconPickerTab::Icons, "Icons"),
            (PageLinkIconPickerTab::Upload, "Upload"),
        ]
        .into_iter()
        .fold(
            self.page_link_icon_picker_tab_list(),
            |tabs, (tab, label)| {
                tabs.child(self.render_page_link_icon_picker_tab(state, tab, label))
            },
        );
        div()
            .id("notion-page-link-icon-picker-header")
            .h(px(40.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .child(tabs)
            .child(self.render_page_link_icon_picker_remove(state))
    }

    fn page_link_icon_picker_tab_list(&self) -> Stateful<Div> {
        div()
            .id("notion-page-link-icon-picker-tabs")
            .role(Role::TabList)
            .aria_label("Icon type")
            .h(px(40.0))
            .w(px(244.0))
            .flex()
            .items_center()
            .gap(px(0.0))
    }

    fn render_page_link_icon_picker_tab(
        &self,
        state: &PageLinkIconPickerState,
        tab: PageLinkIconPickerTab,
        label: &'static str,
    ) -> Div {
        let selected = state.tab == tab;
        div()
            .relative()
            .h_full()
            .flex()
            .items_center()
            .child(self.render_page_link_icon_picker_tab_button(state, tab, label))
            .when(selected, |tab| {
                tab.child(
                    div()
                        .absolute()
                        .left(px(8.0))
                        .right(px(8.0))
                        .bottom(px(0.0))
                        .h(px(2.0))
                        .rounded(px(1.0))
                        .bg(rgb(self.theme.text_primary)),
                )
            })
    }

    fn render_page_link_icon_picker_tab_button(
        &self,
        state: &PageLinkIconPickerState,
        tab: PageLinkIconPickerTab,
        label: &'static str,
    ) -> Stateful<Div> {
        let selected = state.tab == tab;
        let enabled = !state.upload_committed;
        div()
            .id(ElementId::Name(
                format!("notion-page-link-icon-picker-tab-{label}").into(),
            ))
            .role(Role::Tab)
            .aria_label(label)
            .aria_selected(selected)
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .when(selected, |tab| {
                tab.bg(alpha(self.theme.text_primary, 0.055))
            })
            .text_size(px(14.0))
            .text_color(rgb(if selected {
                self.theme.text_primary
            } else {
                self.theme.text_muted
            }))
            .opacity(if enabled { 1.0 } else { 0.4 })
            .focusable()
            .tab_stop(enabled)
            .when(enabled, |tab| tab.cursor_pointer())
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(MouseButton::Left, self.page_link_icon_tab_click(tab))
            .on_key_down(self.page_link_icon_tab_key_down(tab))
            .child(label)
    }

    fn page_link_icon_tab_click(
        &self,
        tab: PageLinkIconPickerTab,
    ) -> impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, _: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Picker(PageLinkIconPickerAction::SetTab(tab)),
                window,
                cx,
            );
        })
    }

    fn page_link_icon_tab_key_down(
        &self,
        tab: PageLinkIconPickerTab,
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
                PageLinkIconAction::Picker(PageLinkIconPickerAction::SetTab(tab)),
                window,
                cx,
            );
        })
    }

    fn remove_icon_key_down(
        &self,
        key_page_id: String,
        key_block_id: String,
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
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetIcon {
                    target: PageLinkIconTarget {
                        page_id: key_page_id.clone(),
                        block_id: key_block_id.clone(),
                    },
                    icon: None,
                }),
                window,
                cx,
            );
        })
    }

    fn render_page_link_icon_picker_remove(&self, state: &PageLinkIconPickerState) -> AnyElement {
        let enabled = !state.upload_committed;
        let page_id = state.page_id.clone();
        let block_id = state.block_id.clone();
        let key_page_id = page_id.clone();
        let key_block_id = block_id.clone();
        div()
            .id(ElementId::Name(
                format!("notion-page-link-icon-remove-{block_id}").into(),
            ))
            .role(Role::Button)
            .aria_label("Remove icon")
            .focusable()
            .tab_stop(enabled)
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(5.0))
            .flex()
            .items_center()
            .justify_center()
            .opacity(if enabled { 1.0 } else { 0.4 })
            .text_size(px(12.0))
            .text_color(rgb(self.theme.text_muted))
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_click(self.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.emit(
                            PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetIcon {
                                target: PageLinkIconTarget {
                                    page_id: page_id.clone(),
                                    block_id: block_id.clone(),
                                },
                                icon: None,
                            }),
                            window,
                            cx,
                        );
                    }))
                    .on_key_down(self.remove_icon_key_down(key_page_id, key_block_id))
            })
            .child("Remove")
            .into_any_element()
    }
}
