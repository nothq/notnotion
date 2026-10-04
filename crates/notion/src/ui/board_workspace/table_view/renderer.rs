use std::{collections::HashSet, sync::Arc};

use gpui::{
    div, img, px, rgb, AnyElement, App, Div, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled,
};

use crate::{
    model::DatabaseStatusProperty,
    ui::{
        alpha, surface::DatabaseSearchState, view_actions::ViewActionSink, AppearanceMode,
        BoardItem, BoardSnapshot, IconSet, TableViewColumn, Theme, ToolbarDialogKind,
        BOARD_VIEWPORT_RIGHT_GUTTER, BOARD_VIEWPORT_X, TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT,
    },
};

use super::super::{inline_database::InlineDatabaseLayout, PageShellIconRenderer};
use super::TableViewAction;

pub(in crate::ui::board_workspace) struct TableViewGeometry<'a> {
    board: &'a BoardSnapshot,
    search: &'a DatabaseSearchState,
}

impl<'a> TableViewGeometry<'a> {
    pub(in crate::ui::board_workspace) fn new(
        board: &'a BoardSnapshot,
        search: &'a DatabaseSearchState,
    ) -> Self {
        Self { board, search }
    }

    pub(in crate::ui::board_workspace) fn inline_height(&self) -> f32 {
        TABLE_HEADER_HEIGHT + self.visible_item_count() as f32 * TABLE_ROW_HEIGHT
    }

    fn visible_items(&self) -> Vec<&'a BoardItem> {
        self.board
            .items
            .iter()
            .filter(|item| self.search.item_is_visible(item))
            .collect()
    }

    fn visible_item_count(&self) -> usize {
        self.board
            .items
            .iter()
            .filter(|item| self.search.item_is_visible(item))
            .count()
    }

    fn total_width(&self, minimum_width: f32) -> f32 {
        (self
            .board
            .table_view_columns
            .iter()
            .map(|column| column.width)
            .sum::<f32>()
            + 64.0)
            .max(minimum_width)
    }
}

pub(in crate::ui::board_workspace) struct TableViewRenderer<'a> {
    pub(super) board: &'a BoardSnapshot,
    search: &'a DatabaseSearchState,
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) editable_status_property_ids: HashSet<String>,
    pub(super) actions: ViewActionSink<TableViewAction>,
}

pub(super) struct TableViewResources {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) status_editing_available: bool,
    pub(super) actions: ViewActionSink<TableViewAction>,
}

impl<'a> TableViewRenderer<'a> {
    pub(super) fn new(
        board: &'a BoardSnapshot,
        search: &'a DatabaseSearchState,
        resources: TableViewResources,
    ) -> Self {
        let TableViewResources {
            theme,
            appearance_mode,
            icons,
            page_icons,
            status_editing_available,
            actions,
        } = resources;
        let editable_status_property_ids = if status_editing_available {
            board
                .database_properties
                .iter()
                .filter_map(|property| DatabaseStatusProperty::try_from(property).ok())
                .map(|property| property.property_id().as_str().to_owned())
                .collect()
        } else {
            std::collections::HashSet::new()
        };
        Self {
            board,
            search,
            theme,
            appearance_mode,
            icons,
            page_icons,
            editable_status_property_ids,
            actions,
        }
    }

    pub(super) fn render_table_view(&self, minimum_width: f32, cx: &mut App) -> Div {
        let geometry = TableViewGeometry::new(self.board, self.search);
        let visible_items = geometry.visible_items();
        let total_width = geometry.total_width(minimum_width);

        div()
            .flex_1()
            .min_h(px(0.0))
            .pl(px(BOARD_VIEWPORT_X))
            .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
            .child(
                div()
                    .h_full()
                    .w_full()
                    .rounded(px(12.0))
                    .bg(rgb(self.theme.app_bg))
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w_full()
                            .id("table-view-scroll")
                            .overflow_scroll()
                            .child(
                                div()
                                    .w(px(total_width))
                                    .min_h_full()
                                    .flex()
                                    .flex_col()
                                    .child(self.render_table_header_row(cx))
                                    .children(
                                        visible_items
                                            .into_iter()
                                            .map(|item| self.render_table_row(item, cx))
                                            .collect::<Vec<_>>(),
                                    ),
                            ),
                    ),
            )
    }

    pub(in crate::ui::board_workspace) fn render_inline_table_view(
        &self,
        layout: InlineDatabaseLayout,
        cx: &mut App,
    ) -> Div {
        let geometry = TableViewGeometry::new(self.board, self.search);
        let visible_items = geometry.visible_items();
        let total_width = geometry.total_width(layout.database_width);
        let height = TABLE_HEADER_HEIGHT + visible_items.len() as f32 * TABLE_ROW_HEIGHT;

        let mut scroll = div()
            .h_full()
            .w_full()
            .id("inline-table-view-scroll")
            .overflow_x_scroll()
            .child(
                div().w(px(layout.content_inset + total_width)).child(
                    div()
                        .ml(px(layout.content_inset))
                        .w(px(total_width))
                        .flex()
                        .flex_col()
                        .child(self.render_table_header_row(cx))
                        .children(
                            visible_items
                                .into_iter()
                                .map(|item| self.render_table_row(item, cx))
                                .collect::<Vec<_>>(),
                        ),
                ),
            );
        scroll.style().restrict_scroll_to_axis = Some(true);

        div()
            .h(px(height))
            .ml(px(-layout.viewport_offset_from_database()))
            .w(px(layout.viewport_width))
            .overflow_hidden()
            .child(scroll)
    }

    fn render_table_header_row(&self, cx: &mut App) -> Div {
        div()
            .h(px(TABLE_HEADER_HEIGHT))
            .flex()
            .items_center()
            .children(
                self.board
                    .table_view_columns
                    .iter()
                    .map(|column| self.render_table_header_cell(column))
                    .collect::<Vec<_>>(),
            )
            .child(self.render_table_header_properties_button())
            .child(self.render_table_header_actions_button(cx))
    }

    fn render_table_header_cell(&self, column: &TableViewColumn) -> AnyElement {
        (div()
            .h_full()
            .w(px(column.width))
            .px(px(12.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    TableViewAction::Toolbar(ToolbarDialogKind::Properties)
                }),
            )
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_muted))
            .child(column.label.clone()))
        .into_any_element()
    }

    fn render_table_header_properties_button(&self) -> AnyElement {
        (div()
            .size(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    TableViewAction::Toolbar(ToolbarDialogKind::Properties)
                }),
            )
            .text_size(px(16.0))
            .text_color(rgb(self.theme.text_muted))
            .child("+"))
        .into_any_element()
    }

    fn render_table_header_actions_button(&self, cx: &mut App) -> AnyElement {
        (div()
            .size(px(28.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    TableViewAction::Toolbar(ToolbarDialogKind::Actions)
                }),
            )
            .child(img(self.icons.topbar_actions.render(cx)).size(px(14.0))))
        .into_any_element()
    }

    fn render_table_row(&self, item: &BoardItem, cx: &mut App) -> AnyElement {
        let row_item = item.clone();
        let click_item = row_item.clone();
        (div()
            .id(format!("notion-table-row-{}", row_item.block_id))
            .h(px(TABLE_ROW_HEIGHT - 1.0))
            .border_b_1()
            .border_color(alpha(self.theme.text_muted, 0.08))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    TableViewAction::Item(click_item.clone())
                }),
            )
            .children(
                self.board
                    .table_view_columns
                    .iter()
                    .map(|column| self.render_table_cell(&row_item, column, cx))
                    .collect::<Vec<_>>(),
            ))
        .into_any_element()
    }
}
