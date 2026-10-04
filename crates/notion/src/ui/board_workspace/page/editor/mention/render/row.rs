use gpui::prelude::FluentBuilder;
use gpui::StyledImage;
use gpui::{
    img, AnyElement, App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, IntoElement,
    ObjectFit, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
};

use super::super::actions::PageMentionAction;
use super::super::state::PageMentionMenuIdentity;
use super::super::{PageMentionMenuRow, PAGE_MENTION_MENU_PAGE_ROW_HEIGHT};
use super::PageMentionRenderer;
use crate::model::NotionWorkspaceUser;
use crate::ui::{
    command_menu_label, command_menu_row_shell, command_menu_sublabel, div, mention_alarm_icon,
    mention_clock_icon, mention_invite_icon, px, rgb, COMMAND_MENU_ICON_SIZE, COMMAND_MENU_ROW_GAP,
    COMMAND_MENU_SUBLABEL_SIZE,
};

pub(super) struct PageMentionRowSpec<'a> {
    pub(super) identity: PageMentionMenuIdentity,
    pub(super) index: usize,
    pub(super) row: &'a PageMentionMenuRow,
    pub(super) selected: bool,
}

impl PageMentionRenderer {
    pub(super) fn render_row(&self, spec: PageMentionRowSpec<'_>, cx: &mut App) -> AnyElement {
        let click_identity = spec.identity.clone();
        let hover_identity = spec.identity;
        let clicked_row = spec.row.clone();
        let index = spec.index;
        let mut row = command_menu_row_shell(self.resources.theme, spec.selected)
            .when(spec.row.is_two_line(), |row| {
                row.h(px(PAGE_MENTION_MENU_PAGE_ROW_HEIGHT))
                    .py(px(4.0))
                    .items_start()
            })
            .id(ElementId::Name(
                format!("notion-native-mention-{}-{index}", click_identity.block_id).into(),
            ))
            .role(Role::ListBoxOption)
            .aria_label(SharedString::from(spec.row.label()))
            .on_click({
                let actions = self.actions.clone();
                move |_: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    actions.emit(
                        PageMentionAction::ActivateMenuRow {
                            identity: click_identity.clone(),
                            row: Box::new(clicked_row.clone()),
                        },
                        window,
                        cx,
                    );
                }
            })
            .child(self.render_row_body(spec.row, cx));
        let actions = self.actions.clone();
        row.interactivity()
            .on_hover(move |hovered: &bool, window, cx| {
                if !*hovered {
                    return;
                }
                actions.emit(
                    PageMentionAction::HoverMenuRow {
                        identity: hover_identity.clone(),
                        index,
                    },
                    window,
                    cx,
                );
            });
        row.into_any_element()
    }

    fn render_row_body(&self, row: &PageMentionMenuRow, cx: &mut App) -> Div {
        let body = div()
            .flex()
            .items_center()
            .gap(px(COMMAND_MENU_ROW_GAP))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .child(
                div()
                    .size(px(COMMAND_MENU_ICON_SIZE))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_row_icon(row, cx)),
            );
        match row {
            PageMentionMenuRow::Date {
                label, sublabel, ..
            } => body
                .child(command_menu_label(self.resources.theme, label.clone()))
                .when_some(sublabel.clone(), |body, sublabel| {
                    body.child(command_menu_sublabel(self.resources.theme, sublabel))
                }),
            PageMentionMenuRow::Reminder { sublabel, .. } => body
                .child(command_menu_label(self.resources.theme, "Remind me"))
                .child(command_menu_sublabel(
                    self.resources.theme,
                    sublabel.clone(),
                )),
            PageMentionMenuRow::Person { user } => self.render_person(body, user),
            PageMentionMenuRow::Invite { .. } | PageMentionMenuRow::NewPage { .. } => {
                body.child(command_menu_label(self.resources.theme, row.label()))
            }
            PageMentionMenuRow::Page { result } => {
                self.render_page(body, result.title.clone(), result.highlight.clone())
            }
        }
    }

    fn render_person(&self, body: Div, user: &NotionWorkspaceUser) -> Div {
        body.child(command_menu_label(self.resources.theme, user.name.clone()))
            .when(user.is_current_user, |body| {
                body.child(
                    div()
                        .ml(px(4.0))
                        .text_size(px(14.0))
                        .text_color(rgb(self.resources.theme.menu_secondary_text))
                        .child("(You)"),
                )
            })
    }

    fn render_page(&self, body: Div, title: String, highlight: Option<String>) -> Div {
        body.items_start().child(
            div()
                .flex()
                .flex_col()
                .min_w(px(0.0))
                .child(command_menu_label(self.resources.theme, title))
                .when_some(
                    highlight.filter(|caption| !caption.is_empty()),
                    |column, caption| {
                        column.child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(COMMAND_MENU_SUBLABEL_SIZE))
                                .line_height(px(16.8))
                                .text_color(rgb(self.resources.theme.text_hint))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(caption),
                        )
                    },
                ),
        )
    }

    fn render_row_icon(&self, row: &PageMentionMenuRow, cx: &mut App) -> AnyElement {
        let stroke = self.resources.theme.text_primary;
        match row {
            PageMentionMenuRow::Date { .. } => img(mention_clock_icon(stroke))
                .size(px(COMMAND_MENU_ICON_SIZE))
                .into_any_element(),
            PageMentionMenuRow::Reminder { .. } => img(mention_alarm_icon(stroke))
                .size(px(COMMAND_MENU_ICON_SIZE))
                .into_any_element(),
            PageMentionMenuRow::Person { user } => self.render_avatar(user, cx),
            PageMentionMenuRow::Invite { .. } => img(mention_invite_icon(stroke))
                .size(px(COMMAND_MENU_ICON_SIZE))
                .into_any_element(),
            PageMentionMenuRow::Page { result } => {
                self.page_icons()
                    .render(&result.icon, COMMAND_MENU_ICON_SIZE, cx)
            }
            PageMentionMenuRow::NewPage { .. } => img(self.resources.icons.page.render(cx))
                .size(px(COMMAND_MENU_ICON_SIZE))
                .into_any_element(),
        }
    }

    fn render_avatar(&self, user: &NotionWorkspaceUser, cx: &mut App) -> AnyElement {
        let photo = user.profile_photo.as_deref().and_then(|url| {
            self.resources
                .notion_resources
                .external_icon_image(url, &self.resources.notifier, cx)
        });
        let frame = div()
            .size(px(COMMAND_MENU_ICON_SIZE))
            .rounded_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center();
        match photo {
            Some(photo) => frame
                .child(
                    img(photo)
                        .size(px(COMMAND_MENU_ICON_SIZE))
                        .object_fit(ObjectFit::Cover),
                )
                .into_any_element(),
            None => frame
                .bg(rgb(0x2383e2))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xffffff))
                .child(
                    user.name
                        .chars()
                        .next()
                        .map(|initial| initial.to_uppercase().to_string())
                        .unwrap_or_default(),
                )
                .into_any_element(),
        }
    }
}
