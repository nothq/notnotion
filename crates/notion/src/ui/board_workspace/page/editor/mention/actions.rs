use gpui::{App, Bounds, Pixels};
use gpui_components::text_input::TextInputSnapshot;

use super::data::PageMentionPageSchedule;
use super::state::{PageMentionController, PageMentionMenuIdentity, PageMentionPickerIdentity};
use super::{mention_menu_token, PageMentionClock, PageMentionMenuRow};
use crate::model::{PageMentionDateFormat, PageMentionReminder, PageMentionTimeFormat};
use crate::ui::surface::PageDocuments;
use crate::ui::{PageMentionMenuState, PageMentionPickerState};

use super::super::persistence::VerifiedPageTextBlockKind;

pub(in crate::ui::board_workspace::page) enum PageMentionAction {
    ReconcileMenu(PageMentionMenuInput),
    OpenMenu(PageMentionMenuState),
    SearchDelayElapsed {
        generation: u64,
        query: String,
    },
    CommitSelection {
        block_id: String,
    },
    MoveMenu {
        block_id: String,
        delta: isize,
    },
    CloseMenu {
        block_id: String,
    },
    DismissMenu(PageMentionMenuIdentity),
    HoverMenuRow {
        identity: PageMentionMenuIdentity,
        index: usize,
    },
    ActivateMenuRow {
        identity: PageMentionMenuIdentity,
        row: Box<PageMentionMenuRow>,
    },
    OpenPickerAtAtom {
        block_id: String,
        atom_index: usize,
        anchor: Bounds<Pixels>,
    },
    Picker {
        identity: PageMentionPickerIdentity,
        action: PageMentionPickerAction,
    },
}

pub(in crate::ui::board_workspace::page) struct PageMentionMenuInput {
    pub(super) block_id: String,
    pub(super) text: String,
    pub(super) cursor: usize,
    pub(super) is_composing: bool,
    pub(super) supported: bool,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page) enum PageMentionPickerAction {
    Dismiss,
    FieldChanged(String),
    SubmitField,
    SelectDay(chrono::NaiveDate),
    ShowToday,
    ShiftMonth(i32),
    ToggleEndDate,
    ToggleIncludeTime,
    ToggleSubmenu(crate::ui::PageMentionPickerSubmenu),
    SetDateFormat(PageMentionDateFormat),
    SetTimeFormat(PageMentionTimeFormat),
    SetReminder(Option<PageMentionReminder>),
    Clear,
}

pub(super) enum PageMentionEffect {
    OpenPickerAtAtom {
        block_id: String,
        atom_index: usize,
        anchor: Bounds<Pixels>,
    },

    EnsurePeople,
    SchedulePages(PageMentionPageSchedule),
    BeginPageSearch {
        generation: u64,
        query: String,
    },
    Mutation(Box<PageMentionMutationEffect>),
}

pub(super) enum PageMentionMutationEffect {
    CommitMenu(PageMentionMenuCommit),
    Picker(PageMentionPickerEffect),
    InviteUnavailable,
}

pub(super) enum PageMentionPickerEffect {
    Commit(PageMentionPickerState),
    Clear(PageMentionPickerState),
}

pub(super) struct PageMentionMenuCommit {
    pub(super) menu: PageMentionMenuState,
    pub(super) row: PageMentionMenuRow,
}

#[derive(Default)]
pub(super) struct PageMentionActionOutcome {
    pub(super) effects: Vec<PageMentionEffect>,
    pub(super) notify: bool,
    pub(super) changed: bool,
    pub(super) consumed: bool,
}

pub(in crate::ui::board_workspace::page) struct PageMentionDispatchResult {
    pub(in crate::ui::board_workspace::page::editor) changed: bool,
    pub(in crate::ui::board_workspace::page::editor) consumed: bool,
}

impl PageMentionAction {
    pub(in crate::ui::board_workspace::page::editor) fn reconcile(
        documents: &PageDocuments,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> Self {
        let supported = documents
            .page_containing_block(block_id)
            .and_then(|page| {
                page.blocks
                    .into_iter()
                    .find(|block| block.block_id == block_id)
            })
            .and_then(|block| {
                block.editable_content().map(|editable| {
                    editable.read_only.is_none()
                        && VerifiedPageTextBlockKind::parse(editable.kind).is_some()
                })
            })
            .unwrap_or(false);
        Self::ReconcileMenu(PageMentionMenuInput {
            block_id: block_id.to_string(),
            text: snapshot.text.clone(),
            cursor: snapshot.cursor,
            is_composing: snapshot.is_composing,
            supported,
        })
    }

    pub(in crate::ui::board_workspace::page::editor) fn commit(block_id: &str) -> Self {
        Self::CommitSelection {
            block_id: block_id.to_string(),
        }
    }
}

impl PageMentionActionOutcome {
    pub(super) fn changed(changed: bool) -> Self {
        Self {
            notify: changed,
            changed,
            ..Self::default()
        }
    }

    pub(super) fn effect(effect: PageMentionEffect) -> Self {
        Self {
            effects: vec![effect],
            ..Self::default()
        }
    }

    fn consumed(effect: PageMentionEffect) -> Self {
        Self {
            effects: vec![effect],
            consumed: true,
            ..Self::default()
        }
    }
}

impl PageMentionController {
    pub(super) fn reduce(
        &mut self,
        action: PageMentionAction,
        clock: &PageMentionClock,
        cx: &mut App,
    ) -> PageMentionActionOutcome {
        match action {
            PageMentionAction::ReconcileMenu(input) => self.reconcile_menu(input),
            PageMentionAction::OpenMenu(menu) => self.open_menu(menu),
            PageMentionAction::SearchDelayElapsed { generation, query } => {
                if self.page_search_timer_is_current(generation) {
                    PageMentionActionOutcome::effect(PageMentionEffect::BeginPageSearch {
                        generation,
                        query,
                    })
                } else {
                    PageMentionActionOutcome::default()
                }
            }
            PageMentionAction::CommitSelection { block_id } => {
                self.commit_selected_menu_row(&block_id, clock)
            }
            PageMentionAction::MoveMenu { block_id, delta } => {
                PageMentionActionOutcome::changed(self.move_menu_selection(&block_id, delta, clock))
            }
            PageMentionAction::CloseMenu { block_id } => {
                PageMentionActionOutcome::changed(self.close_menu(&block_id))
            }
            PageMentionAction::DismissMenu(identity) => {
                PageMentionActionOutcome::changed(self.dismiss_menu(&identity))
            }
            PageMentionAction::HoverMenuRow { identity, index } => {
                PageMentionActionOutcome::changed(self.hover_menu_row(&identity, index, clock))
            }
            PageMentionAction::ActivateMenuRow { identity, row } => {
                self.activate_menu_row(&identity, *row)
            }
            PageMentionAction::OpenPickerAtAtom {
                block_id,
                atom_index,
                anchor,
            } => PageMentionActionOutcome::effect(PageMentionEffect::OpenPickerAtAtom {
                block_id,
                atom_index,
                anchor,
            }),
            PageMentionAction::Picker { identity, action } => {
                self.reduce_picker_action(&identity, action, clock, cx)
            }
        }
    }

    fn reconcile_menu(&mut self, input: PageMentionMenuInput) -> PageMentionActionOutcome {
        let next = self.next_menu(&input);
        if next.is_none()
            && self
                .menu
                .as_ref()
                .is_some_and(|menu| menu.block_id != input.block_id)
        {
            return PageMentionActionOutcome::default();
        }
        if self.menu == next {
            return PageMentionActionOutcome::default();
        }
        let query_changed = self.menu.as_ref().map(|menu| menu.query.as_str())
            != next.as_ref().map(|menu| menu.query.as_str());
        self.menu = next;
        let effects = self.menu_load_effects(query_changed);
        PageMentionActionOutcome {
            effects,
            changed: true,
            ..PageMentionActionOutcome::default()
        }
    }

    fn next_menu(&mut self, input: &PageMentionMenuInput) -> Option<PageMentionMenuState> {
        let token = if input.is_composing || !input.supported {
            None
        } else {
            mention_menu_token(&input.text, input.cursor)
        };
        if token.is_none()
            && self
                .menu_suppressed
                .as_ref()
                .is_some_and(|(block_id, _)| block_id == &input.block_id)
        {
            self.menu_suppressed = None;
        }
        token.and_then(|(trigger_offset, query)| {
            if self
                .menu_suppressed
                .as_ref()
                .is_some_and(|(block_id, offset)| {
                    block_id == &input.block_id && *offset == trigger_offset
                })
            {
                return None;
            }
            let selected_index = self
                .menu
                .as_ref()
                .filter(|menu| {
                    menu.block_id == input.block_id
                        && menu.trigger_offset == trigger_offset
                        && menu.query == query
                })
                .map_or(0, |menu| menu.selected_index);
            Some(PageMentionMenuState {
                block_id: input.block_id.clone(),
                trigger_offset,
                query,
                selected_index,
            })
        })
    }

    fn open_menu(&mut self, menu: PageMentionMenuState) -> PageMentionActionOutcome {
        self.menu_suppressed = None;
        self.menu = Some(menu);
        PageMentionActionOutcome {
            effects: self.menu_load_effects(true),
            changed: true,
            ..PageMentionActionOutcome::default()
        }
    }

    fn menu_load_effects(&mut self, refresh_pages: bool) -> Vec<PageMentionEffect> {
        let Some(query) = self.menu.as_ref().map(|menu| menu.query.clone()) else {
            return Vec::new();
        };
        let mut effects = vec![PageMentionEffect::EnsurePeople];
        if refresh_pages {
            if let Some(schedule) = self.refresh_pages(&query) {
                effects.push(PageMentionEffect::SchedulePages(schedule));
            }
        }
        effects
    }

    fn commit_selected_menu_row(
        &mut self,
        block_id: &str,
        clock: &PageMentionClock,
    ) -> PageMentionActionOutcome {
        let Some(menu) = self.menu_for_block(block_id).cloned() else {
            return PageMentionActionOutcome::default();
        };
        let Some(row) = self.selected_row(&menu, clock) else {
            self.menu = None;
            return PageMentionActionOutcome {
                notify: true,
                changed: true,
                consumed: true,
                ..PageMentionActionOutcome::default()
            };
        };
        PageMentionActionOutcome::consumed(PageMentionEffect::Mutation(Box::new(
            PageMentionMutationEffect::CommitMenu(PageMentionMenuCommit { menu, row }),
        )))
    }

    fn hover_menu_row(
        &mut self,
        identity: &PageMentionMenuIdentity,
        index: usize,
        clock: &PageMentionClock,
    ) -> bool {
        let Some(menu) = self.menu.as_ref().filter(|menu| identity.matches(menu)) else {
            return false;
        };
        let row_count: usize = self
            .sections(menu, clock)
            .iter()
            .map(|s| s.rows.len())
            .sum();
        if index >= row_count || menu.selected_index == index {
            return false;
        }
        self.menu
            .as_mut()
            .expect("mention menu remains open")
            .selected_index = index;
        true
    }

    fn activate_menu_row(
        &mut self,
        identity: &PageMentionMenuIdentity,
        row: PageMentionMenuRow,
    ) -> PageMentionActionOutcome {
        let Some(menu) = self
            .menu
            .as_ref()
            .filter(|menu| identity.matches(menu))
            .cloned()
        else {
            return PageMentionActionOutcome::default();
        };
        if matches!(row, PageMentionMenuRow::Invite { .. }) {
            self.menu = None;
            return PageMentionActionOutcome {
                effects: vec![PageMentionEffect::Mutation(Box::new(
                    PageMentionMutationEffect::InviteUnavailable,
                ))],
                notify: true,
                changed: true,
                ..PageMentionActionOutcome::default()
            };
        }
        PageMentionActionOutcome::effect(PageMentionEffect::Mutation(Box::new(
            PageMentionMutationEffect::CommitMenu(PageMentionMenuCommit { menu, row }),
        )))
    }
}
