use gpui::{
    div, px, AnyElement, App, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled,
};

use super::{
    ShareDialogView, ShareEvent, ShareMutation, SHARE_DIALOG_HEADER_HEIGHT,
    SHARE_ROLE_PICKER_ROW_HEIGHT, SHARE_ROLE_PICKER_WIDTH,
};
use crate::model::{NotionPublicShareRole, NotionShareRole, NotionUserId};
use crate::ui::surface::NotionShareRolePickerTarget;
use crate::ui::{alpha, rgb, rgba, FluentBuilder};

impl ShareDialogView<'_> {
    pub(super) fn render_notion_share_role_picker(
        &self,
        target: &NotionShareRolePickerTarget,
        cx: &mut App,
    ) -> AnyElement {
        let row_count = match target {
            NotionShareRolePickerTarget::User { .. } => 5,
            NotionShareRolePickerTarget::PublicLink { .. } => 4,
        };
        let picker = self.notion_share_role_picker_shell(row_count, cx);
        match target {
            NotionShareRolePickerTarget::User {
                user_id,
                current_role,
            } => self.render_notion_user_role_picker(picker, user_id, *current_role, cx),
            NotionShareRolePickerTarget::PublicLink { current_role } => {
                self.render_notion_public_role_picker(picker, *current_role, cx)
            }
        }
        .into_any_element()
    }

    fn notion_share_role_picker_shell(
        &self,
        row_count: usize,
        _cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id("notion-share-role-picker")
            .role(Role::Dialog)
            .aria_label("Access role")
            .absolute()
            .top(px(SHARE_DIALOG_HEADER_HEIGHT - 4.0))
            .right(px(10.0))
            .w(px(SHARE_ROLE_PICKER_WIDTH))
            .h(px(row_count as f32 * SHARE_ROLE_PICKER_ROW_HEIGHT + 8.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .p(px(4.0))
            .flex()
            .flex_col()
    }

    fn render_notion_user_role_picker(
        &self,
        picker: gpui::Stateful<gpui::Div>,
        user_id: &NotionUserId,
        current_role: NotionShareRole,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        picker
            .children(
                [
                    NotionShareRole::Reader,
                    NotionShareRole::Commenter,
                    NotionShareRole::Editor,
                    NotionShareRole::FullAccess,
                ]
                .into_iter()
                .map(|role| {
                    self.render_notion_user_share_role_option(user_id, current_role, role, cx)
                }),
            )
            .child(self.render_notion_remove_share_user_option(user_id, current_role, cx))
    }

    fn render_notion_public_role_picker(
        &self,
        picker: gpui::Stateful<gpui::Div>,
        current_role: NotionPublicShareRole,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        picker
            .children(
                [
                    NotionPublicShareRole::Reader,
                    NotionPublicShareRole::Commenter,
                    NotionPublicShareRole::Editor,
                ]
                .into_iter()
                .map(|role| self.render_notion_public_share_role_option(current_role, role, cx)),
            )
            .child(self.render_notion_disable_public_share_option(current_role, cx))
    }

    fn render_notion_user_share_role_option(
        &self,
        user_id: &NotionUserId,
        current_role: NotionShareRole,
        role: NotionShareRole,
        _cx: &mut App,
    ) -> AnyElement {
        let user_id = user_id.clone();
        self.notion_share_role_option(
            format!("notion-share-user-role-{}", role.as_notion_str()),
            role.label(),
            current_role == role,
            (current_role != role).then(|| {
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ShareEvent::Mutate(ShareMutation::ChangeUserRole {
                        user_id: user_id.clone(),
                        current: current_role,
                        role,
                    })
                })
            }),
        )
    }

    fn render_notion_remove_share_user_option(
        &self,
        user_id: &NotionUserId,
        current_role: NotionShareRole,
        _cx: &mut App,
    ) -> AnyElement {
        let user_id = user_id.clone();
        self.notion_share_role_option(
            "notion-share-remove-user".to_string(),
            "Remove access",
            false,
            Some(self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                ShareEvent::Mutate(ShareMutation::RemoveUser {
                    user_id: user_id.clone(),
                    current: current_role,
                })
            })),
        )
    }

    fn render_notion_public_share_role_option(
        &self,
        current_role: NotionPublicShareRole,
        role: NotionPublicShareRole,
        _cx: &mut App,
    ) -> AnyElement {
        self.notion_share_role_option(
            format!("notion-share-public-role-{}", role.as_notion_str()),
            role.label(),
            current_role == role,
            (current_role != role).then(|| {
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ShareEvent::Mutate(ShareMutation::ChangePublicRole {
                        current: current_role,
                        role,
                    })
                })
            }),
        )
    }

    fn render_notion_disable_public_share_option(
        &self,
        current_role: NotionPublicShareRole,
        _cx: &mut App,
    ) -> AnyElement {
        self.notion_share_role_option(
            "notion-share-disable-public".to_string(),
            "Turn off public link",
            false,
            Some(self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                ShareEvent::Mutate(ShareMutation::DisablePublic(current_role))
            })),
        )
    }

    fn notion_share_role_option(
        &self,
        id: String,
        label: &'static str,
        selected: bool,
        listener: Option<impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static>,
    ) -> AnyElement {
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .h(px(SHARE_ROLE_PICKER_ROW_HEIGHT))
            .flex_none()
            .rounded(px(5.0))
            .px(px(8.0))
            .when_some(listener, |row, listener| {
                row.cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(MouseButton::Left, listener)
            })
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(12.0))
            .text_color(rgb(self.theme.text_primary))
            .child(label)
            .when(selected, |row| row.child("✓"))
            .into_any_element()
    }
}
