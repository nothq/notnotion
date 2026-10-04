use gpui::{
    div, px, AnyElement, App, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
};

use super::super::{ShareEvent, ShareMutation, ShareUpdate, SHARE_DIALOG_ROW_HEIGHT};
use super::ShareRowsView;
use crate::model::NotionUserId;
use crate::ui::{alpha, rgb, FluentBuilder};

#[derive(Clone, Copy)]
pub(super) struct AddPeopleRowState {
    pub(super) can_manage: bool,
    pub(super) can_mutate: bool,
    pub(super) open: bool,
}

impl AddPeopleRowState {
    fn aria_label(self) -> &'static str {
        if self.open {
            "Hide workspace users"
        } else {
            "Add workspace user"
        }
    }

    fn label(self) -> &'static str {
        match (self.can_manage, self.open) {
            (true, true) => "Hide people",
            (true, false) => "Add people",
            (false, _) => "Only Full access can share",
        }
    }
}

impl ShareRowsView {
    pub(super) fn render_notion_share_add_people_row(&self, cx: &mut App) -> AnyElement {
        let state = self.people;
        self.notion_share_add_people_row_shell(state, cx)
            .child(self.notion_share_avatar("+".into(), 0x16a34a))
            .child(self.notion_share_add_people_label(state))
            .into_any_element()
    }

    fn notion_share_add_people_row_shell(
        &self,
        state: AddPeopleRowState,
        _cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id("notion-share-add-people")
            .aria_label(state.aria_label())
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .rounded(px(7.0))
            .px(px(10.0))
            .when(state.can_mutate, |row| {
                row.role(Role::Button)
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(|_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            ShareEvent::Update(ShareUpdate::ToggleUserPicker)
                        }),
                    )
            })
            .when(state.can_manage && !state.can_mutate, |row| {
                row.opacity(0.45)
            })
            .flex()
            .items_center()
            .gap(px(10.0))
    }

    fn notion_share_add_people_label(&self, state: AddPeopleRowState) -> AnyElement {
        div()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(if state.can_mutate {
                self.theme.text_primary
            } else {
                self.theme.text_muted
            }))
            .child(state.label())
            .into_any_element()
    }

    pub(super) fn render_notion_share_candidate_row(
        &self,
        user_id: &NotionUserId,
        label: &SharedString,
        initial: &SharedString,
        _cx: &mut App,
    ) -> AnyElement {
        let candidate_id = user_id.clone();
        let can_mutate = self.people.can_mutate;
        div()
            .id(format!("notion-share-candidate-{}", user_id.as_str()))
            .aria_label(format!("Add {label} with view access"))
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .rounded(px(7.0))
            .px(px(10.0))
            .opacity(if can_mutate { 1.0 } else { 0.45 })
            .when(can_mutate, |row| {
                row.role(Role::Button)
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            ShareEvent::Mutate(ShareMutation::AddUser(candidate_id.clone()))
                        }),
                    )
            })
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.notion_share_avatar(initial.clone(), 0x6b7280))
            .child(self.notion_share_row_text(label.clone(), "Add with Can view".into()))
            .into_any_element()
    }
}
