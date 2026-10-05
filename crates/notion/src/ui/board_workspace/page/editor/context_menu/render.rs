use std::sync::Arc;

use gpui::{anchored, deferred, Anchor, App, Stateful};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::{
    div, px, rgb, rgba, AnyElement, AppearanceMode, CardPageBlock, CardPageBlockColor, Context,
    Div, ElementId, FluentBuilder, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled, SurfaceState, Theme,
};
use super::actions::{
    handle_page_block_context_menu_action, resolve_page_block_context_menu_target,
    PageBlockContextMenuAction,
};
use super::catalog::{
    page_block_type_label, PageBlockMenuActionSpec, PAGE_BLOCK_MENU_ROW_HEIGHT,
    PAGE_BLOCK_MENU_WIDTH,
};
use super::state::{
    page_block_menu_panel_action, PageBlockContextMenuPanel, PageBlockContextMenuPresentation,
    PageBlockContextMenuState, PageBlockContextMenuTarget, PageBlockMenuSelection,
};
use crate::ui::{view_actions::ViewActionSink, IconSet};

mod row;
use row::PageBlockMenuRootRow;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockContextMenuRenderer {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) viewport_height: f32,
    pub(super) menu: PageBlockContextMenuState,
    pub(super) target: PageBlockContextMenuTarget,
    pub(super) last_used_color: CardPageBlockColor,
    pub(super) actions: ViewActionSink<PageBlockContextMenuAction>,
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_context_menu_renderer(
    surface: &SurfaceState,
    block_id: &str,
    cx: &Context<SurfaceState>,
) -> Option<PageBlockContextMenuRenderer> {
    let menu = surface
        .page_editor
        .page_block_context_menu
        .as_ref()
        .filter(|menu| menu.block_id == block_id)?
        .clone();
    let target = resolve_page_block_context_menu_target(surface, block_id)?;
    Some(PageBlockContextMenuRenderer {
        theme: surface.theme,
        appearance_mode: surface.appearance_mode,
        icons: Arc::clone(&surface.icons),
        viewport_height: surface.viewport.app_height(),
        menu,
        target,
        last_used_color: surface.page_editor.last_used_page_block_color,
        actions: ViewActionSink::new(cx, handle_page_block_context_menu_action),
    })
}

impl PageBlockContextMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn handle_key_down(
        &self,
        event: &gpui::KeyDownEvent,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> bool {
        let Some(action) = self.menu.keyboard_action(event, Some(&self.target)) else {
            return false;
        };
        self.actions.emit(action, window, cx);
        true
    }

    pub(in crate::ui::board_workspace::page::editor) fn render_block(
        &self,
        block: &CardPageBlock,
        row_center: f32,
        cx: &mut App,
    ) -> Div {
        if self.menu.block_id != block.block_id
            || self.menu.presentation == PageBlockContextMenuPresentation::CodeLanguagePicker
        {
            return div();
        }
        self.render_page_block_context_menu(row_center, cx)
    }

    pub(in crate::ui::board_workspace::page::editor) fn render_code_language(
        &self,
        block: &CardPageBlock,
        cx: &mut App,
    ) -> Stateful<Div> {
        if self.menu.block_id != block.block_id {
            return div().id("notion-code-language-menu");
        }
        self.render_page_code_language_menu(cx)
    }

    fn render_page_block_context_menu(&self, row_center: f32, cx: &mut App) -> Div {
        let focus_handle = self.menu.focus_handle.clone();
        let click_away = ClickAwayBoundary::new();
        let main_panel = self.render_page_block_context_menu_root(&click_away, cx);
        let keyboard_menu = self.menu.clone();
        let keyboard_target = self.target.clone();
        let keyboard_actions = self.actions.clone();
        div()
            .absolute()
            .left(px(-33.0))
            .top(px(row_center))
            .size_0()
            .occlude()
            .track_focus(&focus_handle)
            .on_key_down(move |event, window, cx| {
                let Some(action) = keyboard_menu.keyboard_action(event, Some(&keyboard_target))
                else {
                    return;
                };
                keyboard_actions.emit(action, window, cx);
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
            .child(
                deferred(anchored().anchor(Anchor::LeftCenter).child(main_panel))
                    .with_priority(120),
            )
    }

    pub(super) fn render_page_block_context_submenu_for_action(
        &self,
        panel: PageBlockContextMenuPanel,
        action: super::catalog::PageBlockMenuAction,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Option<AnyElement> {
        if page_block_menu_panel_action(panel) != Some(action) {
            return None;
        }
        let submenu = match panel {
            PageBlockContextMenuPanel::Root => return None,
            PageBlockContextMenuPanel::TurnInto => {
                self.render_page_block_turn_into_menu(cx).into_any_element()
            }
            PageBlockContextMenuPanel::Color => {
                self.render_page_block_color_menu(cx).into_any_element()
            }
            PageBlockContextMenuPanel::QuoteSize => self
                .render_page_block_quote_size_menu(cx)
                .into_any_element(),
            PageBlockContextMenuPanel::CodeLanguage => {
                self.render_page_code_language_menu(cx).into_any_element()
            }
        };
        Some(
            click_away
                .member(div().child(submenu), cx)
                .into_any_element(),
        )
    }

    fn render_page_block_context_menu_root(
        &self,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Stateful<Div> {
        let actions = self.target.visible_actions(&self.menu.query);
        let selection = self.menu.root_selection(&self.target, actions.len());
        let filtered = !self.menu.query.is_empty();
        let groups = if filtered {
            self.render_filtered_page_block_menu_actions(&actions, selection, click_away, cx)
        } else {
            self.render_grouped_page_block_menu_actions(&actions, selection, click_away, cx)
        };
        let panel = self
            .page_block_menu_surface(PAGE_BLOCK_MENU_WIDTH, self.viewport_height * 0.70)
            .id(ElementId::Name(
                format!("notion-block-context-menu-{}", self.target.block.block_id).into(),
            ))
            .role(Role::Dialog)
            .aria_label("Block actions")
            .overflow_y_scroll()
            .child(self.render_page_block_menu_search(self.menu.search_input.clone()))
            .child(
                div()
                    .id("notion-block-menu-actions")
                    .role(Role::ListBox)
                    .children(groups),
            )
            .when(!filtered, |panel| {
                panel.when_some(
                    self.render_page_block_menu_footer(&self.target.block),
                    |panel, footer| panel.child(footer),
                )
            });
        let dismiss_actions = self.actions.clone();
        click_away.dismissible_with_handler(
            div().child(panel),
            move |_, window, cx| {
                dismiss_actions.emit(PageBlockContextMenuAction::DismissInteraction, window, cx);
            },
            cx,
        )
    }

    fn render_filtered_page_block_menu_actions(
        &self,
        actions: &[&PageBlockMenuActionSpec],
        selection: PageBlockMenuSelection,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        let panel = self.menu.panel;
        let rows = actions.iter().enumerate().map(|(index, action)| {
            self.render_page_block_menu_root_row(
                PageBlockMenuRootRow {
                    action,
                    index,
                    selected: selection.is_row(index),
                    submenu: self.render_page_block_context_submenu_for_action(
                        panel,
                        action.action,
                        click_away,
                        cx,
                    ),
                },
                cx,
            )
            .into_any_element()
        });
        vec![div()
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .children(rows)
            .when(actions.is_empty(), |list| {
                list.child(
                    div()
                        .h(px(PAGE_BLOCK_MENU_ROW_HEIGHT))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .text_size(px(14.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child("No actions found"),
                )
            })
            .into_any_element()]
    }

    fn render_grouped_page_block_menu_actions(
        &self,
        actions: &[&PageBlockMenuActionSpec],
        selection: PageBlockMenuSelection,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        let panel = self.menu.panel;
        let mut action_index = 0;
        (0..=4)
            .map(|group| {
                let mut rows = Vec::new();
                if group == 0 {
                    rows.push(
                        self.render_page_block_menu_section_title(page_block_type_label(
                            &self.target.block,
                        ))
                        .into_any_element(),
                    );
                }
                for action in actions
                    .iter()
                    .copied()
                    .filter(|action| action.group == group)
                {
                    let index = action_index;
                    action_index += 1;
                    rows.push(
                        self.render_page_block_menu_root_row(
                            PageBlockMenuRootRow {
                                action,
                                index,
                                selected: selection.is_row(index),
                                submenu: self.render_page_block_context_submenu_for_action(
                                    panel,
                                    action.action,
                                    click_away,
                                    cx,
                                ),
                            },
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                self.render_page_block_menu_action_group(group, rows)
            })
            .collect()
    }

    fn render_page_block_menu_action_group(&self, group: u8, rows: Vec<AnyElement>) -> AnyElement {
        div()
            .when(group > 0, |group| {
                group
                    .border_t_1()
                    .border_color(rgba(self.theme.surface_border))
            })
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .children(rows)
            .into_any_element()
    }
}
