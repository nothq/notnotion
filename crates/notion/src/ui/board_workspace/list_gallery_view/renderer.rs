use gpui::{
    div, px, rgb, AnyElement, App, FontWeight, IntoElement, ParentElement, SharedString, Styled,
};

use crate::ui::{
    alpha, column_style, relative, rgba,
    surface::{DatabaseViewPropertyRow, DatabaseViewRow},
    view_actions::ViewActionSink,
    AppearanceMode, Theme,
};

use super::super::PageShellIconRenderer;
use super::{DatabaseViewAction, VisibleDatabaseRows, DATABASE_EMPTY_VIEW_HEIGHT};

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct DatabaseViewRenderer {
    pub(super) visible_rows: VisibleDatabaseRows,
    pub(super) content_width: f32,
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) actions: ViewActionSink<DatabaseViewAction>,
}

impl DatabaseViewRenderer {
    pub(super) fn render_database_view_title(
        &self,
        row: &DatabaseViewRow,
        width: f32,
        cx: &mut App,
    ) -> AnyElement {
        let icon = row
            .icon
            .as_ref()
            .map(|icon| self.page_icons.render(icon, 18.0, cx))
            .unwrap_or_else(|| self.page_icons.builtin("page", 18.0, cx));
        div()
            .w(px(width.max(0.0)))
            .min_w(px(0.0))
            .overflow_hidden()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().flex_none().child(icon))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .line_height(relative(1.2))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child(row.title.clone()),
            )
            .into_any_element()
    }

    pub(super) fn render_database_list_property(
        &self,
        property: &DatabaseViewPropertyRow,
    ) -> AnyElement {
        div()
            .max_w(px(200.0))
            .min_w(px(80.0))
            .overflow_hidden()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(property.label.clone()),
            )
            .child(self.render_database_view_property_value(property, true))
            .into_any_element()
    }

    pub(super) fn render_database_gallery_property(
        &self,
        property: &DatabaseViewPropertyRow,
    ) -> AnyElement {
        div()
            .h(px(20.0))
            .min_h(px(20.0))
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .w(px(82.0))
                    .flex_none()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(11.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(property.label.clone()),
            )
            .child(self.render_database_view_property_value(property, false))
            .into_any_element()
    }

    pub(super) fn render_empty_database_view(
        &self,
        width: f32,
        height: f32,
        message: &'static str,
    ) -> AnyElement {
        div()
            .w(px(width))
            .h(px(height.max(DATABASE_EMPTY_VIEW_HEIGHT)))
            .rounded(px(10.0))
            .border_1()
            .border_color(alpha(self.theme.text_muted, 0.1))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_muted))
            .child(message)
            .into_any_element()
    }

    fn render_database_view_property_value(
        &self,
        property: &DatabaseViewPropertyRow,
        compact: bool,
    ) -> AnyElement {
        if property.property_type.as_ref() == "status" && !property.value.trim().is_empty() {
            let style = column_style(property.value.as_ref(), None, self.appearance_mode);
            return div()
                .h(px(if compact { 22.0 } else { 20.0 }))
                .min_w(px(0.0))
                .max_w(px(132.0))
                .px(px(7.0))
                .rounded(px(5.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(style.tone.pill_text_color(self.appearance_mode)))
                .bg(rgba(style.pill))
                .child(property.value.clone())
                .into_any_element();
        }
        let value = if property.value.trim().is_empty() {
            SharedString::from("Empty")
        } else {
            property.value.clone()
        };
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .text_color(rgb(if property.value.trim().is_empty() {
                self.theme.text_muted
            } else {
                self.theme.text_secondary
            }))
            .child(value)
            .into_any_element()
    }
}
