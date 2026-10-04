use crate::ui::ColorSpec;
use gpui::{App, ClickEvent, Stateful};

use super::super::super::{
    div, img, page_block_background, page_block_foreground, px, rgb, rgba, CardPageBlockColor,
    CardPageBlockColorValue, Div, FluentBuilder, FontWeight, InteractiveElement, ParentElement,
    Role, StatefulInteractiveElement, Styled,
};
use super::super::actions::PageBlockContextMenuAction;
use super::super::catalog::PAGE_BLOCK_SUBMENU_WIDTH;
use super::super::chrome::PageBlockMenuRowState;
use super::super::render::PageBlockContextMenuRenderer;
use super::super::state::{PageBlockContextMenuPanel, PageBlockMenuSelection};

#[derive(Clone, Copy)]
struct PageBlockColorRowSpec<'a> {
    block_id: &'a str,
    color: CardPageBlockColor,
    index: usize,
    selected: bool,
    current: bool,
    show_shortcut: bool,
}

impl PageBlockContextMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor::context_menu) fn render_page_block_color_menu(
        &self,
        cx: &mut App,
    ) -> Stateful<Div> {
        let selection = self.menu.selection;
        let current = self.target.current_color;
        let last_used = self.last_used_color;
        self.page_block_menu_surface(PAGE_BLOCK_SUBMENU_WIDTH, self.viewport_height * 0.70)
            .id("notion-block-color-menu")
            .role(Role::Dialog)
            .aria_label("Change block color")
            .overflow_y_scroll()
            .child(
                div()
                    .id("notion-block-color-options")
                    .role(Role::Menu)
                    .child(self.render_page_block_last_used_color(last_used, selection, cx))
                    .child(self.render_page_block_color_section(false, current, selection, cx))
                    .child(self.render_page_block_color_section(true, current, selection, cx)),
            )
    }

    fn render_page_block_last_used_color(
        &self,
        color: CardPageBlockColor,
        selection: PageBlockMenuSelection,
        cx: &mut App,
    ) -> Div {
        div()
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(self.render_page_block_color_section_title("Last used"))
            .child(self.render_page_block_color_row(
                PageBlockColorRowSpec {
                    block_id: &self.target.block.block_id,
                    color,
                    index: 0,
                    selected: selection.is_row(0),
                    current: false,
                    show_shortcut: true,
                },
                cx,
            ))
    }

    fn render_page_block_color_section(
        &self,
        background: bool,
        current: Option<CardPageBlockColor>,
        selection: PageBlockMenuSelection,
        cx: &mut App,
    ) -> Div {
        let offset = if background { 11 } else { 1 };
        let title = if background {
            "Background color"
        } else {
            "Text color"
        };
        div()
            .relative()
            .mt(px(6.0))
            .pt(px(6.0))
            .px(px(4.0))
            .pb(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(page_block_color_section_divider(self.theme.surface_border))
            .child(self.render_page_block_color_section_title(title))
            .children(
                CardPageBlockColorValue::ALL
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(value_index, value)| {
                        let color = if background {
                            CardPageBlockColor::Background(value)
                        } else {
                            CardPageBlockColor::Text(value)
                        };
                        let index = offset + value_index;
                        self.render_page_block_color_row(
                            PageBlockColorRowSpec {
                                block_id: &self.target.block.block_id,
                                color,
                                index,
                                selected: selection.is_row(index),
                                current: current == Some(color),
                                show_shortcut: false,
                            },
                            cx,
                        )
                    }),
            )
    }

    fn render_page_block_color_row(
        &self,
        spec: PageBlockColorRowSpec<'_>,
        cx: &mut App,
    ) -> Stateful<Div> {
        let enabled = self.target.can_color;
        let label = page_block_color_label(spec.color);
        let mut row = self
            .page_block_menu_row(
                PAGE_BLOCK_SUBMENU_WIDTH,
                format!(
                    "notion-block-color-{}-{}",
                    spec.index,
                    spec.color.api_value()
                ),
                &label,
                PageBlockMenuRowState {
                    selected: spec.selected,
                    enabled,
                },
            )
            .role(Role::MenuItem)
            .when(spec.selected && enabled, |row| row.aria_active_descendant())
            .child(self.render_page_block_color_swatch(spec.color))
            .child(div().min_w(px(0.0)).flex_grow(1.0).child(label))
            .when(spec.show_shortcut, |row| {
                row.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child("⌘⇧H"),
                )
            })
            .when(spec.current, |row| {
                row.child(img(self.icons.block_menu_checked.render(cx)).size(px(16.0)))
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
                        panel: PageBlockContextMenuPanel::Color,
                        index,
                    },
                    window,
                    cx,
                );
            });
        }
        self.bind_page_block_color_hover(row, spec, enabled, cx)
    }

    fn bind_page_block_color_hover(
        &self,
        mut row: Stateful<Div>,
        spec: PageBlockColorRowSpec<'_>,
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
                            panel: PageBlockContextMenuPanel::Color,
                            index,
                        },
                        window,
                        cx,
                    );
                }
            });
        row
    }

    fn render_page_block_color_swatch(&self, color: CardPageBlockColor) -> Div {
        div()
            .size(px(26.0))
            .flex_none()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .when_some(
                page_block_background(color, self.appearance_mode),
                |swatch, background| swatch.bg(background),
            )
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(16.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(page_block_foreground(color, self.appearance_mode))
            .when(matches!(color, CardPageBlockColor::Text(_)), |swatch| {
                swatch.child("A")
            })
    }
}

pub(in crate::ui::board_workspace::page::editor::context_menu) fn page_block_color_menu_row_color(
    index: usize,
    last_used: CardPageBlockColor,
) -> Option<CardPageBlockColor> {
    match index {
        0 => Some(last_used),
        1..=10 => Some(CardPageBlockColor::Text(
            CardPageBlockColorValue::ALL[index - 1],
        )),
        11..=20 => Some(CardPageBlockColor::Background(
            CardPageBlockColorValue::ALL[index - 11],
        )),
        _ => None,
    }
}

fn page_block_color_label(color: CardPageBlockColor) -> String {
    match color {
        CardPageBlockColor::Text(value) => format!("{} text", value.label()),
        CardPageBlockColor::Background(value) => format!("{} background", value.label()),
    }
}

fn page_block_color_section_divider(color: ColorSpec) -> Div {
    div()
        .absolute()
        .left(px(8.0))
        .right(px(8.0))
        .top(px(-1.0))
        .h(px(1.0))
        .bg(rgba(color))
}
