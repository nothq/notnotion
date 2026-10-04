use super::{
    alpha, div, img, notion_page_body_muted, notion_page_body_text, point, px, relative, rgb, rgba,
    AnyElement, AppearanceMode, Arc, BoxShadow, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled,
    NOTION_PAGE_BODY_LEFT_GUTTER, NOTION_PAGE_BODY_TOP_INSET, NOTION_PAGE_BODY_WIDTH,
};
use crate::ui::PageShellIcon;
use crate::ui::PageShellLink;
use crate::ui::PageShellSnapshot;

mod host;
use super::PageShellIconRenderer;
use crate::ui::{view_actions::ViewActionSink, IconSet, Theme};
use gpui::App;
use host::PageShellBodyAction;

#[derive(Clone)]
pub(crate) struct PageShellBodyRenderer {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    page_icons: PageShellIconRenderer,
    snapshot: PageShellBodySnapshot,
    actions: ViewActionSink<PageShellBodyAction>,
}

#[derive(Clone)]
struct PageShellBodySnapshot {
    title: String,
    header_icon: Option<PageShellIcon>,
    top_controls_visible: bool,
    comment_page: Option<String>,
}

impl PageShellBodyRenderer {
    pub(crate) fn render_notion_page_main_pane(
        &self,
        page_shell: &PageShellSnapshot,
        cx: &mut App,
    ) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(self.render_notion_page_body(page_shell, cx))
    }

    pub(crate) fn render_notion_page_body(
        &self,
        page_shell: &PageShellSnapshot,
        cx: &mut App,
    ) -> AnyElement {
        (div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .child(
                div()
                    .pt(px(NOTION_PAGE_BODY_TOP_INSET))
                    .pl(px(NOTION_PAGE_BODY_LEFT_GUTTER))
                    .w(px(NOTION_PAGE_BODY_WIDTH))
                    .flex()
                    .flex_col()
                    .items_start()
                    .when(page_shell.page_icon_is_explicit, |this| {
                        this.child(self.render_notion_page_body_icon(&page_shell.page_icon, cx))
                    })
                    .child(self.render_notion_page_top_controls(cx))
                    .child(
                        div()
                            .pt(px(10.0))
                            .text_size(px(44.0))
                            .font_weight(FontWeight::BOLD)
                            .line_height(relative(1.1))
                            .text_color(rgb(notion_page_body_text(self.appearance_mode)))
                            .child(self.snapshot.title.clone()),
                    )
                    .children(
                        page_shell
                            .links
                            .iter()
                            .map(|link| self.render_notion_page_link(link, cx))
                            .collect::<Vec<_>>(),
                    ),
            ))
        .into_any_element()
    }

    pub(crate) fn standalone_header(
        &self,
        title: AnyElement,
        properties: Option<Div>,
        cx: &mut App,
    ) -> AnyElement {
        (div()
            .id("notion-page-header")
            .w_full()
            .items_start()
            .on_hover(self.actions.listener(|is_hovered: &bool, _, _| {
                PageShellBodyAction::TopControlsVisible(*is_hovered)
            }))
            .when_some(self.snapshot.header_icon.as_ref(), |this, icon| {
                this.child(self.render_notion_page_body_icon(icon, cx))
            })
            .child(self.render_notion_page_top_controls(cx))
            .child(div().mt(px(2.5)).w_full().px(px(8.0)).child(title))
            .when_some(properties, |this, properties| this.child(properties)))
        .into_any_element()
    }

    pub(crate) fn standalone_links(&self, page_shell: &PageShellSnapshot, cx: &mut App) -> Div {
        div()
            .mx(px(6.0))
            .pt(px(22.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(
                page_shell
                    .links
                    .iter()
                    .map(|link| self.render_notion_page_link(link, cx))
                    .collect::<Vec<_>>(),
            )
    }

    pub(crate) fn render_notion_page_body_icon(
        &self,
        icon: &PageShellIcon,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .pl(px(3.0))
            .pb(px(8.0))
            .child(self.page_icons.render(icon, 78.0, cx))
            .into_any_element()
    }

    pub(crate) fn render_notion_page_top_controls(&self, cx: &mut App) -> Div {
        let row = div()
            .ml(px(-1.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .gap(px(0.0))
            .child(self.render_notion_page_top_control("Add cover", cx))
            .child(self.render_notion_page_top_control("Add verification", cx))
            .child(self.render_notion_add_comment_top_control(cx));
        if self.snapshot.top_controls_visible {
            row
        } else {
            row.opacity(0.0)
        }
    }

    pub(crate) fn render_notion_page_top_control(&self, label: &str, cx: &mut App) -> Div {
        div()
            .h(px(28.0))
            .px(px(6.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    PageShellBodyAction::TopControlsVisible(true)
                }),
            )
            .child(img(self.icons.page.render(cx)).size(px(11.0)))
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(relative(1.2))
                    .text_color(rgb(notion_page_body_muted(self.appearance_mode)))
                    .child(label.to_string()),
            )
    }

    fn render_notion_add_comment_top_control(&self, cx: &mut App) -> Div {
        let target = self.snapshot.comment_page.clone();
        self.render_notion_page_top_control("Add comment", cx)
            .when_some(target, |control, page_id| {
                control.on_mouse_down(
                    MouseButton::Left,
                    self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        PageShellBodyAction::Comments(page_id.clone())
                    }),
                )
            })
    }

    pub(crate) fn render_notion_page_link(&self, link: &PageShellLink, cx: &mut App) -> Div {
        div()
            .h(px(28.0))
            .p(px(2.0))
            .flex()
            .items_center()
            .when_some(link.target_board_url.clone(), |this, board_url| {
                let label = link.title.clone();
                this.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    self.actions.listener(move |_: &MouseDownEvent, _, _| {
                        PageShellBodyAction::OpenWorkspace {
                            board_url: board_url.clone(),
                            label: label.clone(),
                        }
                    }),
                )
            })
            .child(
                div()
                    .size(px(24.0))
                    .mr(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.page_icons.render(
                        &link.icon,
                        if link.icon.kind == "emoji" {
                            19.8
                        } else {
                            22.0
                        },
                        cx,
                    )),
            )
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(relative(1.3))
                    .pb(px(1.0))
                    .text_color(rgb(notion_page_body_text(self.appearance_mode)))
                    .child(link.title.clone()),
            )
    }

    pub(crate) fn render_notion_page_create_menu(&self, cx: &mut App) -> AnyElement {
        let rows = [
            ("Page", &self.icons.page),
            ("AI Meeting Notes", &self.icons.property_person),
            ("Database", &self.icons.view_table_inactive),
            ("Templates", &self.icons.folder),
        ];

        (div()
            .absolute()
            .top(px(28.0))
            .left(px(112.0))
            .w(px(132.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(vec![BoxShadow {
                color: alpha(
                    0x000000,
                    if self.appearance_mode == AppearanceMode::Light {
                        0.12
                    } else {
                        0.28
                    },
                ),
                offset: point(px(0.0), px(18.0)),
                blur_radius: px(36.0),
                spread_radius: px(-12.0),
                inset: false,
            }])
            .pt(px(6.0))
            .pb(px(6.0))
            .children(rows.into_iter().enumerate().map(|(index, (label, icon))| {
                div()
                    .mx(px(4.0))
                    .h(px(28.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(index == 0, |this| {
                        this.bg(rgba(self.theme.command_menu_active_bg))
                    })
                    .child(img(icon.render(cx)).size(px(14.0)))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(self.theme.text_primary))
                            .child(label.to_string()),
                    )
            })))
        .into_any_element()
    }
}
