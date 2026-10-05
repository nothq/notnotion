use super::{
    alpha, div, img, px, AnyElement, InlineDatabaseToolbarTarget, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(super) fn render_inline_toolbar_visibility_button(
        &self,
        minimize: bool,
        cx: &mut App,
    ) -> AnyElement {
        let (id, label, icon) = if minimize {
            (
                "notion-inline-toolbar-minimize",
                "Minimize",
                &self.icons.toolbar_minimize,
            )
        } else {
            (
                "notion-inline-toolbar-restore",
                "Expand",
                &self.icons.toolbar_restore,
            )
        };
        div()
            .id(id)
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label(label)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::SetMinimized(minimize)
                }),
            )
            .child(img(icon.render(cx)).size(px(16.0)))
            .into_any_element()
    }

    pub(super) fn render_ai_autofill_button(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        let parent_surface = inline_target.map(|target| target.parent_surface.clone());
        (div()
            .id("notion-ai-autofill-button")
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label("AI Autofill")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::ToggleAiAutofill(parent_surface.clone())
                }),
            )
            .child(
                img(self.icons.toolbar_magic_wand.render(cx))
                    .relative()
                    .left(px(-1.0))
                    .size(px(14.0)),
            ))
        .into_any_element()
    }

    pub(super) fn render_inline_database_expand_button(
        &self,
        target: InlineDatabaseToolbarTarget,
        cx: &mut App,
    ) -> AnyElement {
        (div()
            .id("notion-inline-database-open-full-page")
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label("Open as full page")
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::ExpandDatabase(target.clone())
                }),
            )
            .child(
                img(self.icons.toolbar_expand.render(cx))
                    .relative()
                    .left(px(-1.0))
                    .size(px(14.0)),
            ))
        .into_any_element()
    }
}
