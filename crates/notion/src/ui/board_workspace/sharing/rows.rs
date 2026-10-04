use gpui::{
    div, px, AnyElement, App, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
};

use super::{ShareEvent, ShareMutation, ShareUpdate, SHARE_DIALOG_ROW_HEIGHT};
use crate::model::{NotionShareRole, NotionUserId};
use crate::ui::surface::{
    NotionShareDialogPublicLink, NotionShareDialogRow, NotionShareDialogState,
    NotionShareRolePickerTarget,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, rgb, FluentBuilder, Theme};

mod people;

use people::AddPeopleRowState;

/// The name, avatar initial, and access detail a user's row shows.
struct ShareUserRowText<'a> {
    label: &'a SharedString,
    initial: &'a SharedString,
    detail: &'a SharedString,
}

#[derive(Clone)]
pub(super) struct ShareRowsView {
    theme: Theme,
    people: AddPeopleRowState,
    actions: ViewActionSink<ShareEvent>,
}

impl ShareRowsView {
    pub(super) fn new(
        dialog: &NotionShareDialogState,
        theme: Theme,
        actions: ViewActionSink<ShareEvent>,
    ) -> Self {
        Self {
            theme,
            people: AddPeopleRowState {
                can_manage: dialog.loaded && dialog.can_manage_sharing,
                can_mutate: dialog.can_mutate(),
                open: dialog.user_picker_open,
            },
            actions,
        }
    }
}

impl ShareRowsView {
    pub(super) fn render_notion_share_dialog_row(
        &self,
        row: &NotionShareDialogRow,
        cx: &mut App,
    ) -> AnyElement {
        match row {
            NotionShareDialogRow::PublicLink { state } => {
                self.render_notion_public_share_row(state, cx)
            }
            NotionShareDialogRow::UserPermission {
                user_id,
                label,
                initial,
                detail,
                role,
            } => self.render_notion_user_share_row(
                user_id,
                ShareUserRowText {
                    label,
                    initial,
                    detail,
                },
                *role,
                cx,
            ),
            NotionShareDialogRow::ReadOnlyPermission {
                stable_id,
                label,
                detail,
            } => self.render_notion_read_only_share_row(stable_id, label, detail),
            NotionShareDialogRow::AddPeople => self.render_notion_share_add_people_row(cx),
            NotionShareDialogRow::CandidateUser {
                user_id,
                label,
                initial,
            } => self.render_notion_share_candidate_row(user_id, label, initial, cx),
            NotionShareDialogRow::NoCandidateUsers { query_active } => div()
                .id("notion-share-no-candidate-users")
                .h(px(SHARE_DIALOG_ROW_HEIGHT))
                .px(px(10.0))
                .flex()
                .items_center()
                .text_size(px(12.0))
                .text_color(rgb(self.theme.text_muted))
                .child(if *query_active {
                    "No workspace members match"
                } else {
                    "All visible workspace users already have access"
                })
                .into_any_element(),
        }
    }

    fn render_notion_public_share_row(
        &self,
        state: &NotionShareDialogPublicLink,
        _cx: &mut App,
    ) -> AnyElement {
        let can_mutate = self.people.can_mutate;
        let (detail, current_role, read_only) = match state {
            NotionShareDialogPublicLink::Disabled => {
                (SharedString::from("Not shared to web"), None, false)
            }
            NotionShareDialogPublicLink::Enabled(role) => {
                (SharedString::from(role.label()), Some(*role), false)
            }
            NotionShareDialogPublicLink::ReadOnly(detail) => (detail.clone(), None, true),
        };
        div()
            .id("notion-share-public-link")
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .min_w(px(0.0))
            .rounded(px(7.0))
            .px(px(10.0))
            .when(can_mutate && !read_only, |row| {
                row.role(Role::Button)
                    .aria_label("Public link access")
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            match current_role {
                                Some(current_role) => {
                                    ShareEvent::Update(ShareUpdate::OpenRolePicker(
                                        NotionShareRolePickerTarget::PublicLink { current_role },
                                    ))
                                }
                                None => ShareEvent::Mutate(ShareMutation::EnablePublic),
                            }
                        }),
                    )
            })
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.notion_share_avatar("↗".into(), 0x3b82f6))
            .child(self.notion_share_row_text("Anyone with the link".into(), detail))
            .when(can_mutate && !read_only, |row| {
                row.child(
                    div()
                        .flex_none()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child("›"),
                )
            })
            .into_any_element()
    }

    fn render_notion_user_share_row(
        &self,
        user_id: &NotionUserId,
        text: ShareUserRowText<'_>,
        role: NotionShareRole,
        _cx: &mut App,
    ) -> AnyElement {
        let ShareUserRowText {
            label,
            initial,
            detail,
        } = text;
        let can_mutate = self.people.can_mutate;
        let picker_user_id = user_id.clone();
        div()
            .id(format!("notion-share-user-{}", user_id.as_str()))
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .min_w(px(0.0))
            .rounded(px(7.0))
            .px(px(10.0))
            .when(can_mutate, |row| {
                row.role(Role::Button)
                    .aria_label(format!("Change access for {label}"))
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            ShareEvent::Update(ShareUpdate::OpenRolePicker(
                                NotionShareRolePickerTarget::User {
                                    user_id: picker_user_id.clone(),
                                    current_role: role,
                                },
                            ))
                        }),
                    )
            })
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.notion_share_avatar(initial.clone(), 0x6b7280))
            .child(self.notion_share_row_text(label.clone(), detail.clone()))
            .when(can_mutate, |row| {
                row.child(
                    div()
                        .flex_none()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child("›"),
                )
            })
            .into_any_element()
    }

    fn render_notion_read_only_share_row(
        &self,
        stable_id: &SharedString,
        label: &SharedString,
        detail: &SharedString,
    ) -> AnyElement {
        div()
            .id(format!("notion-share-read-only-{stable_id}"))
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .min_w(px(0.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.notion_share_avatar("•".into(), 0x78716c))
            .child(self.notion_share_row_text(label.clone(), detail.clone()))
            .child(
                div()
                    .flex_none()
                    .text_size(px(10.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("Read only"),
            )
            .into_any_element()
    }

    fn notion_share_avatar(&self, label: SharedString, color: u32) -> AnyElement {
        div()
            .size(px(28.0))
            .flex_none()
            .rounded_full()
            .bg(rgb(color))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0xffffff))
            .child(label)
            .into_any_element()
    }

    fn notion_share_row_text(&self, label: SharedString, detail: SharedString) -> AnyElement {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(13.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child(label),
            )
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(11.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(detail),
            )
            .into_any_element()
    }
}
