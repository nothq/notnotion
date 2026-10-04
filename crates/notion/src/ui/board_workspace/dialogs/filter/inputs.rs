use crate::ui::board_workspace::dialogs::filter::prelude::*;

impl DatabaseFilterRenderer<'_> {
    pub(super) fn database_filter_property_input(&self, cx: &mut App) -> gpui::Entity<TextInput> {
        let change_actions = self.resources.actions.clone();
        let submit_actions = self.resources.actions.clone();
        let up_actions = self.resources.actions.clone();
        let down_actions = self.resources.actions.clone();
        let on_change: TextInputChange = Rc::new(move |value, window, cx| {
            change_actions.emit(DatabaseFilterAction::SetPropertyQuery(value), window, cx);
        });
        let on_submit: TextInputAction = Rc::new(move |window, cx| {
            submit_actions.emit(DatabaseFilterAction::ActivatePropertyHighlight, window, cx);
        });
        let on_up: TextInputAction = Rc::new(move |window, cx| {
            up_actions.emit(DatabaseFilterAction::MovePropertyHighlight(-1), window, cx);
        });
        let on_down: TextInputAction = Rc::new(move |window, cx| {
            down_actions.emit(DatabaseFilterAction::MovePropertyHighlight(1), window, cx);
        });
        let on_escape = database_filter_escape_action(
            self.resources.actions.clone(),
            DatabaseFilterAction::Dismiss(self.resources.host),
        );
        let props = TextInputProps::single_line(self.state.property_query.clone())
            .placeholder("Filter by…")
            .request_focus(self.state.property_focus_requested.replace(false))
            .style(Self::database_filter_input_style(
                self.resources.appearance_mode,
                self.resources.theme,
            ))
            .accessibility("notion-database-filter-property-input", "Filter by…")
            .on_change(on_change)
            .on_submit(on_submit)
            .on_escape(on_escape)
            .on_up(on_up)
            .on_down(on_down);
        if let Some(input) = self.state.property_input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.state
            .property_input
            .borrow_mut()
            .replace(input.clone());
        input
    }

    pub(super) fn database_filter_value_input(&self, cx: &mut App) -> gpui::Entity<TextInput> {
        let draft = self
            .state
            .draft
            .as_ref()
            .expect("filter value input requires a draft");
        let change_actions = self.resources.actions.clone();
        let on_change: TextInputChange = Rc::new(move |value, window, cx| {
            change_actions.emit(DatabaseFilterAction::SetValue(value), window, cx);
        });
        let on_escape = database_filter_escape_action(
            self.resources.actions.clone(),
            DatabaseFilterAction::CloseValueEditor(self.resources.host),
        );
        let (value, placeholder) = database_filter_value_input_content(self.state, draft);
        let props = TextInputProps::single_line(value)
            .placeholder(placeholder)
            .request_focus(self.state.value_focus_requested.replace(false))
            .style(Self::database_filter_value_input_style(
                self.resources.theme,
            ))
            .accessibility("notion-database-filter-value-input", placeholder)
            .on_change(on_change)
            .on_escape(on_escape);
        if let Some(input) = self.state.value_input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.state.value_input.borrow_mut().replace(input.clone());
        input
    }

    fn database_filter_input_style(
        appearance_mode: AppearanceMode,
        theme: Theme,
    ) -> TextInputStyle {
        TextInputStyle {
            height: px(28.0),
            min_height: px(28.0),
            padding_x: px(10.0),
            padding_y: px(4.0),
            radius: px(6.0),
            background: rgb(if appearance_mode == AppearanceMode::Light {
                0xf7f6f3
            } else {
                0x2b2b2b
            })
            .into(),
            border: alpha(theme.text_primary, 0.0),
            focused_border: rgb(0x2383e2).into(),
            text: rgb(theme.text_primary).into(),
            placeholder: rgb(theme.text_hint).into(),
            selection: alpha(0x2383e2, 0.28),
            caret: rgb(theme.text_primary).into(),
            font_size: px(14.0),
            line_height: px(20.0),
            font_family: None,
        }
    }

    fn database_filter_value_input_style(theme: Theme) -> TextInputStyle {
        TextInputStyle {
            height: px(28.0),
            min_height: px(28.0),
            padding_x: px(6.0),
            padding_y: px(4.0),
            radius: px(4.0),
            background: alpha(theme.text_primary, 0.0),
            border: alpha(theme.text_primary, 0.12),
            focused_border: rgb(0x2383e2).into(),
            text: rgb(theme.text_primary).into(),
            placeholder: rgb(theme.text_hint).into(),
            selection: alpha(0x2383e2, 0.28),
            caret: rgb(theme.text_primary).into(),
            font_size: px(14.0),
            line_height: px(20.0),
            font_family: None,
        }
    }
}

fn database_filter_escape_action(
    actions: ViewActionSink<DatabaseFilterAction>,
    action: DatabaseFilterAction,
) -> TextInputAction {
    Rc::new(move |window, cx| actions.emit(action.clone(), window, cx))
}

fn database_filter_value_input_content(
    filter: &DatabaseFilterUiState,
    draft: &DatabaseFilterDraft,
) -> (String, &'static str) {
    match draft.property.filter_type() {
        "person" => (filter.value_query.clone(), "Search for one or more people…"),
        "relation" => (filter.value_query.clone(), "Search for one or more pages…"),
        "select" | "multi_select" => (filter.value_query.clone(), "Select one or more options…"),
        "number" => (draft.value.clone(), "Type a number…"),
        _ => (draft.value.clone(), "Type a value…"),
    }
}
