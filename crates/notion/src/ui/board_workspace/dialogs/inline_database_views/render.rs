use super::renderer::{InlineDatabaseViewMenuAction, InlineDatabaseViewMenuRenderer};
use super::{
    alpha, div, filtered_inline_database_view_tabs, img, inline_database_view_menu_shadow, px, rgb,
    AnyElement, FluentBuilder, IconAsset, InlineDatabaseViewMenuLayout,
    InlineDatabaseViewMenuSelection, InlineDatabaseViewMenuState, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled, ViewTab,
    VIEW_MENU_ACTION_GAP, VIEW_MENU_ACTION_SEPARATOR_HEIGHT, VIEW_MENU_INPUT_SECTION_HEIGHT,
    VIEW_MENU_LIST_PADDING, VIEW_MENU_ROW_HEIGHT, VIEW_MENU_WIDTH,
};
use gpui::{App, Div};

impl InlineDatabaseViewMenuRenderer {
    pub(super) fn render_inline_database_view_menu_dialog(
        &self,
        state: &InlineDatabaseViewMenuState,
        layout: InlineDatabaseViewMenuLayout,
        cx: &mut App,
    ) -> AnyElement {
        let menu_id = format!(
            "notion-inline-database-view-menu-{}",
            state.database_block_id
        );
        let filtered_tabs = filtered_inline_database_view_tabs(state);
        div()
            .id(menu_id)
            .role(Role::Dialog)
            .aria_label("Database views")
            .absolute()
            .left(px(layout.left))
            .top(px(layout.top))
            .w(px(VIEW_MENU_WIDTH))
            .h(px(layout.height))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(inline_database_view_menu_shadow(self.appearance_mode))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(
                div()
                    .h(px(VIEW_MENU_INPUT_SECTION_HEIGHT))
                    .flex_none()
                    .px(px(16.0))
                    .pt(px(12.0))
                    .child(state.input.clone()),
            )
            .child(self.render_inline_database_view_menu_list(
                state,
                layout.list_height,
                filtered_tabs,
                cx,
            ))
            .into_any_element()
    }

    fn render_inline_database_view_menu_list(
        &self,
        state: &InlineDatabaseViewMenuState,
        list_height: f32,
        filtered_tabs: Vec<&ViewTab>,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let list_id = format!(
            "notion-inline-database-view-list-{}",
            state.database_block_id
        );
        div()
            .id(list_id)
            .role(Role::ListBox)
            .aria_label("Database views")
            .h(px(list_height))
            .flex_none()
            .overflow_y_scroll()
            .p(px(VIEW_MENU_LIST_PADDING))
            .flex()
            .flex_col()
            .children(
                filtered_tabs
                    .into_iter()
                    .map(|tab| self.render_inline_database_view_menu_row(state, tab, cx)),
            )
            .child(self.render_inline_database_view_menu_action_separator())
            .child(self.render_inline_database_view_menu_action_row(
                state,
                InlineDatabaseViewMenuSelection::NewView,
                "New view",
                "+",
            ))
            .child(div().h(px(VIEW_MENU_ACTION_GAP)).flex_none())
            .child(self.render_inline_database_view_menu_action_row(
                state,
                InlineDatabaseViewMenuSelection::NewDataSource,
                "New data source",
                "▣",
            ))
    }

    fn render_inline_database_view_menu_row(
        &self,
        state: &InlineDatabaseViewMenuState,
        tab: &ViewTab,
        cx: &mut App,
    ) -> AnyElement {
        let provider_view_id = tab.provider_view_id.clone();
        let selected =
            state.highlighted == InlineDatabaseViewMenuSelection::View(provider_view_id.clone());
        let row_id = format!(
            "notion-inline-database-view-menu-item-{}-{}",
            state.database_block_id,
            provider_view_id.as_str()
        );
        let hover_view_id = provider_view_id.clone();
        let activate_view_id = provider_view_id;
        let hover_actions = self.actions.clone();
        let (icon, _) = self.icons.view_tab_icon(tab);
        div()
            .id(row_id)
            .role(Role::ListBoxOption)
            .aria_label(tab.label.clone())
            .aria_selected(tab.active)
            .when(selected, |row| row.aria_active_descendant())
            .h(px(VIEW_MENU_ROW_HEIGHT))
            .flex_none()
            .rounded(px(6.0))
            .cursor_pointer()
            .when(selected || tab.active, |row| {
                row.bg(alpha(self.theme.text_primary, 0.055))
            })
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered {
                    hover_actions.emit(
                        InlineDatabaseViewMenuAction::Highlight(
                            InlineDatabaseViewMenuSelection::View(hover_view_id.clone()),
                        ),
                        window,
                        cx,
                    );
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    InlineDatabaseViewMenuAction::Activate(InlineDatabaseViewMenuSelection::View(
                        activate_view_id.clone(),
                    ))
                }),
            )
            .flex()
            .items_center()
            .child(self.render_inline_database_view_drag_handle())
            .child(self.render_inline_database_view_icon(icon, cx))
            .child(self.render_inline_database_view_label(tab))
            .child(self.render_inline_database_view_settings(state, tab, cx))
            .into_any_element()
    }

    fn render_inline_database_view_drag_handle(&self) -> Div {
        div()
            .w(px(28.0))
            .h(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_hint))
            .child("⠿")
    }

    fn render_inline_database_view_icon(&self, icon: &IconAsset, cx: &mut App) -> Div {
        div()
            .size(px(20.0))
            .flex_none()
            .child(img(icon.render(cx)).size(px(20.0)))
    }

    fn render_inline_database_view_label(&self, tab: &ViewTab) -> Div {
        div()
            .ml(px(8.0))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .truncate()
            .text_size(px(14.0))
            .line_height(px(17.0))
            .text_color(rgb(self.theme.text_primary))
            .child(tab.label.clone())
    }

    fn render_inline_database_view_settings(
        &self,
        state: &InlineDatabaseViewMenuState,
        tab: &ViewTab,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        div()
            .id(format!(
                "notion-inline-database-view-settings-{}-{}",
                state.database_block_id,
                tab.provider_view_id.as_str()
            ))
            .size(px(24.0))
            .mr(px(12.0))
            .flex_none()
            .rounded(px(5.0))
            .role(Role::Button)
            .aria_label("Database view settings")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(img(self.icons.card_action_ellipsis.render(cx)).size(px(16.0)))
    }

    fn render_inline_database_view_menu_action_separator(&self) -> AnyElement {
        div()
            .h(px(VIEW_MENU_ACTION_SEPARATOR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .child(
                div()
                    .mx(px(-VIEW_MENU_LIST_PADDING))
                    .w_full()
                    .h(px(1.0))
                    .bg(alpha(self.theme.text_primary, 0.10)),
            )
            .into_any_element()
    }

    fn render_inline_database_view_menu_action_row(
        &self,
        state: &InlineDatabaseViewMenuState,
        selection: InlineDatabaseViewMenuSelection,
        label: &'static str,
        glyph: &'static str,
    ) -> AnyElement {
        let selected = state.highlighted == selection;
        let row_id = format!(
            "notion-inline-database-view-menu-{}-{}",
            state.database_block_id,
            label.replace(' ', "-").to_lowercase()
        );
        let hover_selection = selection.clone();
        let activate_selection = selection;
        let hover_actions = self.actions.clone();
        div()
            .id(row_id)
            .role(Role::ListBoxOption)
            .aria_label(label)
            .when(selected, |row| row.aria_active_descendant())
            .h(px(VIEW_MENU_ROW_HEIGHT))
            .flex_none()
            .rounded(px(6.0))
            .cursor_pointer()
            .when(selected, |row| {
                row.bg(alpha(self.theme.text_primary, 0.055))
            })
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered {
                    hover_actions.emit(
                        InlineDatabaseViewMenuAction::Highlight(hover_selection.clone()),
                        window,
                        cx,
                    );
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    InlineDatabaseViewMenuAction::Activate(activate_selection.clone())
                }),
            )
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(14.0))
            .line_height(px(17.0))
            .text_color(rgb(self.theme.text_primary))
            .child(self.render_inline_database_view_action_glyph(glyph))
            .child(label)
            .into_any_element()
    }

    fn render_inline_database_view_action_glyph(&self, glyph: &'static str) -> Div {
        div()
            .size(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(18.0))
            .text_color(rgb(self.theme.text_muted))
            .child(glyph)
    }
}
