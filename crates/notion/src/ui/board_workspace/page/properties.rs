use std::sync::Arc;

use super::super::{
    column_style, div, format_page_property_value, img, px, relative, rgb, rgba, AppearanceMode,
    CardPage, CardPageProperty, Div, FluentBuilder, FontWeight, IntoElement, ParentElement, Styled,
    Theme,
};
use super::editor::{PageDocumentRenderAction, PageRenderAction};
use crate::model::DatabaseStatusProperty;
use crate::ui::{view_actions::ViewActionSink, IconSet};
use gpui::{App, InteractiveElement, MouseButton, MouseDownEvent, SharedString};

#[derive(Clone)]
pub(crate) struct PagePropertiesRenderer {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    status_editing_enabled: bool,
    actions: ViewActionSink<PageRenderAction>,
}

impl PagePropertiesRenderer {
    pub(crate) fn new(
        theme: Theme,
        appearance_mode: AppearanceMode,
        icons: Arc<IconSet>,
        status_editing_enabled: bool,
        actions: ViewActionSink<PageRenderAction>,
    ) -> Self {
        Self {
            theme,
            appearance_mode,
            icons,
            status_editing_enabled,
            actions,
        }
    }

    pub(crate) fn render(&self, page: &CardPage, cx: &mut App) -> Div {
        let theme = self.theme;
        div().pb(px(4.0)).child(
            (div().flex().flex_col().gap(px(12.0)).children(
                page.properties
                    .iter()
                    .map(|property| {
                        (div()
                            .min_h(px(34.0))
                            .flex()
                            .items_start()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .w(px(160.0))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(self.render_page_property_icon(property, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .text_size(px(14.0))
                                            .line_height(relative(1.5))
                                            .text_color(rgb(theme.text_secondary))
                                            .child(property.label.clone()),
                                    ),
                            )
                            .child(self.render_page_property_value(&page.block_id, property, cx)))
                        .into_any_element()
                    })
                    .collect::<Vec<_>>(),
            ))
            .into_any_element(),
        )
    }

    pub(crate) fn render_page_property_icon(
        &self,
        property: &CardPageProperty,
        cx: &mut App,
    ) -> Div {
        div()
            .w(px(22.0))
            .h(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .when_some(self.icons.property_icon_for(property), |this, icon| {
                this.child(img(icon.render(cx)).size(px(14.0)))
            })
            .when(self.icons.property_icon_for(property).is_none(), |this| {
                this.child(
                    div()
                        .size(px(12.0))
                        .rounded(px(3.0))
                        .border_1()
                        .border_color(rgba(self.theme.surface_border)),
                )
            })
    }

    pub(crate) fn render_page_property_value(
        &self,
        page_id: &str,
        property: &CardPageProperty,
        cx: &mut App,
    ) -> Div {
        let rendered_value = format_page_property_value(property);
        let is_empty = rendered_value.trim().is_empty() || rendered_value == "Empty";
        let display_value = if is_empty {
            "Empty".to_string()
        } else {
            rendered_value
        };
        let content = self.render_page_property_content(property, display_value, is_empty, cx);
        let editable = self.status_editing_enabled
            && DatabaseStatusProperty::parse(
                &property.property_id,
                &property.property_type,
                &property.status_options,
            )
            .is_ok();
        let page_id = SharedString::from(page_id.to_string());
        let property = property.clone();
        div()
            .flex_grow(1.0)
            .when(editable, |value| {
                value.cursor_pointer().on_mouse_down(MouseButton::Left, {
                    let actions = self.actions.clone();
                    move |event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        actions.emit(
                            PageRenderAction::Document(
                                PageDocumentRenderAction::OpenStatusPicker {
                                    page_id: page_id.clone(),
                                    property: property.clone(),
                                    position: event.position,
                                },
                            ),
                            window,
                            cx,
                        );
                    }
                })
            })
            .child(content)
    }

    fn render_page_property_content(
        &self,
        property: &CardPageProperty,
        display_value: String,
        is_empty: bool,
        cx: &mut App,
    ) -> Div {
        if !is_empty && property.property_type == "status" {
            return div().child(self.render_page_property_status_badge(&display_value));
        }
        let text = div()
            .text_size(px(14.0))
            .line_height(relative(1.5))
            .text_color(rgb(if is_empty {
                self.theme.text_muted
            } else {
                self.theme.text_primary
            }))
            .child(display_value);
        if !is_empty && property.property_type == "relation" {
            return div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(img(self.icons.page.render(cx)).w(px(14.0)).h(px(14.0)))
                .child(text);
        }
        div().child(text)
    }

    pub(crate) fn render_page_property_status_badge(&self, status: &str) -> Div {
        let style = column_style(status, None, self.appearance_mode);
        let width = 18.0 + status.chars().count() as f32 * 6.4;
        div()
            .w(px(width))
            .h(px(24.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(style.tone.pill_text_color(self.appearance_mode)))
            .bg(rgba(style.pill))
            .child(status.to_string())
    }
}
