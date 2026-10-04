use super::{
    alpha, div, img, px, relative, rgb, rgba, AnyElement, Div, FontWeight, IconAsset,
    InlineDatabaseToolbarTarget, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled, ViewTab, ViewTabKind,
    VIEW_TAB_ICON_GAP, VIEW_TAB_ICON_SIZE,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(crate) fn view_tab(
        &self,
        _index: usize,
        tab: &ViewTab,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        let provider_view_id_for_mouse = tab.provider_view_id.clone();
        let provider_view_id_for_key = tab.provider_view_id.clone();
        let inline_view_for_mouse = inline_target.map(|target| target.inline_view.clone());
        let inline_view_for_key = inline_target.map(|target| target.inline_view.clone());
        (div()
            .id(view_tab_id(tab))
            .role(Role::Tab)
            .aria_label(tab.label.clone())
            .aria_selected(tab.active)
            .focusable()
            .tab_stop(true)
            .h(px(40.0))
            .flex_none()
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                mouse_actions.emit(
                    BoardToolbarAction::SelectView {
                        provider_view_id: provider_view_id_for_mouse.clone(),
                        inline_view: inline_view_for_mouse.clone(),
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
                    BoardToolbarAction::SelectView {
                        provider_view_id: provider_view_id_for_key.clone(),
                        inline_view: inline_view_for_key.clone(),
                    },
                    window,
                    cx,
                );
            })
            .child(self.render_view_tab_content(tab, cx)))
        .into_any_element()
    }

    fn render_view_tab_content(&self, tab: &ViewTab, cx: &mut App) -> Div {
        let (icon, active) = self.icons.view_tab_icon(tab);
        div()
            .h(px(32.0))
            .mt(px(1.0))
            .px(px(13.0))
            .rounded(px(20.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .line_height(relative(1.2))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.view_tab_text_color(active))
            .bg(self.view_tab_background(active))
            .child(
                img(icon.render(cx))
                    .w(px(VIEW_TAB_ICON_SIZE))
                    .h(px(VIEW_TAB_ICON_SIZE))
                    .mr(px(VIEW_TAB_ICON_GAP)),
            )
            .child(tab.label.clone())
    }

    fn view_tab_text_color(&self, active: bool) -> gpui::Hsla {
        if active {
            rgb(self.theme.text_primary).into()
        } else {
            rgb(self.theme.view_tab_icon_inactive).into()
        }
    }

    fn view_tab_background(&self, active: bool) -> gpui::Hsla {
        if active {
            rgba(self.theme.tab_active_bg)
        } else {
            alpha(0xffffff, 0.0)
        }
    }
}

fn view_tab_id(tab: &ViewTab) -> String {
    format!("notion-database-view-tab-{}", tab.provider_view_id.as_str())
}

impl crate::ui::IconSet {
    pub(crate) fn view_tab_icon(&self, tab: &ViewTab) -> (&IconAsset, bool) {
        match (tab.kind, tab.active) {
            (ViewTabKind::Table, true) => (&self.view_table_active, true),
            (ViewTabKind::Table, false) => (&self.view_table_inactive, false),
            (ViewTabKind::Board, true) => (&self.view_board_active, true),
            (ViewTabKind::Board, false) => (&self.view_board_inactive, false),
            (ViewTabKind::List, true) => (&self.view_list_active, true),
            (ViewTabKind::List, false) => (&self.view_list_inactive, false),
            (ViewTabKind::Gallery, true) => (&self.view_gallery_active, true),
            (ViewTabKind::Gallery, false) => (&self.view_gallery_inactive, false),
            (ViewTabKind::Timeline, true) => (&self.view_timeline_active, true),
            (ViewTabKind::Timeline, false) => (&self.view_timeline_inactive, false),
            (ViewTabKind::Calendar, true) => (&self.view_calendar_active, true),
            (ViewTabKind::Calendar, false) => (&self.view_calendar_inactive, false),
            (ViewTabKind::Unknown, true) => (&self.view_table_active, true),
            (ViewTabKind::Unknown, false) => (&self.view_table_inactive, false),
        }
    }
}
