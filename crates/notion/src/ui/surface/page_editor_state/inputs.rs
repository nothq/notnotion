use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Weak},
};

use gpui::FocusHandle;

use gpui_components::text_input::TextInput;

use crate::model::{CardPageBlockColor, CardPageEditableBlock};
use crate::ui::{AppearanceMode, LoadedCardPageData, PageDocumentOuterItemId};

pub(crate) type PageTextInputRegistry = RefCell<HashMap<String, gpui::Entity<TextInput>>>;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PageBlockInputPropsKey {
    pub(crate) editable: CardPageEditableBlock,
    pub(crate) color: CardPageBlockColor,
    pub(crate) format: crate::model::CardPageFormat,
    pub(crate) appearance_mode: AppearanceMode,
    pub(crate) slash_menu_active: bool,
    pub(crate) mention_menu_active: bool,
    pub(crate) today: chrono::NaiveDate,
    pub(crate) mention_ghost: Option<String>,
    pub(crate) request_focus: bool,
    pub(crate) show_placeholder: bool,
}

pub(crate) struct PageDocumentFocusRegistryEntry {
    pub(crate) allocation: Weak<LoadedCardPageData>,
    pub(crate) projection_generation: u64,
    pub(crate) order: Arc<[PageDocumentOuterItemId]>,
    pub(crate) ordered_handles: Arc<[FocusHandle]>,
    pub(crate) handles_by_item: HashMap<PageDocumentOuterItemId, FocusHandle>,
}

impl PageDocumentFocusRegistryEntry {
    pub(crate) fn matches_projection(&self, data: &Arc<LoadedCardPageData>) -> bool {
        Weak::ptr_eq(&self.allocation, &Arc::downgrade(data))
            && self.projection_generation == data.flow_projection_generation
    }
}
