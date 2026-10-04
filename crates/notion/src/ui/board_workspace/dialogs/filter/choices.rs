use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};

impl DatabaseFilterRenderResources {
    pub(super) fn render_database_filter_choice_visual(
        &self,
        label: &str,
        visual: DatabaseFilterChoiceVisual,
        cx: &mut App,
    ) -> AnyElement {
        match visual {
            DatabaseFilterChoiceVisual::Avatar {
                profile_photo,
                fallback,
            } => self.render_database_filter_avatar(profile_photo, fallback),
            DatabaseFilterChoiceVisual::Page(icon) => self.page_icons.render(&icon, 18.0, cx),
            DatabaseFilterChoiceVisual::OptionColor(color) => {
                let style = column_style(label, Some(&color), self.appearance_mode);
                div()
                    .size(px(10.0))
                    .flex_none()
                    .rounded_full()
                    .bg(rgb(style.pill.hex))
                    .into_any_element()
            }
            DatabaseFilterChoiceVisual::Checkbox(checked) => div()
                .size(px(16.0))
                .flex_none()
                .rounded(px(3.0))
                .border_1()
                .border_color(if checked {
                    rgb(0x2383e2).into()
                } else {
                    alpha(self.theme.text_primary, 0.22)
                })
                .when(checked, |mark| mark.bg(rgb(0x2383e2)))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.0))
                .text_color(rgb(0xffffff))
                .when(checked, |mark| mark.child("✓"))
                .into_any_element(),
        }
    }
}

fn database_checkbox_filter_choices() -> Vec<DatabaseFilterChoice> {
    [
        (CHECKBOX_FILTER_UNCHECKED_VALUE, "Unchecked", false),
        (CHECKBOX_FILTER_CHECKED_VALUE, "Checked", true),
    ]
    .into_iter()
    .map(|(key, label, checked)| DatabaseFilterChoice {
        key: Some(key.to_string()),
        label: label.to_string(),
        secondary: None,
        visual: Some(DatabaseFilterChoiceVisual::Checkbox(checked)),
    })
    .collect()
}

fn database_option_filter_choices(
    draft: &DatabaseFilterDraft,
    query: &str,
) -> Vec<DatabaseFilterChoice> {
    draft
        .property
        .options
        .iter()
        .filter(|option| query.is_empty() || option.value.to_lowercase().contains(query))
        .map(|option| DatabaseFilterChoice {
            key: Some(option.value.clone()),
            label: option.value.clone(),
            secondary: None,
            visual: Some(DatabaseFilterChoiceVisual::OptionColor(
                option.color.clone(),
            )),
        })
        .collect()
}

fn database_status_filter_choices(draft: &DatabaseFilterDraft) -> Vec<DatabaseFilterChoice> {
    let mut choices = Vec::new();
    for group in &draft.property.status_groups {
        choices.push(DatabaseFilterChoice {
            key: Some(format!("{STATUS_GROUP_FILTER_VALUE_PREFIX}{}", group.name)),
            label: group.name.clone(),
            secondary: Some("Group".to_string()),
            visual: Some(DatabaseFilterChoiceVisual::OptionColor(group.color.clone())),
        });
        choices.extend(
            draft
                .property
                .options
                .iter()
                .filter(|option| group.option_ids.contains(&option.id))
                .map(|option| DatabaseFilterChoice {
                    key: Some(format!(
                        "{STATUS_OPTION_FILTER_VALUE_PREFIX}{}",
                        option.value
                    )),
                    label: option.value.clone(),
                    secondary: None,
                    visual: Some(DatabaseFilterChoiceVisual::OptionColor(
                        option.color.clone(),
                    )),
                }),
        );
    }
    choices
}

impl DatabaseFilterRenderResources {
    pub(super) fn render_database_filter_choice_row(
        &self,
        index: usize,
        choice: &DatabaseFilterChoice,
        selected: bool,
        cx: &mut App,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(key) = choice.key.clone() else {
            return self.render_database_filter_choice_heading(&choice.label);
        };
        let visual = choice
            .visual
            .clone()
            .map(|visual| self.render_database_filter_choice_visual(&choice.label, visual, cx));
        let action = DatabaseFilterAction::ToggleSelection(key);
        let row = div()
            .id(format!("notion-database-filter-value-{index}"))
            .mx(px(4.0))
            .h(px(28.0))
            .flex_none()
            .rounded(px(6.0))
            .px(px(8.0))
            .role(Role::ListBoxOption)
            .aria_label(choice.label.clone())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .on_click(self.filter_click_listener(action.clone()))
            .on_key_down(self.filter_key_listener(action))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(14.0))
            .text_color(rgb(theme.text_primary));
        self.render_database_filter_choice_content(row, choice, visual, selected)
    }

    fn render_database_filter_choice_content(
        &self,
        row: gpui::Stateful<Div>,
        choice: &DatabaseFilterChoice,
        visual: Option<AnyElement>,
        selected: bool,
    ) -> AnyElement {
        row.when_some(visual, |row, visual| row.child(visual))
            .child(div().min_w(px(0.0)).truncate().child(choice.label.clone()))
            .when_some(choice.secondary.clone(), |row, secondary| {
                row.child(div().flex_grow(1.0)).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.text_hint))
                        .child(secondary),
                )
            })
            .when(choice.secondary.is_none(), |row| {
                row.child(div().flex_grow(1.0))
            })
            .when(selected, |row| {
                row.child(
                    div()
                        .size(px(16.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(self.theme.text_primary))
                        .child("✓"),
                )
            })
            .into_any_element()
    }

    fn render_database_filter_choice_heading(&self, label: &str) -> AnyElement {
        div()
            .h(px(FILTER_PROPERTY_ROW_HEIGHT))
            .px(px(12.0))
            .flex_none()
            .flex()
            .items_center()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_secondary))
            .child(label.to_string())
            .into_any_element()
    }

    fn render_database_filter_avatar(
        &self,
        profile_photo: Option<String>,
        fallback: String,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .relative()
            .size(px(20.0))
            .flex_none()
            .rounded_full()
            .overflow_hidden()
            .bg(alpha(theme.text_primary, 0.12))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(9.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(theme.text_secondary))
            .child(fallback)
            .when_some(profile_photo, |avatar, url| {
                avatar.child(
                    img(url)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .with_loading(|| div().size_full().into_any_element())
                        .with_fallback(|| div().size_full().into_any_element()),
                )
            })
            .into_any_element()
    }
}

impl DatabaseFilterUiState {
    pub(super) fn choice_is_selected(&self, key: &str) -> bool {
        self.draft.as_ref().is_some_and(|draft| {
            if draft.property.filter_type() == "checkbox" {
                return database_checkbox_filter_value(key)
                    .is_some_and(|checked| draft.checkbox_value == Some(checked));
            }
            draft.selected_values.iter().any(|selected| selected == key)
        })
    }

    pub(super) fn choices_for_draft(
        &self,
        draft: &DatabaseFilterDraft,
    ) -> Vec<DatabaseFilterChoice> {
        let query = self.value_query.trim().to_lowercase();
        match draft.property.filter_type() {
            "checkbox" => database_checkbox_filter_choices(),
            "person" => self.person_choices(&query),
            "relation" => self.relation_choices(&query),
            "select" | "multi_select" => database_option_filter_choices(draft, &query),
            "status" => database_status_filter_choices(draft),
            _ => Vec::new(),
        }
    }

    fn person_choices(&self, query: &str) -> Vec<DatabaseFilterChoice> {
        let mut choices = Vec::new();
        if query.is_empty() || "me".contains(query) {
            let current_user = self
                .users
                .as_ref()
                .and_then(|users| users.iter().find(|user| user.is_current_user));
            choices.push(DatabaseFilterChoice {
                key: Some(CURRENT_USER_FILTER_VALUE.to_string()),
                label: "Me".to_string(),
                secondary: None,
                visual: Some(DatabaseFilterChoiceVisual::Avatar {
                    profile_photo: current_user.and_then(|user| user.profile_photo.clone()),
                    fallback: current_user.map_or_else(
                        || "M".to_string(),
                        |user| database_filter_avatar_fallback(&user.name),
                    ),
                }),
            });
        }
        if let Some(users) = self.users.as_ref() {
            choices.extend(
                users
                    .iter()
                    .filter(|user| !user.is_current_user)
                    .filter(|user| query.is_empty() || user.name.to_lowercase().contains(query))
                    .map(|user| DatabaseFilterChoice {
                        key: Some(user.user_id.as_str().to_string()),
                        label: user.name.clone(),
                        secondary: None,
                        visual: Some(DatabaseFilterChoiceVisual::Avatar {
                            profile_photo: user.profile_photo.clone(),
                            fallback: database_filter_avatar_fallback(&user.name),
                        }),
                    }),
            );
        }
        choices
    }

    fn relation_choices(&self, query: &str) -> Vec<DatabaseFilterChoice> {
        self.relation_pages
            .iter()
            .filter(|page| query.is_empty() || page.title.to_lowercase().contains(query))
            .map(|page| DatabaseFilterChoice {
                key: Some(page.page_id.as_str().to_string()),
                label: page.title.clone(),
                secondary: None,
                visual: Some(DatabaseFilterChoiceVisual::Page(page.icon.clone())),
            })
            .collect()
    }
}
