use gpui::{
    div, px, uniform_list, AnyElement, App, Div, InteractiveElement, IntoElement,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Styled,
};

use crate::ui::{
    surface::DatabaseViewRow, ACTIVE_VIEW_HEIGHT, BOARD_VIEWPORT_RIGHT_GUTTER, BOARD_VIEWPORT_X,
};

use super::super::inline_database::InlineDatabaseLayout;
use super::{
    database_view_card, DatabaseViewAction, DatabaseViewGeometry, DatabaseViewRenderer,
    DatabaseViewViewport, VisibleDatabaseRows, LIST_ROW_HEIGHT, LIST_TITLE_WIDTH,
};

impl DatabaseViewRenderer {
    pub(in crate::ui::board_workspace) fn render_list_view(&self, cx: &mut App) -> Div {
        let visible_rows = self.visible_rows.clone();
        let content_width = self.content_width;
        div()
            .h(px(ACTIVE_VIEW_HEIGHT))
            .min_h(px(0.0))
            .pl(px(BOARD_VIEWPORT_X))
            .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
            .child(self.render_list_viewport(
                visible_rows,
                DatabaseViewViewport {
                    element_id: "notion-database-list-rows",
                    width: content_width,
                    height: ACTIVE_VIEW_HEIGHT,
                },
                cx,
            ))
    }

    pub(in crate::ui::board_workspace) fn render_inline_list_view(
        &self,
        layout: InlineDatabaseLayout,
        cx: &mut App,
    ) -> AnyElement {
        let visible_rows = self.visible_rows.clone();
        let height = DatabaseViewGeometry::inline_list_height(visible_rows.len());
        div()
            .h(px(height))
            .ml(px(-layout.viewport_offset_from_database()))
            .w(px(layout.viewport_width))
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .ml(px(layout.content_inset))
                    .w(px(layout.content_width()))
                    .child(self.render_list_viewport(
                        visible_rows,
                        DatabaseViewViewport {
                            element_id: "notion-inline-database-list-rows",
                            width: layout.content_width(),
                            height,
                        },
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn render_list_viewport(
        &self,
        visible_rows: VisibleDatabaseRows,
        viewport: DatabaseViewViewport,
        _cx: &mut App,
    ) -> AnyElement {
        if visible_rows.is_empty() {
            return self.render_empty_database_view(viewport.width, viewport.height, "No pages");
        }
        let renderer = self.clone();
        uniform_list(
            viewport.element_id,
            visible_rows.len(),
            move |range, _window, cx| {
                range
                    .filter_map(|visible_index| {
                        visible_rows
                            .row(visible_index)
                            .map(|row| renderer.render_database_list_row(row, viewport.width, cx))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .w(px(viewport.width))
        .h(px(viewport.height))
        .rounded(px(10.0))
        .overflow_x_hidden()
        .into_any_element()
    }

    fn render_database_list_row(
        &self,
        row: &DatabaseViewRow,
        viewport_width: f32,
        cx: &mut App,
    ) -> AnyElement {
        let card = database_view_card(row);
        let title_width = LIST_TITLE_WIDTH.min((viewport_width * 0.45).max(120.0));
        div()
            .id(format!("notion-database-list-row-{}", row.block_id))
            .h(px(LIST_ROW_HEIGHT))
            .w_full()
            .min_w(px(0.0))
            .px(px(12.0))
            .border_b_1()
            .border_color(crate::ui::alpha(self.theme.text_muted, 0.1))
            .cursor_pointer()
            .hover(|style| style.bg(crate::ui::alpha(self.theme.text_primary, 0.035)))
            .flex()
            .items_center()
            .gap(px(12.0))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    DatabaseViewAction::OpenCard(card.clone())
                }),
            )
            .child(self.render_database_view_title(row, title_width, cx))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap(px(18.0))
                    .children(
                        row.properties
                            .iter()
                            .map(|property| self.render_database_list_property(property)),
                    ),
            )
            .into_any_element()
    }
}
