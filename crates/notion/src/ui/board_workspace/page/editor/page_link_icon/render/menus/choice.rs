use gpui::prelude::FluentBuilder;
use gpui::App;
use gpui::{
    anchored, Anchor, AnchoredPositionMode, AnyElement, ClickEvent, Div, ElementId,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, Point, Role, Stateful,
    StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::super::{PageLinkIconAction, PageLinkIconSelectionAction, PageLinkIconTarget};
use crate::ui::{
    board_workspace::{
        alpha, div, img,
        page::editor::{context_menu::page_block_menu_shadow, PageLinkIconView},
        px, rgb,
    },
    NotionNamedIconAsset, NotionNamedIconColor, NotionNamedIconSlug,
};

pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) struct PageLinkNamedIconChoiceMenu
{
    pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) target:
        PageLinkIconTarget,
    pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) slug:
        NotionNamedIconSlug,
}

impl PageLinkIconView {
    pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) fn render_page_link_named_icon_choice_menu(
        &self,
        menu: PageLinkNamedIconChoiceMenu,
        anchor: Point<Pixels>,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> AnyElement {
        anchored()
            .position(anchor)
            .position_mode(AnchoredPositionMode::Local)
            .anchor(Anchor::TopLeft)
            .child(self.render_page_link_named_icon_choice_palette(menu, click_away, cx))
            .into_any_element()
    }

    fn render_page_link_named_icon_choice_palette(
        &self,
        menu: PageLinkNamedIconChoiceMenu,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> Stateful<Div> {
        let slug = menu.slug;
        NotionNamedIconColor::ALL.chunks(5).enumerate().fold(
            click_away
                .member(
                    div()
                        .w(px(168.0))
                        .h(px(69.0))
                        .p(px(2.0))
                        .rounded(px(10.0))
                        .bg(rgb(self.theme.elevated_surface_bg))
                        .shadow(page_block_menu_shadow(self.appearance_mode)),
                    cx,
                )
                .id(ElementId::Name(
                    format!("notion-page-link-named-icon-choice-menu-{slug}").into(),
                ))
                .role(Role::Dialog)
                .aria_label("Select icon color")
                .flex()
                .flex_col()
                .gap(px(1.0)),
            |palette, (row_index, colors)| {
                palette.child(
                    self.render_page_link_named_icon_choice_row(&menu, row_index, colors, cx),
                )
            },
        )
    }

    fn render_page_link_named_icon_choice_row(
        &self,
        menu: &PageLinkNamedIconChoiceMenu,
        row_index: usize,
        colors: &[NotionNamedIconColor],
        cx: &mut App,
    ) -> Stateful<Div> {
        let slug = menu.slug;
        colors.iter().copied().fold(
            div()
                .id(ElementId::Name(
                    format!("notion-page-link-named-icon-choice-row-{slug}-{row_index}").into(),
                ))
                .role(Role::Row)
                .h(px(32.0))
                .flex()
                .gap(px(1.0)),
            |row, color| row.child(self.render_page_link_named_icon_choice(menu, color, cx)),
        )
    }

    fn render_page_link_named_icon_choice(
        &self,
        menu: &PageLinkNamedIconChoiceMenu,
        color: NotionNamedIconColor,
        cx: &mut App,
    ) -> Stateful<Div> {
        let slug = menu.slug;
        let PageLinkIconTarget { page_id, block_id } = &menu.target;
        let asset = NotionNamedIconAsset::new(slug, color, self.appearance_mode);
        let rendered_icon = self.resources.named_icon_image(asset, &self.notifier, cx);
        div()
            .id(ElementId::Name(
                format!(
                    "notion-page-link-named-icon-choice-{slug}-{}",
                    color.suffix()
                )
                .into(),
            ))
            .role(Role::GridCell)
            .aria_label(format!("{} {}", slug.label(), color.suffix()))
            .focusable()
            .tab_stop(true)
            .size(px(32.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_click(self.page_link_named_icon_choice_click(
                page_id.to_string(),
                block_id.to_string(),
                slug,
                color,
            ))
            .on_key_down(self.page_link_named_icon_choice_key_down(
                page_id.to_string(),
                block_id.to_string(),
                slug,
                color,
            ))
            .when_some(rendered_icon, |choice, rendered| {
                choice.child(img(rendered).size(px(24.0)))
            })
    }

    fn page_link_named_icon_choice_click(
        &self,
        page_id: String,
        block_id: String,
        slug: NotionNamedIconSlug,
        color: NotionNamedIconColor,
    ) -> impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::ChooseNamedIconColor {
                    target: PageLinkIconTarget {
                        page_id: page_id.clone(),
                        block_id: block_id.clone(),
                    },
                    slug,
                    color,
                }),
                window,
                cx,
            );
        })
    }

    fn page_link_named_icon_choice_key_down(
        &self,
        page_id: String,
        block_id: String,
        slug: NotionNamedIconSlug,
        color: NotionNamedIconColor,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::ChooseNamedIconColor {
                    target: PageLinkIconTarget {
                        page_id: page_id.clone(),
                        block_id: block_id.clone(),
                    },
                    slug,
                    color,
                }),
                window,
                cx,
            );
        })
    }
}
