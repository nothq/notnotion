use super::render::{
    inline_toolbar_dialog_button_id, inline_toolbar_dialog_label, toolbar_dialog_button_id,
};
use super::{
    alpha, div, img, px, AnyElement, Div, IconAsset, InlineDatabaseToolbarTarget,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, ToolbarDialogKind, ViewTabKind,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(super) fn render_toolbar_icon_node(
        &self,
        icon_node: ToolbarIconNode<'_>,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        match (inline_target, icon_node.dialog) {
            (Some(target), Some(dialog)) => {
                self.inline_toolbar_icon_button(icon_node.icon, dialog, target, cx)
            }
            _ => {
                (self.toolbar_icon_button(icon_node.icon, icon_node.dialog, cx)).into_any_element()
            }
        }
    }

    pub(super) fn inline_toolbar_icon_button(
        &self,
        icon: &IconAsset,
        dialog: ToolbarDialogKind,
        target: &InlineDatabaseToolbarTarget,
        cx: &mut App,
    ) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        let mouse_parent_surface = target.parent_surface.clone();
        let key_parent_surface = target.parent_surface.clone();
        div()
            .id(inline_toolbar_dialog_button_id(dialog))
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label(inline_toolbar_dialog_label(dialog))
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(
                    BoardToolbarAction::OpenDialog {
                        dialog,
                        parent: Some(mouse_parent_surface.clone()),
                    },
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
                    BoardToolbarAction::OpenDialog {
                        dialog,
                        parent: Some(key_parent_surface.clone()),
                    },
                    window,
                    cx,
                );
            })
            .child(
                img(icon.render(cx))
                    .relative()
                    .left(px(-1.0))
                    .size(px(14.0)),
            )
            .into_any_element()
    }

    pub(super) fn render_toolbar_search_node(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        (self.render_toolbar_search_button(inline_target, cx)).into_any_element()
    }

    pub(crate) fn toolbar_icon_button(
        &self,
        icon: &IconAsset,
        dialog: Option<ToolbarDialogKind>,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let dialog = dialog.expect("database toolbar icon buttons require a dialog action");
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        div()
            .id(toolbar_dialog_button_id(dialog))
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label(inline_toolbar_dialog_label(dialog))
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(
                    BoardToolbarAction::OpenDialog {
                        dialog,
                        parent: None,
                    },
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
                    BoardToolbarAction::OpenDialog {
                        dialog,
                        parent: None,
                    },
                    window,
                    cx,
                );
            })
            .child(
                img(icon.render(cx))
                    .relative()
                    .left(px(-1.0))
                    .size(px(14.0)),
            )
    }
}

#[derive(Clone, Copy)]
pub(super) struct ToolbarIconNode<'a> {
    icon: &'a IconAsset,
    dialog: Option<ToolbarDialogKind>,
}

impl<'a> ToolbarIconNode<'a> {
    pub(super) const fn new(icon: &'a IconAsset, dialog: Option<ToolbarDialogKind>) -> Self {
        Self { icon, dialog }
    }
}

impl crate::model::BoardSnapshot {
    pub(crate) fn toolbar_undated_count(&self, kind: ViewTabKind) -> Option<usize> {
        match kind {
            ViewTabKind::Timeline | ViewTabKind::Calendar => {
                self.date_undated_count.filter(|count| *count > 0)
            }
            ViewTabKind::Table
            | ViewTabKind::Board
            | ViewTabKind::List
            | ViewTabKind::Gallery
            | ViewTabKind::Unknown => None,
        }
    }
}
