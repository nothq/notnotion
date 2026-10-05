use crate::ui::{
    render_page_command_bulleted_list_icon, render_page_command_callout_icon,
    render_page_command_glyph_icon, render_page_command_numbered_list_icon,
    render_page_command_todo_list_icon, render_page_command_toggle_list_icon,
};
use gpui::{App, ClickEvent, IntoElement, Stateful};

use super::super::super::{
    div, img, px, CardPageBlockKind, Div, FluentBuilder, InteractiveElement, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::super::actions::PageBlockContextMenuAction;
use super::super::catalog::{
    page_block_turn_into_glyph, PageBlockTurnIntoSpec, PAGE_BLOCK_SUBMENU_WIDTH,
    PAGE_BLOCK_TURN_INTO_ACTIONS,
};
use super::super::chrome::PageBlockMenuRowState;
use super::super::render::PageBlockContextMenuRenderer;
use super::super::state::PageBlockContextMenuPanel;

struct PageBlockTurnIntoRow<'a> {
    block_id: &'a str,
    conversion: &'a PageBlockTurnIntoSpec,
    index: usize,
    selected: bool,
    current_kind: Option<CardPageBlockKind>,
}

impl PageBlockContextMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor::context_menu) fn render_page_block_turn_into_menu(
        &self,
        cx: &mut App,
    ) -> Stateful<Div> {
        let selection = self.menu.selection;
        let current_kind = self
            .target
            .block
            .editable_content()
            .map(|editable| editable.kind);
        self.page_block_menu_surface(PAGE_BLOCK_SUBMENU_WIDTH, self.viewport_height * 0.70)
            .id("notion-block-turn-into-menu")
            .role(Role::Dialog)
            .aria_label("Turn block into")
            .overflow_y_scroll()
            .child(
                div()
                    .id("notion-block-turn-into-options")
                    .p(px(4.0))
                    .role(Role::Menu)
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .children(PAGE_BLOCK_TURN_INTO_ACTIONS.iter().enumerate().map(
                        |(index, conversion)| {
                            self.render_page_block_turn_into_row(
                                PageBlockTurnIntoRow {
                                    block_id: &self.target.block.block_id,
                                    conversion,
                                    index,
                                    selected: selection.is_row(index),
                                    current_kind,
                                },
                                cx,
                            )
                        },
                    )),
            )
    }

    fn render_page_block_turn_into_row(
        &self,
        spec: PageBlockTurnIntoRow<'_>,
        cx: &mut App,
    ) -> Stateful<Div> {
        let enabled =
            self.target.turn_into_kind(spec.conversion).is_some() && self.target.can_turn_into;
        let icon = self.render_page_block_turn_into_icon(spec.conversion, cx);
        let mut row = self
            .page_block_menu_row(
                PAGE_BLOCK_SUBMENU_WIDTH,
                format!("notion-block-turn-into-{}", spec.conversion.label),
                spec.conversion.label,
                PageBlockMenuRowState {
                    selected: spec.selected,
                    enabled,
                },
            )
            .role(Role::MenuItem)
            .when(spec.selected && enabled, |row| row.aria_active_descendant())
            .child(div().w(px(20.0)).flex().justify_center().child(icon))
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .child(spec.conversion.label),
            )
            .when(spec.current_kind == spec.conversion.kind, |row| {
                row.child(img(self.icons.block_menu_checked.render(cx)).size(px(16.0)))
            })
            .when(spec.conversion.label == "Page in", |row| {
                row.child(img(self.icons.block_menu_chevron.render(cx)).size(px(16.0)))
            });
        if enabled {
            let block_id = spec.block_id.to_string();
            let index = spec.index;
            let actions = self.actions.clone();
            row = row.on_click(move |_: &ClickEvent, window, cx| {
                cx.stop_propagation();
                actions.emit(
                    PageBlockContextMenuAction::ActivateSubmenuRow {
                        block_id: block_id.clone(),
                        panel: PageBlockContextMenuPanel::TurnInto,
                        index,
                    },
                    window,
                    cx,
                );
            });
        }
        self.bind_page_block_turn_into_hover(row, &spec, enabled, cx)
    }

    fn render_page_block_turn_into_icon(
        &self,
        conversion: &PageBlockTurnIntoSpec,
        cx: &mut App,
    ) -> gpui::AnyElement {
        match conversion.kind {
            Some(CardPageBlockKind::Text) => {
                render_page_command_glyph_icon(self.theme, "T").into_any_element()
            }
            Some(CardPageBlockKind::SubHeader) => {
                render_page_command_glyph_icon(self.theme, "H1").into_any_element()
            }
            Some(CardPageBlockKind::SubSubHeader) => {
                render_page_command_glyph_icon(self.theme, "H2").into_any_element()
            }
            Some(CardPageBlockKind::Heading3) => {
                render_page_command_glyph_icon(self.theme, "H3").into_any_element()
            }
            Some(CardPageBlockKind::Heading4) => {
                render_page_command_glyph_icon(self.theme, "H4").into_any_element()
            }
            Some(CardPageBlockKind::BulletedList) => {
                render_page_command_bulleted_list_icon(self.theme).into_any_element()
            }
            Some(CardPageBlockKind::NumberedList) => {
                render_page_command_numbered_list_icon(self.theme).into_any_element()
            }
            Some(CardPageBlockKind::ToDoList) => {
                render_page_command_todo_list_icon(self.theme).into_any_element()
            }
            Some(CardPageBlockKind::ToggleList) => render_page_command_toggle_list_icon(
                self.theme,
                self.icons.page_toggle_collapsed.render(cx),
            )
            .into_any_element(),
            Some(CardPageBlockKind::PageLink) => img(self.icons.page.render(cx))
                .size(px(14.0))
                .into_any_element(),
            Some(CardPageBlockKind::Callout) => {
                render_page_command_callout_icon(self.theme).into_any_element()
            }
            Some(CardPageBlockKind::Quote) => {
                render_page_command_glyph_icon(self.theme, "\"").into_any_element()
            }
            Some(CardPageBlockKind::Code) => {
                render_page_command_glyph_icon(self.theme, "</>").into_any_element()
            }
            None => self.render_page_block_menu_glyph(page_block_turn_into_glyph(conversion.label)),
        }
    }

    fn bind_page_block_turn_into_hover(
        &self,
        mut row: Stateful<Div>,
        spec: &PageBlockTurnIntoRow<'_>,
        enabled: bool,
        _cx: &mut App,
    ) -> Stateful<Div> {
        let block_id = spec.block_id.to_string();
        let index = spec.index;
        let actions = self.actions.clone();
        row.interactivity()
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered && enabled {
                    actions.emit(
                        PageBlockContextMenuAction::HoverSubmenu {
                            block_id: block_id.clone(),
                            panel: PageBlockContextMenuPanel::TurnInto,
                            index,
                        },
                        window,
                        cx,
                    );
                }
            });
        row
    }
}
