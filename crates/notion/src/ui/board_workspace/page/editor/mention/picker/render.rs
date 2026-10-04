use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    anchored, deferred, point, Anchor, AnchoredPositionMode, AnyElement, App, Context, Div,
    ElementId, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Role, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};
use gpui_components::backdrop::ClickAwayBoundary;
use gpui_components::text_input::TextInput;

use super::super::actions::{PageMentionAction, PageMentionPickerAction};
use super::super::callbacks::page_mention_action_sink;
use super::super::render::{PageMentionRenderer, PageMentionRendererResources};
use super::super::state::{PageMentionController, PageMentionPickerIdentity};
use super::super::PageMentionClock;
use super::{PAGE_MENTION_PICKER_INSET, PAGE_MENTION_PICKER_ROW_HEIGHT};
use crate::ui::view_actions::ViewNotifier;
use crate::ui::{alpha, div, px, rgb, rgba, PageMentionPickerState, SurfaceState};

mod calendar;
mod dialog;
mod submenu;

#[derive(Clone)]
struct PageMentionPickerPresentation {
    picker: PageMentionPickerState,
    input: Option<Entity<TextInput>>,
}

#[derive(Clone)]
struct PageMentionPickerCommand {
    identity: PageMentionPickerIdentity,
    action: PageMentionPickerAction,
}

impl PageMentionPickerPresentation {
    fn command(&self, action: PageMentionPickerAction) -> PageMentionPickerCommand {
        PageMentionPickerCommand {
            identity: PageMentionPickerIdentity::new(&self.picker),
            action,
        }
    }
}

impl PageMentionController {
    fn picker_presentation(&self) -> Option<PageMentionPickerPresentation> {
        Some(PageMentionPickerPresentation {
            picker: self.picker.clone()?,
            input: self.picker_input.borrow().clone(),
        })
    }
}

impl PageMentionRenderer {
    fn render_picker(
        &self,
        presentation: &PageMentionPickerPresentation,
        cx: &mut App,
    ) -> AnyElement {
        let click_away = ClickAwayBoundary::new();
        let actions = self.actions.clone();
        let identity = PageMentionPickerIdentity::new(&presentation.picker);
        let dialog = click_away.dismissible_with_handler(
            div().child(self.render_page_mention_picker_dialog(presentation)),
            move |_, window, cx| {
                actions.emit(
                    PageMentionAction::Picker {
                        identity: identity.clone(),
                        action: PageMentionPickerAction::Dismiss,
                    },
                    window,
                    cx,
                );
            },
            cx,
        );
        let submenu = presentation.picker.submenu.map(|submenu| {
            self.render_page_mention_picker_submenu(presentation, submenu, &click_away, cx)
        });
        div()
            .child(
                deferred(
                    anchored()
                        .position(point(
                            presentation.picker.anchor.left(),
                            presentation.picker.anchor.bottom(),
                        ))
                        .position_mode(AnchoredPositionMode::Window)
                        .anchor(Anchor::TopLeft)
                        .child(dialog),
                )
                .with_priority(110),
            )
            .when_some(submenu, |host, submenu| host.child(submenu))
            .into_any_element()
    }

    fn render_page_mention_picker_divider(&self) -> Div {
        div()
            .mx(px(PAGE_MENTION_PICKER_INSET))
            .mt(px(4.0))
            .h(px(1.0))
            .bg(rgba(self.resources.theme.menu_ring))
    }

    fn render_page_mention_picker_switch_row(
        &self,
        id: &'static str,
        label: &'static str,
        on: bool,
        command: PageMentionPickerCommand,
    ) -> Stateful<Div> {
        self.render_page_mention_picker_row(id, label)
            .role(Role::Switch)
            .on_mouse_down(MouseButton::Left, self.picker_action_handler(command))
            .child(render_switch(on))
    }

    fn render_page_mention_picker_value_row(
        &self,
        id: &'static str,
        label: &'static str,
        value: impl Into<SharedString>,
        command: PageMentionPickerCommand,
    ) -> Stateful<Div> {
        self.render_page_mention_picker_row(id, label)
            .role(Role::Button)
            .on_mouse_down(MouseButton::Left, self.picker_action_handler(command))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(14.0))
                    .text_color(rgb(self.resources.theme.menu_secondary_text))
                    .child(value.into())
                    .child(div().text_size(px(12.0)).child("›")),
            )
    }

    fn render_page_mention_picker_action_row(
        &self,
        id: &'static str,
        label: &'static str,
        command: PageMentionPickerCommand,
    ) -> Stateful<Div> {
        self.render_page_mention_picker_row(id, label)
            .role(Role::Button)
            .on_mouse_down(MouseButton::Left, self.picker_action_handler(command))
    }

    fn render_page_mention_picker_row(
        &self,
        id: &'static str,
        label: &'static str,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-mention-date-picker-{id}").into(),
            ))
            .aria_label(label)
            .h(px(PAGE_MENTION_PICKER_ROW_HEIGHT))
            .rounded(px(6.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(16.8))
                    .text_color(rgb(self.resources.theme.text_primary))
                    .child(label),
            )
    }

    fn picker_action_handler(
        &self,
        command: PageMentionPickerCommand,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        let actions = self.actions.clone();
        move |_: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            actions.emit(
                PageMentionAction::Picker {
                    identity: command.identity.clone(),
                    action: command.action.clone(),
                },
                window,
                cx,
            );
        }
    }
}

impl SurfaceState {
    /// The picker popover, rendered from the page root so it can overlap the
    /// document like Notion's dialog does.
    pub(crate) fn render_page_mention_picker(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let presentation = self.page_editor.mention.picker_presentation()?;
        let clock = PageMentionClock::from(&self.board);
        Some(
            PageMentionRenderer::new(
                PageMentionRendererResources {
                    theme: self.theme,
                    appearance_mode: self.appearance_mode,
                    icons: Arc::clone(&self.icons),
                    notion_resources: self.notion_resources.clone(),
                    notifier: ViewNotifier::new(cx),
                    viewport_height: self.viewport.app_height(),
                    chrome_top_inset: self.viewport.chrome_top_inset(),
                    clock,
                },
                page_mention_action_sink(cx),
            )
            .render_picker(&presentation, cx),
        )
    }
}

/// A 30 × 18 toggle with a 14 px knob, blue when on.
fn render_switch(on: bool) -> Div {
    div()
        .w(px(30.0))
        .h(px(18.0))
        .rounded(px(9.0))
        .relative()
        .bg(if on {
            rgb(0x2383e2).into()
        } else {
            alpha(0x878378, 0.3)
        })
        .child(
            div()
                .absolute()
                .top(px(2.0))
                .left(px(if on { 14.0 } else { 2.0 }))
                .size(px(14.0))
                .rounded_full()
                .bg(rgb(0xffffff)),
        )
}
