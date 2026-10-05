use gpui::App;

use super::{
    alpha, div, px, relative, rgb, rgba, AnyElement, Bounds, Context, Div, FluentBuilder,
    FontWeight, InlineDatabaseToolbarTarget, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Role, StatefulInteractiveElement, Styled,
    SurfaceState, Window, INLINE_DATABASE_CONTROLS_HEIGHT, INLINE_DATABASE_MORE_HORIZONTAL_PADDING,
    INLINE_DATABASE_VIEW_TAB_GAP,
};
use crate::ui::board_workspace::board::toolbar::BoardToolbarRenderer;
use crate::ui::surface::InlineDatabaseViewMenuOwner;
use crate::ui::{view_actions::ViewActionSink, Theme, ViewTab};

pub(in crate::ui::board_workspace::inline_database) struct InlineDatabaseViewTabsRenderer<'a> {
    theme: Theme,
    view_tabs: &'a [ViewTab],
    visible_indices: Vec<usize>,
    toolbar: BoardToolbarRenderer<'a>,
    actions: ViewActionSink<InlineDatabaseTabAction>,
}

enum InlineDatabaseTabAction {
    Measured(Bounds<Pixels>),
    MoreMeasured(Bounds<Pixels>),
    Toggle(Box<InlineDatabaseToolbarTarget>),
}

pub(in crate::ui::board_workspace::inline_database) fn inline_database_view_tabs_renderer<'a>(
    surface: &'a SurfaceState,
    target: &InlineDatabaseToolbarTarget,
    cx: &Context<SurfaceState>,
) -> InlineDatabaseViewTabsRenderer<'a> {
    InlineDatabaseViewTabsRenderer {
        theme: surface.theme,
        view_tabs: &surface.board.view_tabs,
        visible_indices: surface
            .board_view
            .inline_database_visible_tab_indices(&surface.board.view_tabs),
        toolbar: surface.board_toolbar_renderer(Some(target), cx),
        actions: ViewActionSink::new(cx, handle_inline_database_tab_action),
    }
}

fn handle_inline_database_tab_action(
    surface: &mut SurfaceState,
    action: InlineDatabaseTabAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        InlineDatabaseTabAction::Measured(bounds) => {
            if surface.board_view.update_inline_database_view_tabs_layout(
                &surface.board.view_tabs,
                bounds,
                window,
            ) {
                window.refresh();
            }
        }
        InlineDatabaseTabAction::MoreMeasured(bounds) => {
            surface.board_view.inline_database_view_more_bounds = Some(bounds);
        }
        InlineDatabaseTabAction::Toggle(target) => {
            let anchor = surface
                .board_view
                .inline_database_view_more_bounds
                .expect("inline database More button must be laid out before activation");
            let view_tabs = surface.board.view_tabs.clone().into();
            let owner = InlineDatabaseViewMenuOwner {
                database_block_id: target.database_block_id.clone(),
                inline_view: target.inline_view.clone(),
            };
            target
                .parent_surface
                .update(cx, move |parent, cx| {
                    parent.toggle_inline_database_view_menu(anchor, owner, view_tabs, cx);
                })
                .expect("inline database view menu requires its parent surface");
        }
    }
}

impl InlineDatabaseViewTabsRenderer<'_> {
    pub(in crate::ui::board_workspace::inline_database) fn render_inline_database_view_tabs(
        &self,
        target: &InlineDatabaseToolbarTarget,
        cx: &mut App,
    ) -> AnyElement {
        let visible_indices = self.visible_indices.clone();
        let hidden_count = self.view_tabs.len().saturating_sub(visible_indices.len());
        let actions = self.actions.clone();
        div()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first().copied() else {
                    return;
                };
                actions.emit(InlineDatabaseTabAction::Measured(bounds), window, cx);
            })
            .id(format!(
                "notion-inline-database-view-tabs-{}",
                target.database_block_id
            ))
            .role(Role::TabList)
            .aria_label("Database views")
            .relative()
            .h(px(INLINE_DATABASE_CONTROLS_HEIGHT))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .child(self.render_inline_database_visible_tabs(
                visible_indices,
                hidden_count,
                target,
                cx,
            ))
            .into_any_element()
    }

    fn render_inline_database_visible_tabs(
        &self,
        visible_indices: Vec<usize>,
        hidden_count: usize,
        target: &InlineDatabaseToolbarTarget,
        cx: &mut App,
    ) -> Div {
        div()
            .size_full()
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(INLINE_DATABASE_VIEW_TAB_GAP))
            .children(visible_indices.into_iter().filter_map(|index| {
                self.view_tabs
                    .get(index)
                    .map(|tab| self.toolbar.view_tab(index, tab, Some(target), cx))
            }))
            .when(hidden_count > 0, |tabs| {
                tabs.child(self.render_inline_database_more_views(hidden_count, target, cx))
            })
    }

    fn render_inline_database_more_views(
        &self,
        hidden_count: usize,
        target: &InlineDatabaseToolbarTarget,
        cx: &mut App,
    ) -> AnyElement {
        let more_actions = self.actions.clone();
        let mouse_actions = self.actions.clone();
        let key_actions = self.actions.clone();
        let target_for_mouse = target.clone();
        let target_for_key = target.clone();
        let menu_open = inline_database_view_menu_is_open(target, cx);
        div()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first().copied() else {
                    return;
                };
                more_actions.emit(InlineDatabaseTabAction::MoreMeasured(bounds), window, cx);
            })
            .h(px(40.0))
            .flex_none()
            .flex()
            .items_center()
            .child(
                self.inline_database_more_views_button(hidden_count, target, menu_open)
                    .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        mouse_actions.emit(
                            InlineDatabaseTabAction::Toggle(Box::new(target_for_mouse.clone())),
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
                            InlineDatabaseTabAction::Toggle(Box::new(target_for_key.clone())),
                            window,
                            cx,
                        );
                    })
                    .child(format!("{hidden_count} more…")),
            )
            .into_any_element()
    }

    fn inline_database_more_views_button(
        &self,
        hidden_count: usize,
        target: &InlineDatabaseToolbarTarget,
        menu_open: bool,
    ) -> gpui::Stateful<Div> {
        div()
            .id(format!(
                "notion-inline-database-more-views-{}",
                target.database_block_id
            ))
            .role(Role::Button)
            .aria_label(format!("{hidden_count} more database views"))
            .aria_expanded(menu_open)
            .focusable()
            .tab_stop(true)
            .h(px(32.0))
            .mt(px(1.0))
            .px(px(INLINE_DATABASE_MORE_HORIZONTAL_PADDING))
            .rounded(px(20.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .text_size(px(14.0))
            .line_height(relative(1.2))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.view_tab_icon_inactive))
            .bg(if menu_open {
                rgba(self.theme.tab_active_bg)
            } else {
                alpha(0xffffff, 0.0)
            })
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
    }
}

fn inline_database_view_menu_is_open(target: &InlineDatabaseToolbarTarget, cx: &App) -> bool {
    target.parent_surface.upgrade().is_some_and(|surface| {
        surface
            .read(cx)
            .notion_chrome
            .inline_database_view_menu
            .as_ref()
            .is_some_and(|state| state.database_block_id == target.database_block_id)
    })
}
