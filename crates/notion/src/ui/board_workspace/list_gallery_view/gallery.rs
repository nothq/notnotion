use gpui::{
    div, px, rgb, uniform_list, AnyElement, App, Div, InteractiveElement, IntoElement,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Styled,
};

use crate::ui::{
    surface::DatabaseViewRow, ACTIVE_VIEW_HEIGHT, BOARD_VIEWPORT_RIGHT_GUTTER, BOARD_VIEWPORT_X,
};

use super::super::inline_database::InlineDatabaseLayout;
use super::{
    database_view_card, DatabaseViewAction, DatabaseViewGeometry, DatabaseViewRenderer,
    DatabaseViewViewport, VisibleDatabaseRows, GALLERY_CARD_HEIGHT, GALLERY_GAP,
    GALLERY_ROW_HEIGHT,
};

#[derive(Clone, Copy)]
struct GalleryRowLayout {
    row_index: usize,
    cards_per_row: usize,
    card_width: f32,
}

impl DatabaseViewRenderer {
    pub(in crate::ui::board_workspace) fn render_gallery_view(&self, cx: &mut App) -> Div {
        let visible_rows = self.visible_rows.clone();
        let content_width = self.content_width;
        div()
            .h(px(ACTIVE_VIEW_HEIGHT))
            .min_h(px(0.0))
            .pl(px(BOARD_VIEWPORT_X))
            .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
            .child(self.render_gallery_viewport(
                visible_rows,
                DatabaseViewViewport {
                    element_id: "notion-database-gallery-rows",
                    width: content_width,
                    height: ACTIVE_VIEW_HEIGHT,
                },
                cx,
            ))
    }

    pub(in crate::ui::board_workspace) fn render_inline_gallery_view(
        &self,
        layout: InlineDatabaseLayout,
        cx: &mut App,
    ) -> AnyElement {
        let visible_rows = self.visible_rows.clone();
        let height =
            DatabaseViewGeometry::inline_gallery_height(visible_rows.len(), layout.content_width());
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
                    .child(self.render_gallery_viewport(
                        visible_rows,
                        DatabaseViewViewport {
                            element_id: "notion-inline-database-gallery-rows",
                            width: layout.content_width(),
                            height,
                        },
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn render_gallery_viewport(
        &self,
        visible_rows: VisibleDatabaseRows,
        viewport: DatabaseViewViewport,
        _cx: &mut App,
    ) -> AnyElement {
        if visible_rows.is_empty() {
            return self.render_empty_database_view(viewport.width, viewport.height, "No pages");
        }
        let cards_per_row = DatabaseViewGeometry::gallery_cards_per_row(viewport.width);
        let row_count = DatabaseViewGeometry::gallery_row_count(visible_rows.len(), cards_per_row);
        let card_width = DatabaseViewGeometry::gallery_card_width(viewport.width, cards_per_row);
        let renderer = self.clone();
        uniform_list(viewport.element_id, row_count, move |range, _window, cx| {
            range
                .map(|row_index| {
                    renderer.render_database_gallery_row(
                        &visible_rows,
                        GalleryRowLayout {
                            row_index,
                            cards_per_row,
                            card_width,
                        },
                        cx,
                    )
                })
                .collect::<Vec<_>>()
        })
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .w(px(viewport.width))
        .h(px(viewport.height))
        .overflow_x_hidden()
        .into_any_element()
    }

    fn render_database_gallery_row(
        &self,
        visible_rows: &VisibleDatabaseRows,
        layout: GalleryRowLayout,
        cx: &mut App,
    ) -> AnyElement {
        let start = layout.row_index * layout.cards_per_row;
        let end = (start + layout.cards_per_row).min(visible_rows.len());
        div()
            .h(px(GALLERY_ROW_HEIGHT))
            .w_full()
            .pb(px(GALLERY_GAP))
            .flex()
            .gap(px(GALLERY_GAP))
            .children((start..end).filter_map(|visible_index| {
                visible_rows
                    .row(visible_index)
                    .map(|row| self.render_database_gallery_card(row, layout.card_width, cx))
            }))
            .into_any_element()
    }

    fn render_database_gallery_card(
        &self,
        row: &DatabaseViewRow,
        width: f32,
        cx: &mut App,
    ) -> AnyElement {
        let card = database_view_card(row);
        div()
            .id(format!("notion-database-gallery-card-{}", row.block_id))
            .w(px(width))
            .h(px(GALLERY_CARD_HEIGHT))
            .min_w(px(0.0))
            .flex_none()
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(crate::ui::alpha(self.theme.text_muted, 0.14))
            .bg(rgb(self.theme.app_bg))
            .cursor_pointer()
            .hover(|style| style.bg(crate::ui::alpha(self.theme.text_primary, 0.035)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    DatabaseViewAction::OpenCard(card.clone())
                }),
            )
            .child(
                div()
                    .h(px(58.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(crate::ui::alpha(self.theme.text_muted, 0.08))
                    .flex()
                    .items_center()
                    .child(self.render_database_view_title(row, width - 24.0, cx)),
            )
            .child(
                div()
                    .h(px(GALLERY_CARD_HEIGHT - 58.0))
                    .min_h(px(0.0))
                    .px(px(12.0))
                    .py(px(9.0))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(7.0))
                    .children(
                        row.properties
                            .iter()
                            .map(|property| self.render_database_gallery_property(property)),
                    ),
            )
            .into_any_element()
    }
}
