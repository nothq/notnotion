use crate::ui::board_workspace::dialogs::filter::{helpers::*, prelude::*, types::*};
use crate::ui::board_workspace::dialogs::property_icons::render_property_picker_option_icon;

impl DatabaseFilterRenderer<'_> {
    pub(in crate::ui::board_workspace::dialogs) fn render_database_filter_dialog(
        &self,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let filter = self.state;
        let resources = &self.resources;
        match filter.stage {
            DatabaseFilterDialogStage::PropertyPicker => {
                self.render_database_filter_property_picker(placement, cx)
            }
            DatabaseFilterDialogStage::Editor => self.render_database_filter_editor(placement, cx),
            DatabaseFilterDialogStage::OperatorPicker => {
                let editor = self.render_database_filter_editor(placement, cx);
                let picker =
                    filter.render_database_filter_operator_picker(resources, placement, cx);
                database_filter_dialog_overlay(editor, picker)
            }
            DatabaseFilterDialogStage::DateModePicker => {
                let editor = self.render_database_filter_editor(placement, cx);
                let picker =
                    filter.render_database_filter_date_mode_picker(resources, placement, cx);
                database_filter_dialog_overlay(editor, picker)
            }
            DatabaseFilterDialogStage::DateValuePicker => {
                let editor = self.render_database_filter_editor(placement, cx);
                let picker =
                    filter.render_database_filter_date_value_picker(resources, placement, cx);
                database_filter_dialog_overlay(editor, picker)
            }
            DatabaseFilterDialogStage::MoreActions => {
                let editor = self.render_database_filter_editor(placement, cx);
                let actions = filter.render_database_filter_more_actions(resources, placement, cx);
                database_filter_dialog_overlay(editor, actions)
            }
            DatabaseFilterDialogStage::AdvancedEditor
            | DatabaseFilterDialogStage::AdvancedAddMenu
            | DatabaseFilterDialogStage::AdvancedCombinerPicker => {
                self.render_database_advanced_filter_editor(placement, cx)
            }
        }
    }

    fn render_database_filter_property_picker(
        &self,
        placement: DatabaseFilterDialogPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let properties: Arc<[DatabaseProperty]> = self
            .state
            .filtered_properties(&self.board.database_properties)
            .into();
        let (show_advanced_footer, list_height, list_viewport_height, height) =
            database_filter_property_picker_layout(
                placement,
                self.board.is_locked,
                properties.len(),
                &self.state.property_picker_target,
            );
        let input = self.database_filter_property_input(cx);
        let list = self.render_database_filter_property_list(
            properties,
            list_height,
            list_viewport_height,
        );

        self.resources
            .database_filter_dialog_surface(
                placement,
                DatabaseFilterDialogSize::new(FILTER_PROPERTY_DIALOG_WIDTH, height),
                DatabaseFilterDialogBehavior::new(
                    None,
                    matches!(placement, DatabaseFilterDialogPlacement::FullPage)
                        .then_some(DatabaseFilterOutsideAction::CloseToolbar),
                ),
                cx,
            )
            .child(
                div()
                    .h(px(FILTER_PROPERTY_INPUT_HEIGHT))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(266.0)).h(px(28.0)).flex_none().child(input)),
            )
            .child(list)
            .when(show_advanced_footer, |dialog| {
                dialog.child(self.render_database_advanced_filter_footer())
            })
            .into_any_element()
    }

    fn render_database_filter_property_list(
        &self,
        properties: Arc<[DatabaseProperty]>,
        list_height: f32,
        viewport_height: f32,
    ) -> Div {
        let resources = self.resources.clone();
        let highlighted_index = self.state.property_highlighted_index;
        div()
            .h(px(list_height))
            .flex_none()
            .overflow_hidden()
            .pt(px(4.0))
            .pb(px(3.0))
            .child(
                uniform_list(
                    "notion-database-filter-properties",
                    properties.len(),
                    move |range, _window, cx| {
                        range
                            .filter_map(|index| {
                                properties.get(index).map(|property| {
                                    resources.render_database_filter_property_row(
                                        index,
                                        property,
                                        highlighted_index == index,
                                        cx,
                                    )
                                })
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .w_full()
                .h(px(viewport_height)),
            )
    }

    fn render_database_advanced_filter_footer(&self) -> Div {
        let theme = self.resources.theme;
        let action = DatabaseFilterAction::BeginAdvanced;
        let button = div()
            .id("notion-database-add-advanced-filter")
            .h(px(28.0))
            .rounded(px(6.0))
            .px(px(8.0))
            .role(Role::Button)
            .aria_label("Add advanced filter")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(14.0))
            .text_color(rgb(theme.text_primary))
            .child("+")
            .child("Add advanced filter");
        div()
            .h(px(FILTER_FOOTER_HEIGHT))
            .flex_none()
            .border_t_1()
            .border_color(alpha(theme.text_primary, 0.10))
            .p(px(4.0))
            .child(button)
    }
}

impl DatabaseFilterRenderResources {
    fn render_database_filter_property_row(
        &self,
        index: usize,
        property: &DatabaseProperty,
        selected: bool,
        cx: &mut App,
    ) -> AnyElement {
        let enabled = database_filter_property_type(property.filter_type());
        let action = DatabaseFilterAction::BeginFilter(Box::new(property.clone()));
        let icon = render_property_picker_option_icon(
            self.theme,
            self.appearance_mode,
            database_filter_property_icon(&property.property_type),
            selected,
            cx,
        );
        let row = div()
            .id(format!("notion-database-filter-property-{index}"))
            .mx(px(4.0))
            .h(px(FILTER_PROPERTY_ROW_CONTENT_HEIGHT))
            .flex_none()
            .rounded(px(6.0))
            .px(px(8.0))
            .role(Role::ListBoxOption)
            .aria_label(property.label.clone())
            .focusable()
            .tab_stop(enabled)
            .when(selected, |row| row.bg(alpha(self.theme.text_primary, 0.08)))
            .when(enabled, |row| {
                row.cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_click(self.filter_click_listener(action.clone()))
                    .on_key_down(self.filter_key_listener(action))
            })
            .when(!enabled, |row| row.opacity(0.45))
            .flex()
            .items_center()
            .child(icon)
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child(property.label.clone()),
            );
        database_filter_property_row_container(row)
    }
}

fn database_filter_property_picker_layout(
    placement: DatabaseFilterDialogPlacement,
    board_is_locked: bool,
    property_count: usize,
    target: &DatabaseFilterPropertyPickerTarget,
) -> (bool, f32, f32, f32) {
    let locked_full_page =
        matches!(placement, DatabaseFilterDialogPlacement::FullPage) && board_is_locked;
    let show_footer =
        !locked_full_page && matches!(target, DatabaseFilterPropertyPickerTarget::Simple);
    let natural_height =
        property_count as f32 * FILTER_PROPERTY_ROW_HEIGHT + FILTER_PROPERTY_LIST_PADDING;
    let list_height = if locked_full_page {
        FILTER_FULL_PAGE_PROPERTY_LIST_HEIGHT
    } else {
        natural_height.clamp(8.0, FILTER_INLINE_PROPERTY_LIST_MAX_HEIGHT)
    };
    let viewport_height = (list_height - FILTER_PROPERTY_LIST_PADDING).max(1.0);
    let footer_height = if show_footer {
        FILTER_FOOTER_HEIGHT
    } else {
        0.0
    };
    let height = FILTER_PROPERTY_INPUT_HEIGHT + list_height + footer_height;
    (show_footer, list_height, viewport_height, height)
}

fn database_filter_dialog_overlay(editor: AnyElement, picker: AnyElement) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .child(editor)
        .child(picker)
        .into_any_element()
}

fn database_filter_property_row_container(row: gpui::Stateful<Div>) -> AnyElement {
    div()
        .h(px(FILTER_PROPERTY_ROW_HEIGHT))
        .flex_none()
        .child(row)
        .into_any_element()
}
