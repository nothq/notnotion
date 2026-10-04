use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use super::{
    CardPageBlockColor, CardPageBlockColorValue, PageBlockSelection, PageDocumentDragRuntime,
    PageDocumentFlowRuntime, PageEditorState, PageInputRuntime, PageLinkIconController,
    PageSimpleTableRuntime, PageToggleDisclosureState,
};

impl Default for PageEditorState {
    fn default() -> Self {
        Self {
            input: PageInputRuntime::default(),
            flow: PageDocumentFlowRuntime::default(),
            drag: PageDocumentDragRuntime::default(),
            tables: PageSimpleTableRuntime::default(),
            active_page_block: None,
            hovered_page_block: None,
            page_block_compositions: HashSet::new(),
            page_block_context_menu: None,
            page_link_icons: PageLinkIconController::default(),
            last_used_page_block_color: CardPageBlockColor::Background(
                CardPageBlockColorValue::Default,
            ),
            page_toggle_disclosure: PageToggleDisclosureState::default(),
            page_slash_menu: None,
            mention: Default::default(),
            page_text_selection: None,
            page_forced_text_annotations: None,
            page_pending_rich_text_typing: None,
            page_pending_rich_text_composition: None,
            page_pending_cross_block_composition: None,
            page_rich_text_dialog: None,
            page_rich_text_link_value: String::new(),
            page_rich_text_link_input: RefCell::new(None),
            page_block_selection: PageBlockSelection::default(),
            page_edit_histories: HashMap::new(),
        }
    }
}
