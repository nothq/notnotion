use crate::ui::board_workspace::dialogs::filter::{
    controller::DatabaseFilterController, helpers::*, prelude::*, types::*,
};
use crate::ui::board_workspace::dialogs::property_icons::render_property_picker_option_icon;

impl DatabaseFilterRenderer<'_> {
    pub(super) fn render_database_filter_chip(
        &self,
        chip: DatabaseFilterChip,
        cx: &mut App,
    ) -> AnyElement {
        let measured_filter_id = chip.draft_filter_id.clone();
        let dirty = chip.dirty;
        let anchor = self.chip_anchor(measured_filter_id);
        let button = self.render_database_filter_chip_button(chip, cx);
        anchor
            .child(button)
            .when(dirty, |chip| {
                chip.child(
                    div()
                        .absolute()
                        .top(px(-2.0))
                        .right(px(-2.0))
                        .size(px(6.0))
                        .rounded_full()
                        .bg(rgb(0xd9730d)),
                )
            })
            .into_any_element()
    }

    fn render_database_filter_chip_button(
        &self,
        chip: DatabaseFilterChip,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let theme = self.resources.theme;
        let editable = chip.filter.is_some()
            || (chip.filter.is_none()
                && self.state.draft.as_ref().map(|draft| &draft.filter_id)
                    == chip.draft_filter_id.as_ref());
        let button = div()
            .id(format!(
                "notion-database-filter-chip-{}",
                chip.draft_filter_id
                    .as_ref()
                    .map_or("unsupported", |id| id.as_str())
            ))
            .h(px(28.0))
            .max_w(px(260.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(theme.text_primary, 0.10))
            .bg(alpha(theme.text_primary, 0.035))
            .role(Role::Button)
            .aria_label(chip.label.clone())
            .focusable()
            .tab_stop(true);
        self.chip_button_interaction(button, editable, &chip)
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .text_color(rgb(theme.text_primary))
            .child(render_property_picker_option_icon(
                theme,
                self.resources.appearance_mode,
                database_filter_property_icon(&chip.property.property_type),
                false,
                cx,
            ))
            .child(database_filter_chip_content(theme, chip.label))
    }

    fn chip_anchor(&self, measured_filter_id: Option<crate::model::NotionDatabaseFilterId>) -> Div {
        let active_filter_id = self
            .state
            .draft
            .as_ref()
            .map(|draft| draft.filter_id.clone());
        let current_anchor = self.state.editor_anchor;
        let actions = self.resources.actions.clone();
        div()
            .relative()
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first().copied() else {
                    return;
                };
                if active_filter_id.as_ref() != measured_filter_id.as_ref()
                    || current_anchor == Some(bounds)
                {
                    return;
                }
                actions.emit(
                    DatabaseFilterAction::MeasureEditorAnchor {
                        filter_id: measured_filter_id.clone(),
                        bounds,
                    },
                    window,
                    cx,
                );
                window.refresh();
            })
            .h(px(28.0))
            .flex_none()
    }

    fn chip_button_interaction(
        &self,
        button: gpui::Stateful<Div>,
        editable: bool,
        chip: &DatabaseFilterChip,
    ) -> gpui::Stateful<Div> {
        if !editable {
            return button;
        }
        let action = DatabaseFilterAction::OpenChip {
            chip: Box::new(chip.clone()),
            host: self.resources.host,
        };
        button
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
    }
}

impl DatabaseFilterController<'_> {
    pub(super) fn open_filter_chip(&mut self, chip: DatabaseFilterChip, host: DatabaseFilterHost) {
        if let Some(filter) = chip.filter.as_ref() {
            self.state.draft = Some(DatabaseFilterDraft::from_filter(chip.property, filter));
            self.state.draft_advanced_path = None;
        } else if self.state.draft.as_ref().map(|draft| &draft.filter_id)
            != chip.draft_filter_id.as_ref()
        {
            return;
        }
        self.state.stage = DatabaseFilterDialogStage::Editor;
        self.state.value_query.clear();
        self.state.value_input.borrow_mut().take();
        self.state.value_focus_requested.set(true);
        self.load_picker_values();
        self.effects.push(DatabaseFilterEffect::Show(host));
    }
}

fn database_filter_chip_content(theme: Theme, label: String) -> Div {
    div()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(2.0))
        .child(div().min_w(px(0.0)).truncate().child(label))
        .child(
            div()
                .flex_none()
                .text_size(px(13.0))
                .line_height(px(16.0))
                .text_color(rgb(theme.text_secondary))
                .child("⌄"),
        )
}

fn unsupported_database_filter_chip(
    filter_id: Option<crate::model::NotionDatabaseFilterId>,
    dirty: bool,
) -> DatabaseFilterChip {
    DatabaseFilterChip {
        filter: None,
        property: DatabaseProperty {
            property_id: String::new(),
            label: "Filter".to_string(),
            property_type: "text".to_string(),
            filter_type: None,
            options: Vec::new(),
            status_groups: Vec::new(),
            relation_collection_id: None,
        },
        label: "Filter".to_string(),
        draft_filter_id: filter_id,
        dirty,
    }
}

impl DatabaseFilterUiState {
    pub(super) fn chips(&self, board: &BoardSnapshot) -> Vec<DatabaseFilterChip> {
        let persisted = self.persisted_state(board);
        let state = self.effective_state(board);
        let mut chips = match state.simple() {
            DatabaseSimpleFiltersState::Entries(filters) => filters
                .iter()
                .filter_map(|filter| self.chip_from_state(filter, &persisted, board))
                .collect(),
            DatabaseSimpleFiltersState::Unsupported => {
                vec![unsupported_database_filter_chip(None, false)]
            }
        };
        if let Some(draft) = self
            .draft
            .as_ref()
            .filter(|draft| !draft.present_in_effective_state)
        {
            chips.push(DatabaseFilterChip {
                filter: None,
                property: draft.property.clone(),
                label: draft.property.label.clone(),
                draft_filter_id: Some(draft.filter_id.clone()),
                dirty: true,
            });
        }
        chips
    }

    fn chip_from_state(
        &self,
        filter_state: &DatabaseSimpleFilterState,
        persisted: &DatabaseViewFilterState,
        board: &BoardSnapshot,
    ) -> Option<DatabaseFilterChip> {
        match filter_state {
            DatabaseSimpleFilterState::Unsupported(filter_id) => {
                Some(unsupported_database_filter_chip(
                    Some(filter_id.clone()),
                    database_filter_entry_is_dirty(filter_state, persisted),
                ))
            }
            DatabaseSimpleFilterState::Editable(filter) => {
                let property = board
                    .database_properties
                    .iter()
                    .find(|property| {
                        property.property_id == filter.filter().property_id().as_str()
                    })?
                    .clone();
                let label =
                    database_filter_chip_label(&property.label, filter.filter().condition());
                Some(DatabaseFilterChip {
                    filter: Some(filter.clone()),
                    property,
                    label,
                    draft_filter_id: Some(filter.filter_id().clone()),
                    dirty: database_filter_entry_is_dirty(filter_state, persisted),
                })
            }
        }
    }
}
