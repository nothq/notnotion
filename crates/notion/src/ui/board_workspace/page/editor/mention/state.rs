use std::{cell::RefCell, sync::Arc};

use gpui::Entity;
use gpui_components::text_input::TextInput;

use super::{rows, PageMentionClock, PageMentionMenuRow, PageMentionMenuSection};
use crate::ui::{
    PageMentionMenuState, PageMentionPagesState, PageMentionPeopleState, PageMentionPickerState,
};

/// State owned by the page editor's inline mention menu and date picker.
#[derive(Default)]
pub(crate) struct PageMentionController {
    pub(super) menu: Option<PageMentionMenuState>,
    /// A dismissed trigger `(block id, offset)` that must remain closed while
    /// the same `@` token is still being typed after.
    pub(super) menu_suppressed: Option<(String, usize)>,
    pub(super) people: PageMentionPeopleState,
    pub(super) pages: PageMentionPagesState,
    pub(super) picker: Option<PageMentionPickerState>,
    pub(super) picker_input: RefCell<Option<Entity<TextInput>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ui::board_workspace::page) struct PageMentionMenuIdentity {
    pub(super) block_id: String,
    pub(super) trigger_offset: usize,
    pub(super) query: String,
}

impl PageMentionMenuIdentity {
    pub(super) fn new(menu: &PageMentionMenuState) -> Self {
        Self {
            block_id: menu.block_id.clone(),
            trigger_offset: menu.trigger_offset,
            query: menu.query.clone(),
        }
    }

    pub(super) fn matches(&self, menu: &PageMentionMenuState) -> bool {
        self.block_id == menu.block_id
            && self.trigger_offset == menu.trigger_offset
            && self.query == menu.query
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ui::board_workspace::page) struct PageMentionPickerIdentity {
    pub(super) block_id: String,
    pub(super) offset_utf8: usize,
}

impl PageMentionPickerIdentity {
    pub(super) fn new(picker: &PageMentionPickerState) -> Self {
        Self {
            block_id: picker.block_id.clone(),
            offset_utf8: picker.offset_utf8,
        }
    }

    pub(super) fn matches(&self, picker: &PageMentionPickerState) -> bool {
        self.block_id == picker.block_id && self.offset_utf8 == picker.offset_utf8
    }
}

impl PageMentionController {
    pub(crate) fn menu(&self) -> Option<&PageMentionMenuState> {
        self.menu.as_ref()
    }

    pub(crate) fn menu_for_block(&self, block_id: &str) -> Option<&PageMentionMenuState> {
        self.menu.as_ref().filter(|menu| menu.block_id == block_id)
    }

    pub(crate) fn menu_is_open(&self) -> bool {
        self.menu.is_some()
    }

    pub(crate) fn clear_menu(&mut self) -> bool {
        self.menu.take().is_some()
    }

    pub(crate) fn close_menu(&mut self, block_id: &str) -> bool {
        let Some(menu) = self.menu.take_if(|menu| menu.block_id == block_id) else {
            return false;
        };
        self.menu_suppressed = Some((menu.block_id, menu.trigger_offset));
        true
    }

    pub(super) fn dismiss_menu(&mut self, identity: &PageMentionMenuIdentity) -> bool {
        if !self
            .menu
            .as_ref()
            .is_some_and(|menu| identity.matches(menu))
        {
            return false;
        }
        self.menu = None;
        true
    }

    pub(super) fn finish_menu_commit(&mut self, identity: &PageMentionMenuIdentity) -> bool {
        if !self
            .menu
            .as_ref()
            .is_some_and(|menu| identity.matches(menu))
        {
            return false;
        }
        self.menu = None;
        self.menu_suppressed = None;
        true
    }

    pub(super) fn sections(
        &self,
        menu: &PageMentionMenuState,
        clock: &PageMentionClock,
    ) -> Vec<PageMentionMenuSection> {
        let page_results = if self.pages.loaded_query.as_deref() == Some(menu.query.trim()) {
            Arc::clone(&self.pages.results)
        } else {
            Arc::from(Vec::new())
        };
        rows::build_mention_menu_sections(rows::PageMentionMenuInputs {
            query: &menu.query,
            clock,
            users: self.people.users.as_deref().unwrap_or(&[]),
            pages: &page_results,
        })
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn menu_rows(&self, clock: &PageMentionClock) -> Vec<PageMentionMenuRow> {
        self.menu
            .as_ref()
            .map(|menu| {
                self.sections(menu, clock)
                    .into_iter()
                    .flat_map(|section| section.rows)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn move_menu_selection(
        &mut self,
        block_id: &str,
        delta: isize,
        clock: &PageMentionClock,
    ) -> bool {
        let Some(menu) = self.menu_for_block(block_id).cloned() else {
            return false;
        };
        let row_count = self
            .sections(&menu, clock)
            .iter()
            .map(|section| section.rows.len())
            .sum::<usize>();
        if row_count == 0 {
            return false;
        }
        let menu = self
            .menu
            .as_mut()
            .expect("matching mention menu must remain open");
        menu.selected_index =
            (menu.selected_index as isize + delta).rem_euclid(row_count as isize) as usize;
        true
    }

    pub(super) fn selected_row(
        &self,
        menu: &PageMentionMenuState,
        clock: &PageMentionClock,
    ) -> Option<PageMentionMenuRow> {
        let rows = self
            .sections(menu, clock)
            .into_iter()
            .flat_map(|section| section.rows)
            .collect::<Vec<_>>();
        let index = menu.selected_index.min(rows.len().saturating_sub(1));
        rows.into_iter().nth(index)
    }

    pub(crate) fn close_picker(&mut self) -> bool {
        let closed = self.picker.take().is_some();
        self.picker_input.borrow_mut().take();
        closed
    }

    pub(super) fn picker_matches(&self, identity: &PageMentionPickerIdentity) -> bool {
        self.picker
            .as_ref()
            .is_some_and(|picker| identity.matches(picker))
    }
}
