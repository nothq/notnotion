use gpui::prelude::FluentBuilder;
use gpui::App;
use gpui::{
    AnyElement, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};

use super::super::super::{PageLinkIconAction, PageLinkIconPickerAction};
use crate::ui::board_workspace::{
    alpha, div, img,
    page::editor::{
        render::page_toggle_disclosure_focus_shadow, PageLinkIconPickerState,
        PageLinkIconPickerTab, PageLinkIconView,
    },
    px, rgb,
};

use super::super::catalog::{category_specs, PageLinkPickerCategorySpec};

#[derive(Clone, Copy)]
struct PageLinkEmojiCategoryButton {
    selected: bool,
    section_index: Option<usize>,
    label: &'static str,
    adds_custom_emoji: bool,
    custom_emoji_add_enabled: bool,
    enabled: bool,
}

impl PageLinkIconView {
    pub(super) fn render_page_link_emoji_category_nav(
        &self,
        state: &PageLinkIconPickerState,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id("notion-page-link-emoji-category-nav")
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .h(px(51.0))
            .px(px(12.0))
            .pt(px(8.0))
            .pb(px(10.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .flex()
            .items_center()
            .gap(px(3.2))
            .children(
                category_specs()
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, spec)| {
                        self.render_page_link_emoji_category_button(state, index, spec, cx)
                    }),
            )
            .into_any_element()
    }

    fn render_page_link_emoji_category_button(
        &self,
        state: &PageLinkIconPickerState,
        category_index: usize,
        spec: PageLinkPickerCategorySpec,
        cx: &mut App,
    ) -> AnyElement {
        let button_state = page_link_emoji_category_button(state, category_index, spec);
        div()
            .id(ElementId::Name(
                format!("notion-page-link-emoji-category-{category_index}").into(),
            ))
            .role(Role::Button)
            .aria_label(page_link_emoji_category_label(button_state))
            .aria_toggled(if button_state.selected && button_state.enabled {
                gpui::Toggled::True
            } else {
                gpui::Toggled::False
            })
            .size(px(32.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .opacity(if button_state.enabled { 1.0 } else { 0.38 })
            .when(button_state.selected && button_state.enabled, |button| {
                button.bg(alpha(self.theme.text_primary, 0.05))
            })
            .when(button_state.enabled, |button| {
                self.enable_page_link_emoji_category_button(button, category_index, button_state)
            })
            .child(
                img(self.icons.page_link_picker_categories[category_index].render(cx))
                    .size(px(22.0)),
            )
            .into_any_element()
    }

    fn enable_page_link_emoji_category_button(
        &self,
        button: Stateful<Div>,
        category_index: usize,
        state: PageLinkEmojiCategoryButton,
    ) -> Stateful<Div> {
        button
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .focus_visible(|style| style.shadow(page_toggle_disclosure_focus_shadow()))
            .on_mouse_down(
                MouseButton::Left,
                self.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let action = if state.adds_custom_emoji {
                        PageLinkIconAction::Picker(PageLinkIconPickerAction::SetTab(
                            PageLinkIconPickerTab::Upload,
                        ))
                    } else {
                        PageLinkIconAction::Picker(PageLinkIconPickerAction::JumpCategory {
                            section_index: state
                                .section_index
                                .expect("a non-add emoji category must have a section"),
                            category_index,
                        })
                    };
                    this.emit(action, window, cx);
                }),
            )
            .on_key_down(
                self.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let Some(action) = page_link_emoji_category_key_action(
                        event,
                        category_index,
                        state.custom_emoji_add_enabled,
                    ) else {
                        return;
                    };
                    window.prevent_default();
                    cx.stop_propagation();
                    this.emit(action, window, cx);
                }),
            )
    }
}

fn page_link_emoji_category_key_action(
    event: &KeyDownEvent,
    category_index: usize,
    custom_emoji_add_enabled: bool,
) -> Option<PageLinkIconAction> {
    let last_enabled = page_link_emoji_last_enabled_category(custom_emoji_add_enabled);
    let next = match event.keystroke.key.as_str() {
        "left" if !event.keystroke.modifiers.modified() => Some(category_index.saturating_sub(1)),
        "right" if !event.keystroke.modifiers.modified() => {
            Some((category_index + 1).min(last_enabled))
        }
        "enter" | "space" if !event.keystroke.modifiers.modified() => Some(category_index),
        _ => None,
    };
    let next = next?;
    let next_spec = category_specs()[next];
    if let Some(section_index) = next_spec.section_index {
        Some(PageLinkIconAction::Picker(
            PageLinkIconPickerAction::JumpCategory {
                section_index,
                category_index: next,
            },
        ))
    } else {
        Some(PageLinkIconAction::Picker(
            PageLinkIconPickerAction::SetTab(PageLinkIconPickerTab::Upload),
        ))
    }
}

fn page_link_emoji_category_button(
    state: &PageLinkIconPickerState,
    category_index: usize,
    spec: PageLinkPickerCategorySpec,
) -> PageLinkEmojiCategoryButton {
    let adds_custom_emoji = spec.label == "add";
    let custom_emoji_add_enabled =
        state.custom_emoji_creation_allowed && !state.custom_emoji_limit_reached();
    PageLinkEmojiCategoryButton {
        selected: state.category_index == category_index,
        section_index: spec.section_index,
        label: spec.label,
        adds_custom_emoji,
        custom_emoji_add_enabled,
        enabled: spec.section_index.is_some() || (adds_custom_emoji && custom_emoji_add_enabled),
    }
}

fn page_link_emoji_category_label(state: PageLinkEmojiCategoryButton) -> String {
    if state.enabled && state.adds_custom_emoji {
        return "Add custom emoji".to_string();
    }
    if state.enabled {
        return format!("Jump to category: {}", state.label);
    }
    "Add image, unavailable".to_string()
}

fn page_link_emoji_last_enabled_category(custom_emoji_add_enabled: bool) -> usize {
    if custom_emoji_add_enabled {
        return category_specs().len() - 1;
    }
    category_specs()
        .iter()
        .rposition(|spec| spec.section_index.is_some())
        .expect("the emoji picker has enabled categories")
}
