//! The date picker popover Notion opens when a date mention chip is clicked.
//!
//! Measured on Notion desktop 7.31.3: a 248 px dialog anchored at the chip's
//! bottom-left corner, a 224 × 28 date field, a 7 × 32 px calendar with 28 px
//! day buttons, and 28 px option rows (End date, Date format, Include time,
//! Remind, Clear). Every change is committed to Notion immediately.

use std::rc::Rc;

use chrono::{Datelike, NaiveDate};
use gpui::{Bounds, Entity, Pixels};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

use super::super::rich_text::annotations::mention_at_offset;
use super::actions::{PageMentionAction, PageMentionPickerAction};
use super::atoms::page_mention_offset_for_atom;
use super::state::{PageMentionController, PageMentionPickerIdentity};
use crate::model::{mention_date_label, PageMention, PageMentionDate};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, rgb, CardPage, PageMentionPickerState, Theme};

mod actions;
pub(super) mod commit;
mod render;

pub(crate) const PAGE_MENTION_PICKER_WIDTH: f32 = 248.0;
pub(crate) const PAGE_MENTION_PICKER_INSET: f32 = 12.0;
pub(crate) const PAGE_MENTION_PICKER_FIELD_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_MENTION_PICKER_CELL: f32 = 32.0;
pub(crate) const PAGE_MENTION_PICKER_DAY_BUTTON: f32 = 28.0;
pub(crate) const PAGE_MENTION_PICKER_ROW_HEIGHT: f32 = 28.0;
pub(crate) const PAGE_MENTION_PICKER_ROW_INSET: f32 = 4.0;
pub(crate) const PAGE_MENTION_PICKER_SUBMENU_WIDTH: f32 = 220.0;
pub(crate) const PAGE_MENTION_PICKER_SELECTED_DAY: u32 = 0x2783de;
pub(crate) const PAGE_MENTION_PICKER_MUTED_DAY: u32 = 0x8b9898;
pub(crate) const PAGE_MENTION_PICKER_TODAY_BUTTON: u32 = 0x8e8b86;
pub(crate) const PAGE_MENTION_PICKER_TODAY_MARK: u32 = 0xeb5757;
pub(crate) const PAGE_MENTION_PICKER_CALENDAR_ROWS: usize = 6;

impl PageMentionController {
    pub(super) fn open_picker(&mut self, picker: PageMentionPickerState, input: Entity<TextInput>) {
        self.menu = None;
        *self.picker_input.borrow_mut() = Some(input);
        self.picker = Some(picker);
    }
}

pub(super) fn prepare_page_mention_picker_open(
    page: &CardPage,
    block_id: String,
    atom_index: usize,
    anchor: Bounds<Pixels>,
) -> Option<PageMentionPickerState> {
    let editable = page
        .blocks
        .iter()
        .find(|block| block.block_id == block_id)
        .and_then(|block| block.editable_content())?;
    let offset_utf8 = page_mention_offset_for_atom(editable, atom_index)?;
    let PageMention::Date(date) = mention_at_offset(editable, offset_utf8)?.clone() else {
        return None;
    };
    let field_text = picker_field_text(&date);
    Some(PageMentionPickerState {
        block_id,
        offset_utf8,
        visible_month: first_of_month(date.start_date),
        draft: date,
        submenu: None,
        anchor,
        field_text,
    })
}

pub(super) fn page_mention_picker_field_props(
    value: &str,
    theme: Theme,
    actions: ViewActionSink<PageMentionAction>,
    identity: PageMentionPickerIdentity,
) -> TextInputProps {
    let style = TextInputStyle {
        height: gpui::px(PAGE_MENTION_PICKER_FIELD_HEIGHT),
        min_height: gpui::px(PAGE_MENTION_PICKER_FIELD_HEIGHT),
        padding_x: gpui::px(8.0),
        padding_y: gpui::px(4.0),
        radius: gpui::px(6.0),
        background: gpui::Hsla::from(rgb(0x422303)).opacity(0.03),
        border: gpui::Hsla::from(rgb(0x1c1301)).opacity(0.11),
        focused_border: gpui::Hsla::from(rgb(0x1c1301)).opacity(0.11),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_hint).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: gpui::px(14.0),
        line_height: gpui::px(20.0),
        font_family: None,
    };
    TextInputProps::single_line(value.to_string())
        .style(style)
        .bordered(true)
        .request_focus(true)
        .accessibility("notion-mention-picker-date-field", "Date")
        .on_change(picker_field_on_change(actions.clone(), identity.clone()))
        .on_submit(picker_field_on_submit(actions.clone(), identity.clone()))
        .on_escape(picker_field_on_escape(actions, identity))
}

/// "Sep 2, 2026", plus the time when one is set.
pub(crate) fn picker_field_text(date: &PageMentionDate) -> String {
    let mut text = date.start_date.format("%b %-d, %Y").to_string();
    if date.has_time() {
        let mut time_only = date.clone();
        time_only.end_date = None;
        time_only.end_time = None;
        let label = mention_date_label(&time_only, date.start_date);
        if let Some((_, time)) = label.split_once(' ') {
            text.push(' ');
            text.push_str(time);
        }
    }
    text
}

pub(crate) fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("the first day of a valid month exists")
}

fn picker_field_on_change(
    actions: ViewActionSink<PageMentionAction>,
    identity: PageMentionPickerIdentity,
) -> TextInputChange {
    Rc::new(move |value, window, cx| {
        actions.emit(
            PageMentionAction::Picker {
                identity: identity.clone(),
                action: PageMentionPickerAction::FieldChanged(value),
            },
            window,
            cx,
        );
    })
}

fn picker_field_on_submit(
    actions: ViewActionSink<PageMentionAction>,
    identity: PageMentionPickerIdentity,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(
            PageMentionAction::Picker {
                identity: identity.clone(),
                action: PageMentionPickerAction::SubmitField,
            },
            window,
            cx,
        );
    })
}

fn picker_field_on_escape(
    actions: ViewActionSink<PageMentionAction>,
    identity: PageMentionPickerIdentity,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(
            PageMentionAction::Picker {
                identity: identity.clone(),
                action: PageMentionPickerAction::Dismiss,
            },
            window,
            cx,
        );
    })
}
