use std::{rc::Rc, sync::Arc};

use gpui::{
    div, px, AnyElement, App, AppContext, Entity, InteractiveElement, IntoElement, ParentElement,
    Styled,
};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

use super::{ShareDialogView, ShareEvent, ShareUpdate, SHARE_DIALOG_ROW_HEIGHT};
use crate::ui::{alpha, rgb};

impl ShareDialogView<'_> {
    pub(super) fn render_notion_share_member_search(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-share-member-search")
            .h(px(SHARE_DIALOG_ROW_HEIGHT))
            .flex_none()
            .px(px(12.0))
            .flex()
            .items_center()
            .child(self.notion_share_member_search_input_entity(cx))
            .into_any_element()
    }

    fn notion_share_member_search_input_entity(&self, cx: &mut App) -> Entity<TextInput> {
        let dialog = self.dialog;
        let props = self.notion_share_member_search_input_props(
            dialog.user_picker_session.clone(),
            dialog.user_query.clone(),
            cx,
        );
        if let Some(input) = dialog.user_query_input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        *dialog.user_query_input.borrow_mut() = Some(input.clone());
        input
    }

    fn notion_share_member_search_input_props(
        &self,
        picker_session: Arc<()>,
        query: String,
        cx: &mut App,
    ) -> TextInputProps {
        TextInputProps::single_line(query)
            .placeholder("Search workspace members")
            .request_focus(true)
            .style(TextInputStyle {
                height: px(36.0),
                min_height: px(36.0),
                padding_x: px(10.0),
                padding_y: px(0.0),
                radius: px(7.0),
                background: alpha(self.theme.text_primary, 0.04),
                border: alpha(self.theme.text_primary, 0.10),
                focused_border: alpha(0x2383e2, 0.55),
                text: rgb(self.theme.text_primary).into(),
                placeholder: rgb(self.theme.text_muted).into(),
                selection: alpha(0x2383e2, 0.28),
                caret: rgb(self.theme.text_primary).into(),
                font_size: px(13.0),
                line_height: px(20.0),
                font_family: None,
            })
            .on_change(self.notion_share_member_search_on_change(picker_session.clone(), cx))
            .on_escape(self.notion_share_member_search_on_escape(picker_session, cx))
    }

    fn notion_share_member_search_on_change(
        &self,
        session: Arc<()>,
        _cx: &mut App,
    ) -> TextInputChange {
        let actions = self.actions.clone();
        Rc::new(move |value, window, cx| {
            actions.emit(
                ShareEvent::Update(ShareUpdate::ChangeQuery {
                    session: session.clone(),
                    value,
                }),
                window,
                cx,
            );
        })
    }

    fn notion_share_member_search_on_escape(
        &self,
        session: Arc<()>,
        _cx: &mut App,
    ) -> TextInputAction {
        let actions = self.actions.clone();
        Rc::new(move |window, cx| {
            actions.emit(
                ShareEvent::Update(ShareUpdate::CloseUserPicker {
                    session: session.clone(),
                }),
                window,
                cx,
            );
        })
    }
}
