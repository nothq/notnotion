use super::{
    alpha, div, px, rgb, AnyElement, FontWeight, InlineDatabaseToolbarTarget, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(super) fn render_timeline_undated_badge(
        &self,
        undated_count: usize,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        _cx: &mut App,
    ) -> AnyElement {
        let count_label = undated_count_label(undated_count);
        let expanded = self.undated_expanded;
        let key_actions = self.actions.clone();
        let mouse_target = inline_target.cloned();
        let key_target = mouse_target.clone();
        (div()
            .id(undated_badge_id(inline_target))
            .role(Role::Button)
            .aria_label(format!("No date, {undated_count} items"))
            .aria_expanded(expanded)
            .focusable()
            .tab_stop(true)
            .mr(px(8.0))
            .h(px(28.0))
            .px(px(6.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_secondary))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::ToggleUndated(mouse_target.clone())
                }),
            )
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                key_actions.emit(
                    BoardToolbarAction::ToggleUndated(key_target.clone()),
                    window,
                    cx,
                );
            })
            .child(format!("No date ({count_label})")))
        .into_any_element()
    }
}

fn undated_count_label(undated_count: usize) -> String {
    if undated_count >= 100 {
        "99+".to_string()
    } else {
        undated_count.to_string()
    }
}

fn undated_badge_id(target: Option<&InlineDatabaseToolbarTarget>) -> String {
    target.map_or_else(
        || "notion-date-undated-badge".to_string(),
        |target| {
            format!(
                "notion-inline-date-undated-badge-{}",
                target.database_block_id
            )
        },
    )
}
