use gpui::{anchored, deferred, Anchor, App, ClickEvent, Stateful, StatefulInteractiveElement};

use super::super::super::{
    alpha, div, img, px, rgb, AnyElement, Div, FluentBuilder, FontWeight, InteractiveElement,
    ParentElement, Styled,
};
use super::super::actions::PageBlockContextMenuAction;
use super::super::catalog::{
    PageBlockMenuAction, PageBlockMenuActionSpec, PAGE_BLOCK_MENU_PANEL_INSET,
    PAGE_BLOCK_MENU_ROW_HEIGHT, PAGE_BLOCK_MENU_SUBMENU_OVERLAP, PAGE_BLOCK_MENU_WIDTH,
};
use super::super::chrome::PageBlockMenuRowState;
use super::PageBlockContextMenuRenderer;

pub(super) struct PageBlockMenuRootRow<'a> {
    pub(super) action: &'a PageBlockMenuActionSpec,
    pub(super) index: usize,
    pub(super) selected: bool,
    pub(super) submenu: Option<AnyElement>,
}

impl PageBlockContextMenuRenderer {
    pub(super) fn render_page_block_menu_root_row(
        &self,
        spec: PageBlockMenuRootRow<'_>,
        cx: &mut App,
    ) -> Stateful<Div> {
        let enabled = self.target.action_enabled(spec.action.action);
        let mut row = self
            .page_block_menu_row(
                PAGE_BLOCK_MENU_WIDTH,
                format!("notion-block-menu-action-{}", spec.action.label),
                spec.action.label,
                PageBlockMenuRowState {
                    selected: spec.selected,
                    enabled,
                },
            )
            .child(self.render_page_block_menu_icon(spec.action.action, spec.action.icon, cx))
            .child(self.render_page_block_menu_root_label(spec.action))
            .child(self.render_page_block_menu_root_trailing(spec.action, cx));
        if enabled {
            let block_id = self.target.block.block_id.clone();
            let action = spec.action.action;
            let actions = self.actions.clone();
            row = row.on_click(move |_: &ClickEvent, window, cx| {
                cx.stop_propagation();
                actions.emit(
                    PageBlockContextMenuAction::ActivateRoot {
                        block_id: block_id.clone(),
                        action,
                    },
                    window,
                    cx,
                );
            });
        }
        let row =
            self.bind_page_block_menu_root_hover(row, spec.action.action, spec.index, enabled);
        row.when_some(spec.submenu, |row, submenu| {
            row.relative().child(
                div()
                    .absolute()
                    .left(px(PAGE_BLOCK_MENU_WIDTH
                        - PAGE_BLOCK_MENU_PANEL_INSET
                        - PAGE_BLOCK_MENU_SUBMENU_OVERLAP))
                    .top(px(PAGE_BLOCK_MENU_ROW_HEIGHT / 2.0))
                    .size_0()
                    .child(
                        deferred(anchored().anchor(Anchor::LeftCenter).child(submenu))
                            .with_priority(121),
                    ),
            )
        })
    }

    fn render_page_block_menu_root_label(&self, spec: &PageBlockMenuActionSpec) -> Div {
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(spec.label)
            .when(spec.action == PageBlockMenuAction::Present, |label| {
                label.child(
                    div()
                        .rounded(px(3.0))
                        .px(px(4.0))
                        .py(px(1.0))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(self.theme.text_muted))
                        .bg(alpha(self.theme.text_primary, 0.08))
                        .child("Beta"),
                )
            })
    }

    fn render_page_block_menu_root_trailing(
        &self,
        spec: &PageBlockMenuActionSpec,
        cx: &mut App,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .when_some(spec.shortcut, |right, shortcut| {
                right.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child(shortcut),
                )
            })
            .when(page_block_menu_action_has_submenu(spec.action), |right| {
                right.child(img(self.icons.block_menu_chevron.render(cx)).size(px(16.0)))
            })
            .when(spec.action == PageBlockMenuAction::CodeWrap, |right| {
                right.child(self.render_page_code_wrap_switch())
            })
    }

    fn bind_page_block_menu_root_hover(
        &self,
        mut row: Stateful<Div>,
        action: PageBlockMenuAction,
        index: usize,
        enabled: bool,
    ) -> Stateful<Div> {
        let block_id = self.target.block.block_id.clone();
        let actions = self.actions.clone();
        row.interactivity()
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered && enabled {
                    actions.emit(
                        PageBlockContextMenuAction::HoverRoot {
                            block_id: block_id.clone(),
                            action,
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

fn page_block_menu_action_has_submenu(action: PageBlockMenuAction) -> bool {
    matches!(
        action,
        PageBlockMenuAction::CodeLanguage
            | PageBlockMenuAction::TurnInto
            | PageBlockMenuAction::Color
            | PageBlockMenuAction::QuoteSize
            | PageBlockMenuAction::Skills
    )
}
