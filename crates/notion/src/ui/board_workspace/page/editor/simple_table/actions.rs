use gpui::App;
use gpui_components::text_input::TextInputSnapshot;

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::surface::{PageSimpleTableCellFocusMode, PageSimpleTableCellGeneration};

use super::super::editing::PageEditSession;
use super::{PageSimpleTableFormatAction, PageSimpleTableHostEffect};

pub(in crate::ui::board_workspace) enum PageSimpleTableEditAction {
    Activate {
        page_id: String,
        address: CardPageSimpleTableCellAddress,
        focus: PageSimpleTableCellFocusMode,
    },
    FinishBlur {
        address: CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    },
    Changed {
        page_id: String,
        address: CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: TextInputSnapshot,
    },
    Format(PageSimpleTableFormatAction),
    Remeasure {
        page_id: String,
        address: CardPageSimpleTableCellAddress,
    },
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn apply_page_simple_table_action(
        &mut self,
        action: PageSimpleTableEditAction,
        cx: &mut App,
    ) -> Option<PageSimpleTableHostEffect> {
        match action {
            PageSimpleTableEditAction::Activate {
                page_id,
                address,
                focus,
            } => {
                self.activate_page_simple_table_cell(&page_id, address, focus, cx);
                None
            }
            PageSimpleTableEditAction::FinishBlur {
                address,
                generation,
            } => self.finish_page_simple_table_cell_blur(&address, generation),
            PageSimpleTableEditAction::Changed {
                page_id,
                address,
                generation,
                snapshot,
            } => self.apply_page_simple_table_cell_change(&page_id, &address, generation, snapshot),
            PageSimpleTableEditAction::Format(action) => {
                self.apply_page_simple_table_format(action);
                None
            }
            PageSimpleTableEditAction::Remeasure { page_id, address } => {
                self.remeasure_page_table_cell_layout(&page_id, &address);
                None
            }
        }
    }
}
