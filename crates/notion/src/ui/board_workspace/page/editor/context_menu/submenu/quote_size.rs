use gpui::{App, ClickEvent, Stateful};

use super::super::super::{
    div, px, CardPageQuoteSize, Div, FluentBuilder, InteractiveElement, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::super::actions::PageBlockContextMenuAction;
use super::super::catalog::PAGE_BLOCK_SUBMENU_WIDTH;
use super::super::chrome::PageBlockMenuRowState;
use super::super::render::PageBlockContextMenuRenderer;
use super::super::state::PageBlockContextMenuPanel;

impl PageBlockContextMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor::context_menu) fn render_page_block_quote_size_menu(
        &self,
        cx: &mut App,
    ) -> Stateful<Div> {
        let selection = self.menu.selection;
        self.page_block_menu_surface(PAGE_BLOCK_SUBMENU_WIDTH, self.viewport_height * 0.70)
            .id("notion-block-quote-size-menu")
            .role(Role::Dialog)
            .aria_label("Change quote size")
            .child(
                div()
                    .id("notion-block-quote-size-options")
                    .p(px(4.0))
                    .role(Role::Menu)
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(self.render_page_block_menu_section_title("Quote size"))
                    .children(
                        [CardPageQuoteSize::Default, CardPageQuoteSize::Large]
                            .into_iter()
                            .enumerate()
                            .map(|(index, size)| {
                                self.render_page_block_quote_size_row(
                                    size,
                                    index,
                                    selection.is_row(index),
                                    cx,
                                )
                            }),
                    ),
            )
    }

    fn render_page_block_quote_size_row(
        &self,
        size: CardPageQuoteSize,
        index: usize,
        selected: bool,
        cx: &mut App,
    ) -> Stateful<Div> {
        let enabled = self.target.targets_are_quotes;
        let label = match size {
            CardPageQuoteSize::Default => "Default",
            CardPageQuoteSize::Large => "Large",
        };
        let block_id = self.target.block.block_id.clone();
        let mut row = self
            .page_block_menu_row(
                PAGE_BLOCK_SUBMENU_WIDTH,
                format!("notion-block-quote-size-{label}"),
                label,
                PageBlockMenuRowState { selected, enabled },
            )
            .role(Role::MenuItem)
            .when(selected && enabled, |row| row.aria_active_descendant())
            .child(label);
        if enabled {
            let actions = self.actions.clone();
            row = row.on_click(move |_: &ClickEvent, window, cx| {
                cx.stop_propagation();
                actions.emit(
                    PageBlockContextMenuAction::ActivateSubmenuRow {
                        block_id: block_id.clone(),
                        panel: PageBlockContextMenuPanel::QuoteSize,
                        index,
                    },
                    window,
                    cx,
                );
            });
        }
        self.bind_page_block_quote_size_hover(row, index, enabled, cx)
    }

    fn bind_page_block_quote_size_hover(
        &self,
        mut row: Stateful<Div>,
        index: usize,
        enabled: bool,
        _cx: &mut App,
    ) -> Stateful<Div> {
        let block_id = self.target.block.block_id.clone();
        let actions = self.actions.clone();
        row.interactivity()
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered && enabled {
                    actions.emit(
                        PageBlockContextMenuAction::HoverSubmenu {
                            block_id: block_id.clone(),
                            panel: PageBlockContextMenuPanel::QuoteSize,
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
