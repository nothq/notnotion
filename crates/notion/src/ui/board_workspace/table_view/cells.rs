use gpui::{
    div, px, relative, rgb, App, Div, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, SharedString, Styled,
};

use crate::{
    model::{BoardItemProperty, DatabaseProperty},
    ui::{column_style, format_date_like_value, rgba, BoardItem, FluentBuilder, TableViewColumn},
};

use super::{renderer::TableViewRenderer, TableViewAction};

impl<'a> TableViewRenderer<'a> {
    pub(super) fn render_table_cell(
        &self,
        item: &BoardItem,
        column: &TableViewColumn,
        cx: &mut App,
    ) -> Div {
        let cell = div()
            .h_full()
            .w(px(column.width))
            .px(px(8.0))
            .overflow_hidden()
            .flex()
            .items_center();

        if column.property_id == "title" {
            return cell.child(self.render_table_title_cell(item, cx));
        }

        let property = item
            .properties
            .iter()
            .find(|property| property.property_id == column.property_id);
        let property_schema = self
            .board
            .database_properties
            .iter()
            .find(|property| property.property_id == column.property_id);
        if let Some(property_schema) =
            property_schema.filter(|property| property.property_type == "status")
        {
            return self.render_table_status_cell(cell, item, property, property_schema);
        }

        let Some(property) = property else {
            return cell.child(self.render_table_text_value("", false));
        };

        match property.property_type.as_str() {
            "multi_select" => cell.child(self.render_table_multi_select_value(&property.value)),
            "people" | "created_by" | "last_edited_by" => {
                cell.child(self.render_table_people_value(&property.value))
            }
            "created_time" | "last_edited_time" => cell.child(
                self.render_table_text_value(&format_table_datetime_value(&property.value), true),
            ),
            _ => cell.child(self.render_table_text_value(&property.value, true)),
        }
    }

    fn render_table_status_cell(
        &self,
        cell: Div,
        item: &BoardItem,
        property: Option<&BoardItemProperty>,
        property_schema: &DatabaseProperty,
    ) -> Div {
        let status = property
            .map(|property| property.value.as_str())
            .or(item.status.as_deref())
            .unwrap_or_default();
        let editable = self
            .editable_status_property_ids
            .contains(&property_schema.property_id);
        let page_id = SharedString::from(item.block_id.clone());
        let property_id = SharedString::from(property_schema.property_id.clone());
        let current_value = SharedString::from(status.to_string());
        cell.when(editable, |cell| {
            cell.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    TableViewAction::StatusPicker {
                        page_id: page_id.clone(),
                        property_id: property_id.clone(),
                        current_value: current_value.clone(),
                        position: event.position,
                    }
                }),
            )
        })
        .child(if status.trim().is_empty() {
            self.render_table_text_value("Empty", false)
        } else {
            self.render_status_badge(status)
        })
    }

    fn render_table_title_cell(&self, item: &BoardItem, cx: &mut App) -> Div {
        let icon = item
            .icon
            .as_ref()
            .map(|icon| self.page_icons.render(icon, 16.0, cx))
            .unwrap_or_else(|| {
                gpui::img(self.icons.page.render(cx))
                    .w(px(16.0))
                    .h(px(16.0))
                    .into_any_element()
            });
        div()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(icon)
            .child(
                div()
                    .min_w(px(0.0))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(14.0))
                    .line_height(relative(1.2))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child(item.title.clone()),
            )
    }

    fn render_table_text_value(&self, value: &str, has_value: bool) -> Div {
        div()
            .min_w(px(0.0))
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .text_size(px(13.0))
            .line_height(relative(1.2))
            .text_color(rgb(if has_value {
                self.theme.text_secondary
            } else {
                self.theme.text_muted
            }))
            .child(value.to_string())
    }

    fn render_table_multi_select_value(&self, value: &str) -> Div {
        let tags = value
            .split(", ")
            .filter(|tag| !tag.trim().is_empty())
            .collect::<Vec<_>>();
        if tags.is_empty() {
            return self.render_table_text_value("", false);
        }
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .flex_wrap()
            .children(
                tags.into_iter()
                    .map(|tag| self.render_table_tag_chip(tag).into_any_element())
                    .collect::<Vec<_>>(),
            )
    }

    fn render_table_tag_chip(&self, tag: &str) -> Div {
        let style = column_style(tag, None, self.appearance_mode);
        div()
            .h(px(20.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(style.tone.pill_text_color(self.appearance_mode)))
            .bg(rgba(style.pill))
            .child(tag.to_string())
    }

    fn render_table_people_value(&self, value: &str) -> Div {
        let people = value
            .split(", ")
            .filter(|person| !person.trim().is_empty())
            .collect::<Vec<_>>();
        if people.is_empty() {
            return self.render_table_text_value("", false);
        }
        div().flex().items_center().gap(px(6.0)).children(
            people
                .into_iter()
                .map(|person| self.render_table_person_chip(person).into_any_element())
                .collect::<Vec<_>>(),
        )
    }

    fn render_table_person_chip(&self, person: &str) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .size(px(18.0))
                    .rounded_full()
                    .bg(rgba(self.theme.page_icon_bg))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(self.theme.text_secondary))
                    .child(table_person_initials(person)),
            )
            .child(self.render_table_text_value(person, true))
    }

    fn render_status_badge(&self, status: &str) -> Div {
        let style = column_style(status, None, self.appearance_mode);
        div()
            .flex_none()
            .h(px(24.0))
            .px(px(12.0))
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

fn format_table_datetime_value(value: &str) -> String {
    let normalized = value
        .split_once('T')
        .and_then(|(date, time)| time.get(..5).map(|time| format!("{date} {time}")))
        .unwrap_or_else(|| value.to_string());
    format_date_like_value(&normalized).unwrap_or_else(|| value.to_string())
}

fn table_person_initials(person: &str) -> String {
    let mut initials = person
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if initials.is_empty() {
        initials = "?".to_string();
    }
    initials
}
