use std::rc::Rc;

use gpui::{App, AppContext, Entity};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputKeyAction, TextInputProps, TextInputStyle,
};

use crate::ui::{alpha, px, rgb, surface::NotionSearchState, view_actions::ViewActionSink, Theme};

use super::QuickFindAction;

pub(super) struct QuickFindInputConfig {
    pub(super) query: String,
    pub(super) workspace_name: String,
    pub(super) request_focus: bool,
    pub(super) loading: bool,
    pub(super) theme: Theme,
}

impl NotionSearchState {
    pub(super) fn input_entity(
        &self,
        config: QuickFindInputConfig,
        actions: ViewActionSink<QuickFindAction>,
        cx: &mut App,
    ) -> Entity<TextInput> {
        let props = quick_find_input_props(config, actions);
        if let Some(input) = self.input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        *self.input.borrow_mut() = Some(input.clone());
        input
    }
}

fn quick_find_input_props(
    config: QuickFindInputConfig,
    actions: ViewActionSink<QuickFindAction>,
) -> TextInputProps {
    let (height, padding_y, font_size, line_height) = if config.loading {
        (52.0, 0.0, 18.0, 28.0)
    } else {
        (48.0, 14.0, 16.0, 18.0)
    };
    TextInputProps::single_line(config.query)
        .placeholder(format!(
            "Search or ask a question in {}...",
            config.workspace_name
        ))
        .request_focus(config.request_focus)
        .style(quick_find_input_style(
            config.theme,
            height,
            padding_y,
            font_size,
            line_height,
        ))
        .on_change(quick_find_on_change(actions.clone()))
        .on_submit_with_state(quick_find_on_submit(actions.clone()))
        .on_escape(quick_find_action(actions.clone(), QuickFindAction::Close))
        .on_up(quick_find_action(
            actions.clone(),
            QuickFindAction::MoveSelection(-1),
        ))
        .on_down(quick_find_action(
            actions,
            QuickFindAction::MoveSelection(1),
        ))
}

fn quick_find_input_style(
    theme: Theme,
    height: f32,
    padding_y: f32,
    font_size: f32,
    line_height: f32,
) -> TextInputStyle {
    TextInputStyle {
        height: px(height),
        min_height: px(height),
        padding_x: px(0.0),
        padding_y: px(padding_y),
        radius: px(0.0),
        background: alpha(theme.elevated_surface_bg, 0.0),
        border: alpha(theme.elevated_surface_bg, 0.0),
        focused_border: alpha(theme.elevated_surface_bg, 0.0),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_muted).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(font_size),
        line_height: px(line_height),
        font_family: None,
    }
}

fn quick_find_on_change(actions: ViewActionSink<QuickFindAction>) -> TextInputChange {
    Rc::new(move |value, window, cx| {
        actions.emit(QuickFindAction::QueryChanged(value), window, cx);
    })
}

fn quick_find_on_submit(actions: ViewActionSink<QuickFindAction>) -> TextInputKeyAction {
    Rc::new(move |_snapshot, modifiers, window, cx| {
        actions.emit(
            QuickFindAction::Submit {
                open_in_new_tab: modifiers.platform,
            },
            window,
            cx,
        );
    })
}

fn quick_find_action(
    actions: ViewActionSink<QuickFindAction>,
    action: QuickFindAction,
) -> TextInputAction {
    Rc::new(move |window, cx| actions.emit(action.clone(), window, cx))
}
