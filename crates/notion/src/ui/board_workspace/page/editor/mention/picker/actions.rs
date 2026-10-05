use chrono::{Months, NaiveDate, NaiveTime, Timelike};
use gpui::App;

use super::super::actions::{
    PageMentionActionOutcome, PageMentionEffect, PageMentionMutationEffect,
    PageMentionPickerAction, PageMentionPickerEffect,
};
use super::super::state::{PageMentionController, PageMentionPickerIdentity};
use super::super::PageMentionClock;
use super::{first_of_month, picker_field_text};
use crate::model::PageMentionTimeFormat;
use crate::model::{parse_mention_date_query, PageMentionDateFormat, PageMentionReminder};
use crate::ui::KeyDownEvent;

impl PageMentionController {
    pub(in crate::ui::board_workspace::page::editor::mention) fn reduce_picker_action(
        &mut self,
        identity: &PageMentionPickerIdentity,
        action: PageMentionPickerAction,
        clock: &PageMentionClock,
        cx: &mut App,
    ) -> PageMentionActionOutcome {
        if !self.picker_matches(identity) {
            return PageMentionActionOutcome::default();
        }
        match action {
            PageMentionPickerAction::Dismiss => {
                PageMentionActionOutcome::changed(self.close_picker())
            }
            PageMentionPickerAction::FieldChanged(value) => {
                let picker = self.picker.as_mut().expect("matching picker remains open");
                if picker.field_text == value {
                    return PageMentionActionOutcome::default();
                }
                picker.field_text = value;
                PageMentionActionOutcome {
                    changed: true,
                    ..PageMentionActionOutcome::default()
                }
            }
            PageMentionPickerAction::SubmitField => self.submit_picker_field(clock, cx),
            PageMentionPickerAction::SelectDay(date) => self.select_picker_day(date),
            PageMentionPickerAction::ShowToday => self.show_picker_today(clock.today),
            PageMentionPickerAction::ShiftMonth(delta) => self.shift_picker_month(delta),
            PageMentionPickerAction::ToggleEndDate => self.toggle_picker_end_date(),
            PageMentionPickerAction::ToggleIncludeTime => self.toggle_picker_time(clock),
            PageMentionPickerAction::ToggleSubmenu(submenu) => self.toggle_picker_submenu(submenu),
            PageMentionPickerAction::SetDateFormat(format) => self.set_picker_date_format(format),
            PageMentionPickerAction::SetTimeFormat(format) => self.set_picker_time_format(format),
            PageMentionPickerAction::SetReminder(reminder) => self.set_picker_reminder(reminder),
            PageMentionPickerAction::Clear => self.clear_picker_effect(),
        }
    }

    pub(crate) fn handle_picker_key_down(&mut self, event: &KeyDownEvent) -> bool {
        if self.picker.is_none() || event.keystroke.key != "escape" {
            return false;
        }
        if self
            .picker
            .as_mut()
            .is_some_and(|picker| picker.submenu.take().is_some())
        {
            return true;
        }
        self.close_picker();
        true
    }

    fn select_picker_day(&mut self, date: NaiveDate) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        if picker.draft.end_date.is_some() && date > picker.draft.start_date {
            picker.draft.end_date = Some(date);
        } else {
            picker.draft.start_date = date;
            if picker.draft.end_date.is_some_and(|end| end < date) {
                picker.draft.end_date = Some(date);
            }
        }
        picker.visible_month = first_of_month(date);
        self.commit_picker_effect()
    }

    fn show_picker_today(&mut self, today: NaiveDate) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        picker.visible_month = first_of_month(today);
        PageMentionActionOutcome::changed(true)
    }

    fn shift_picker_month(&mut self, delta: i32) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        let months = Months::new(delta.unsigned_abs());
        picker.visible_month = if delta >= 0 {
            picker.visible_month.checked_add_months(months)
        } else {
            picker.visible_month.checked_sub_months(months)
        }
        .unwrap_or(picker.visible_month);
        PageMentionActionOutcome::changed(true)
    }

    fn toggle_picker_end_date(&mut self) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        if picker.draft.end_date.is_some() {
            picker.draft.end_date = None;
            picker.draft.end_time = None;
        } else {
            picker.draft.end_date = Some(picker.draft.start_date);
            picker.draft.end_time = picker.draft.start_time;
        }
        self.commit_picker_effect()
    }

    fn toggle_picker_time(&mut self, clock: &PageMentionClock) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        if picker.draft.has_time() {
            picker.draft.start_time = None;
            picker.draft.end_time = None;
            picker.draft.time_zone = None;
            picker.draft.reminder = picker
                .draft
                .reminder
                .take()
                .filter(|reminder| reminder.unit == "day");
        } else {
            let rounded = NaiveTime::from_hms_opt(
                clock.now.hour(),
                if clock.now.minute() >= 30 { 30 } else { 0 },
                0,
            )
            .expect("rounded clock time is valid");
            picker.draft.start_time = Some(rounded);
            if picker.draft.end_date.is_some() {
                picker.draft.end_time = Some(rounded);
            }
            picker.draft.time_zone = Some(clock.time_zone.clone());
            picker.draft.reminder = picker
                .draft
                .reminder
                .take()
                .filter(|reminder| reminder.unit != "day");
        }
        self.commit_picker_effect()
    }

    fn toggle_picker_submenu(
        &mut self,
        submenu: crate::ui::PageMentionPickerSubmenu,
    ) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        picker.submenu = if picker.submenu == Some(submenu) {
            None
        } else {
            Some(submenu)
        };
        PageMentionActionOutcome::changed(true)
    }

    fn set_picker_date_format(
        &mut self,
        format: PageMentionDateFormat,
    ) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        picker.draft.date_format = format;
        picker.submenu = None;
        self.commit_picker_effect()
    }

    fn set_picker_time_format(
        &mut self,
        format: PageMentionTimeFormat,
    ) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        picker.draft.time_format = format;
        picker.submenu = None;
        self.commit_picker_effect()
    }

    fn set_picker_reminder(
        &mut self,
        reminder: Option<PageMentionReminder>,
    ) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        picker.draft.reminder = reminder;
        picker.submenu = None;
        self.commit_picker_effect()
    }

    fn submit_picker_field(
        &mut self,
        clock: &PageMentionClock,
        cx: &mut App,
    ) -> PageMentionActionOutcome {
        let picker = self.picker.as_mut().expect("picker remains open");
        let Some(parsed) = parse_mention_date_query(&picker.field_text, clock.today, clock.now)
        else {
            self.sync_picker_field_text(cx);
            return PageMentionActionOutcome {
                changed: true,
                ..PageMentionActionOutcome::default()
            };
        };
        picker.draft.start_date = parsed.date;
        if let Some(time) = parsed.time {
            picker.draft.start_time = Some(time);
            if picker.draft.time_zone.is_none() {
                picker.draft.time_zone = Some(clock.time_zone.clone());
            }
        }
        if picker.draft.end_date.is_some_and(|end| end < parsed.date) {
            picker.draft.end_date = Some(parsed.date);
        }
        picker.visible_month = first_of_month(parsed.date);
        self.commit_picker_effect()
    }

    fn clear_picker_effect(&self) -> PageMentionActionOutcome {
        let picker = self.picker.clone().expect("picker remains open");
        PageMentionActionOutcome::effect(PageMentionEffect::Mutation(Box::new(
            PageMentionMutationEffect::Picker(PageMentionPickerEffect::Clear(picker)),
        )))
    }

    fn commit_picker_effect(&self) -> PageMentionActionOutcome {
        let picker = self.picker.clone().expect("picker remains open");
        PageMentionActionOutcome::effect(PageMentionEffect::Mutation(Box::new(
            PageMentionMutationEffect::Picker(PageMentionPickerEffect::Commit(picker)),
        )))
    }

    pub(in crate::ui::board_workspace::page::editor::mention) fn sync_picker_field_text(
        &mut self,
        cx: &mut App,
    ) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        picker.field_text = picker_field_text(&picker.draft);
        let text = picker.field_text.clone();
        if let Some(input) = self.picker_input.borrow().clone() {
            input.update(cx, |input, cx| {
                input.set_text_and_move_cursor_to_end(text, cx)
            });
        }
    }
}
