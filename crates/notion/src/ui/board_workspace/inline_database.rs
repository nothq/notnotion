use super::page::editor::PageDocumentColumn;
pub(super) use super::InlineDatabaseToolbarTarget;
pub(super) use super::{
    alpha, div, px, relative, rgb, rgba, AnyElement, Context, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState, ViewTabKind, Window, COLUMN_STRIDE,
    VIEW_TAB_ICON_GAP, VIEW_TAB_ICON_SIZE,
};
pub(super) use crate::ui::surface::InlineDatabaseViewTabLayoutSource;
use gpui::Div;
pub(super) use gpui::{Bounds, Pixels, Role, TextRun};
use std::sync::Arc;

mod metrics;
mod view_tabs;
pub(crate) use metrics::InlineDatabaseMetrics;

const INLINE_DATABASE_CONTROLS_INSET: f32 = 12.0;
const INLINE_DATABASE_CONTENT_INSET: f32 = INLINE_DATABASE_CONTROLS_INSET + 16.0;
pub(crate) const INLINE_DATABASE_LEFT_OVERFLOW: f32 = 8.0;
const INLINE_DATABASE_CONTROLS_HEIGHT: f32 = 40.0;
const INLINE_DATABASE_BOARD_TOP_INSET: f32 = 8.0;
const INLINE_DATABASE_COLUMN_HEADER_HEIGHT: f32 = 40.0;
const INLINE_DATABASE_VIEW_TAB_FONT_SIZE: f32 = 14.0;
const INLINE_DATABASE_VIEW_TAB_HORIZONTAL_PADDING: f32 = 13.0;
const INLINE_DATABASE_VIEW_TAB_GAP: f32 = 1.0;
const INLINE_DATABASE_MORE_HORIZONTAL_PADDING: f32 = 12.0;

#[derive(Clone)]
pub(crate) struct InlineDatabaseBlockRenderer {
    views: Arc<crate::ui::surface::InlineDatabaseViews>,
    width: f32,
}

impl InlineDatabaseBlockRenderer {
    pub(crate) fn new(
        views: crate::ui::surface::InlineDatabaseViews,
        column: PageDocumentColumn,
    ) -> Self {
        Self {
            views: Arc::new(views),
            width: column.width(),
        }
    }

    pub(crate) fn render(&self, page_id: &str, block_id: &str) -> Div {
        let key = format!("{page_id}:{block_id}");
        div()
            .relative()
            .ml(px(-INLINE_DATABASE_LEFT_OVERFLOW))
            .w(px(self.width))
            .when_some(self.views.get(&key), |this, view| this.child(view.clone()))
    }
}

/// An inline database spans its page's column, and its views scroll across
/// the whole pane showing the page.
#[derive(Clone, Copy)]
pub(crate) struct InlineDatabaseLayout {
    pub(crate) viewport_width: f32,
    pub(crate) content_inset: f32,
    pub(crate) database_width: f32,
}

impl InlineDatabaseLayout {
    pub(crate) fn for_page_column(column: PageDocumentColumn) -> Self {
        Self {
            viewport_width: column.pane_width(),
            content_inset: column.leading_inset(),
            database_width: column.width(),
        }
    }

    pub(crate) fn viewport_offset_from_database(&self) -> f32 {
        (self.content_inset - INLINE_DATABASE_LEFT_OVERFLOW).max(0.0)
    }

    fn controls_width(&self) -> f32 {
        (self.database_width - INLINE_DATABASE_CONTROLS_INSET).max(0.0)
    }

    pub(crate) fn content_width(&self) -> f32 {
        (self.database_width - INLINE_DATABASE_CONTENT_INSET).max(0.0)
    }
}

impl SurfaceState {
    pub(crate) fn inline_database_metrics(
        &self,
        layout: InlineDatabaseLayout,
    ) -> InlineDatabaseMetrics {
        InlineDatabaseMetrics::new(self, layout.content_width())
    }

    pub(crate) fn render_inline_database_surface(
        &self,
        layout: InlineDatabaseLayout,
        toolbar_target: InlineDatabaseToolbarTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let columns = (0..self.columns.len())
            .map(|column_index| {
                self.board_columns_renderer(cx)
                    .render_inline_column(column_index, cx)
            })
            .collect::<Vec<_>>();
        let metrics = self.inline_database_metrics(layout);
        let surface_id = format!("notion-inline-database-{}", toolbar_target.board_url);

        div()
            .id(surface_id)
            .relative()
            .w(px(layout.database_width))
            .h(px(metrics.total_height()))
            .flex()
            .flex_col()
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .child(self.render_inline_database_controls(
                toolbar_target,
                layout,
                metrics.controls_height,
                cx,
            ))
            .child(self.render_inline_database_active_view(
                columns,
                layout,
                metrics.active_view_height,
                cx,
            ))
            .into_any_element()
    }

    fn render_inline_database_controls(
        &self,
        toolbar_target: InlineDatabaseToolbarTarget,
        layout: InlineDatabaseLayout,
        controls_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view_tabs_target = toolbar_target.clone();
        div()
            .h(px(controls_height))
            .w(px(layout.controls_width()))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(INLINE_DATABASE_CONTROLS_HEIGHT))
                    .w_full()
                    .pl(px(8.0))
                    .pr(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        view_tabs::inline_database_view_tabs_renderer(self, &view_tabs_target, cx)
                            .render_inline_database_view_tabs(&view_tabs_target, cx),
                    )
                    .child(self.render_inline_board_toolbar(toolbar_target.clone(), cx)),
            )
            .when(
                self.database_filter.bar_is_visible(&self.board),
                |controls| {
                    controls.child(self.render_database_filter_bar(Some(&toolbar_target), cx))
                },
            )
            .into_any_element()
    }

    fn render_inline_database_active_view(
        &self,
        columns: Vec<AnyElement>,
        layout: InlineDatabaseLayout,
        active_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.board.active_view_kind() {
            ViewTabKind::Board | ViewTabKind::Unknown => {
                let columns_width = self.columns.len() as f32 * COLUMN_STRIDE;
                let mut scroll = div()
                    .id("inline-board-scroll")
                    .h_full()
                    .w_full()
                    .overflow_x_scroll()
                    .track_scroll(&self.board_view.scroll_handle)
                    .child(
                        div().w(px(layout.content_inset + columns_width)).child(
                            div()
                                .ml(px(layout.content_inset))
                                .w(px(columns_width))
                                .flex()
                                .gap_3()
                                .children(columns),
                        ),
                    );
                scroll.style().restrict_scroll_to_axis = Some(true);
                div()
                    .h(px(active_height))
                    .ml(px(-layout.viewport_offset_from_database()))
                    .w(px(layout.viewport_width))
                    .child(scroll)
                    .into_any_element()
            }
            ViewTabKind::Table => super::table_view::table_view_renderer(self, cx)
                .render_inline_table_view(layout, cx)
                .into_any_element(),
            ViewTabKind::List => super::list_gallery_view::database_view_renderer(self, cx)
                .render_inline_list_view(layout, cx),
            ViewTabKind::Gallery => super::list_gallery_view::database_view_renderer(self, cx)
                .render_inline_gallery_view(layout, cx),
            ViewTabKind::Timeline => self.render_timeline_view(cx).into_any_element(),
            ViewTabKind::Calendar => self
                .render_inline_calendar_view(layout, cx)
                .into_any_element(),
        }
    }
}
