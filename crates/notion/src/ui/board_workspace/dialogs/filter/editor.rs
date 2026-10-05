use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

/// Each value choice with whether the draft selects it.
type SelectableFilterChoices = Arc<[(DatabaseFilterChoice, bool)]>;

impl DatabaseFilterRenderer<'_> {
    pub(super) fn render_database_filter_editor(
        &self,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let draft = self
            .state
            .draft
            .as_ref()
            .expect("filter editor requires a draft");
        let operator = draft.operator;
        if operator.requires_value() && !operator.uses_text_input() {
            return self.render_database_picker_filter_editor(placement, draft, cx);
        }
        let value_input = operator
            .requires_value()
            .then(|| self.database_filter_value_input(cx));
        let more_actions = draft.present_in_effective_state.then(|| {
            self.state
                .render_database_filter_more_actions_button(&self.resources)
        });
        let header = self.render_database_text_filter_header(draft, more_actions);
        self.database_text_filter_dialog(placement, operator, cx)
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(5.0))
            .child(header)
            .when_some(value_input, |editor, input| {
                editor.child(
                    div()
                        .w(px(196.0))
                        .h(px(28.0))
                        .mx(px(4.0))
                        .flex_none()
                        .child(input),
                )
            })
            .when(!operator.requires_value(), |editor| {
                editor.child(div().w(px(196.0)).h(px(28.0)).mx(px(4.0)).flex_none())
            })
            .into_any_element()
    }

    fn database_text_filter_dialog(
        &self,
        placement: DatabaseFilterDialogPlacement,
        operator: DatabaseTextFilterOperator,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let action = DatabaseFilterAction::Dismiss(self.resources.host);
        self.resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(
                    if operator.uses_text_input() {
                        FILTER_TEXT_EDITOR_WIDTH
                    } else {
                        FILTER_EDITOR_WIDTH
                    },
                    FILTER_EDITOR_HEIGHT,
                ),
                DatabaseFilterDialogBehavior::new(
                    self.state.editor_anchor,
                    (matches!(placement, DatabaseFilterDialogPlacement::FullPage)
                        && self.state.stage == DatabaseFilterDialogStage::Editor)
                        .then_some(DatabaseFilterOutsideAction::CloseToolbar),
                ),
                cx,
            )
            .when(!operator.requires_value(), |dialog| {
                dialog.on_key_down(self.escape_key_listener(action))
            })
    }

    fn render_database_text_filter_header(
        &self,
        draft: &DatabaseFilterDraft,
        more_actions: Option<AnyElement>,
    ) -> Div {
        let theme = self.resources.theme;
        div()
            .h(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .child(
                div()
                    .h_full()
                    .max_w(px(98.0))
                    .pl(px(4.0))
                    .pr(px(2.0))
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(rgb(theme.text_secondary))
                    .child(draft.property.label.clone()),
            )
            .child(self.render_database_text_filter_operator(draft.operator))
            .child(div().flex_grow(1.0))
            .when_some(more_actions, |header, actions| header.child(actions))
    }

    fn render_database_text_filter_operator(&self, operator: DatabaseTextFilterOperator) -> Div {
        let actions = self.resources.actions.clone();
        let current_anchor = self.state.operator_anchor;
        div()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first().copied() else {
                    return;
                };
                if current_anchor == Some(bounds) {
                    return;
                }
                actions.emit(
                    DatabaseFilterAction::MeasureAnchor {
                        kind: DatabaseFilterAnchorKind::Operator,
                        bounds,
                    },
                    window,
                    cx,
                );
                window.refresh();
            })
            .h_full()
            .child(self.render_database_text_filter_operator_button(operator))
    }

    fn render_database_text_filter_operator_button(
        &self,
        operator: DatabaseTextFilterOperator,
    ) -> gpui::Stateful<Div> {
        let theme = self.resources.theme;
        let action = DatabaseFilterAction::OpenOperatorPicker;
        div()
            .id("notion-database-filter-operator")
            .h_full()
            .px(px(2.0))
            .rounded(px(4.0))
            .role(Role::Button)
            .aria_label(operator.label())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .gap(px(3.0))
            .text_size(px(12.0))
            .text_color(rgb(theme.text_secondary))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .child(operator.header_label())
            .child(database_filter_dropdown_mark(theme))
    }

    fn render_database_picker_filter_editor(
        &self,
        placement: DatabaseFilterDialogPlacement,
        draft: &DatabaseFilterDraft,
        cx: &mut App,
    ) -> AnyElement {
        let height = self.state.picker_editor_height(draft);
        let header = self
            .state
            .render_database_picker_filter_header(&self.resources, draft);
        let content = self.render_database_picker_filter_content(draft, height, cx);
        let escape_action = if matches!(
            self.state.stage,
            DatabaseFilterDialogStage::DateModePicker | DatabaseFilterDialogStage::DateValuePicker
        ) {
            DatabaseFilterAction::SetStage(DatabaseFilterDialogStage::Editor)
        } else {
            DatabaseFilterAction::Dismiss(self.resources.host)
        };

        self.resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(FILTER_EDITOR_WIDTH, height),
                DatabaseFilterDialogBehavior::new(
                    self.state.editor_anchor,
                    (matches!(placement, DatabaseFilterDialogPlacement::FullPage)
                        && self.state.stage == DatabaseFilterDialogStage::Editor)
                        .then_some(DatabaseFilterOutsideAction::CloseToolbar),
                ),
                cx,
            )
            .on_key_down(self.escape_key_listener(escape_action))
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(5.0))
            .child(header)
            .child(content)
            .into_any_element()
    }

    fn render_database_picker_filter_content(
        &self,
        draft: &DatabaseFilterDraft,
        height: f32,
        cx: &mut App,
    ) -> AnyElement {
        match draft.property.filter_type() {
            "date" => self
                .state
                .render_database_date_filter_editor(&self.resources, draft),
            "checkbox" | "status" => {
                let choices = self.state.choices_for_draft(draft);
                div()
                    .flex_grow(1.0)
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .children(choices.iter().enumerate().map(|(index, choice)| {
                        self.resources.render_database_filter_choice_row(
                            index,
                            choice,
                            self.state
                                .choice_is_selected(choice.key.as_deref().unwrap_or_default()),
                            cx,
                        )
                    }))
                    .into_any_element()
            }
            "person" | "relation" | "select" | "multi_select" => {
                self.render_database_filter_search_choices(draft, height, cx)
            }
            _ => div().h(px(28.0)).into_any_element(),
        }
    }

    fn render_database_filter_search_choices(
        &self,
        draft: &DatabaseFilterDraft,
        height: f32,
        cx: &mut App,
    ) -> AnyElement {
        let choices: SelectableFilterChoices = self
            .state
            .choices_for_draft(draft)
            .into_iter()
            .map(|choice| {
                let selected = choice
                    .key
                    .as_deref()
                    .is_some_and(|key| self.state.choice_is_selected(key));
                (choice, selected)
            })
            .collect::<Vec<_>>()
            .into();
        let resources = self.resources.clone();
        let input = self.database_filter_value_input(cx);
        let list = uniform_list(
            "notion-database-filter-values",
            choices.len(),
            move |range, _window, cx| {
                range
                    .filter_map(|index| {
                        choices.get(index).map(|(choice, selected)| {
                            resources
                                .render_database_filter_choice_row(index, choice, *selected, cx)
                        })
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .w_full()
        .h(px((height - 69.0).max(28.0)));
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .child(database_filter_search_input(input))
            .child(list)
            .into_any_element()
    }

    fn escape_key_listener(
        &self,
        action: DatabaseFilterAction,
    ) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
        let actions = self.resources.actions.clone();
        move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified() || event.keystroke.key != "escape" {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            actions.emit(action.clone(), window, cx);
        }
    }
}

fn database_filter_dropdown_mark(theme: Theme) -> Div {
    div()
        .flex_none()
        .text_size(px(12.0))
        .line_height(px(14.0))
        .text_color(rgb(theme.text_hint))
        .child("⌄")
}

fn database_filter_search_input(input: gpui::Entity<TextInput>) -> Div {
    div()
        .w(px(224.0))
        .h(px(28.0))
        .mx(px(10.0))
        .mb(px(5.0))
        .flex_none()
        .child(input)
}
