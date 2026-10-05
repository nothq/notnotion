use std::sync::Arc;

use gpui::App;

use super::{
    render::{
        new_page_link_icon_picker_ui, page_link_picker_category_index_for_item,
        page_link_picker_emoji_item_count, page_link_picker_emoji_section_item_index,
        page_link_picker_named_icon_item_count,
    },
    PageLinkIconAction, PageLinkIconController, PageLinkIconPickerState, PageLinkIconPickerTab,
    PageLinkIconTarget,
};
use crate::{
    model::NotionCustomEmojiLibrary,
    ui::{notion_named_icon_matches, view_actions::ViewActionSink, NotionNamedIconSlug, Theme},
};

mod state;

use state::{
    apply_custom_emoji_library, new_page_link_icon_picker_state, reset_page_link_icon_picker,
};

pub(super) struct PageLinkIconPickerUi {
    pub(super) search_input: gpui::Entity<gpui_components::text_input::TextInput>,
    pub(super) upload_name_input: gpui::Entity<gpui_components::text_input::TextInput>,
    pub(super) emoji_list_state: gpui::ListState,
    pub(super) named_icon_list_state: gpui::ListState,
    pub(super) named_icon_matches: Arc<[NotionNamedIconSlug]>,
    pub(super) recent_named_icons: Arc<[NotionNamedIconSlug]>,
}

#[derive(Clone)]
pub(super) struct PageLinkCustomEmojiLibraryRequest {
    pub(super) instance_id: u64,
    pub(super) page_id: String,
    pub(super) block_id: String,
    pub(super) generation: u64,
}

pub(super) enum PageLinkCustomEmojiLibraryResolution {
    Stale,
    Applied,
    Failed(crate::model::NotionWorkspaceOperationFailure),
}

#[derive(Clone, Copy)]
pub(super) enum PageLinkIconToggleResult {
    Ignored,
    Closed,
    Opened,
}

impl PageLinkIconController {
    pub(super) fn toggle_picker(
        &mut self,
        target: PageLinkIconTarget,
        theme: Theme,
        actions: ViewActionSink<PageLinkIconAction>,
        cx: &mut App,
    ) -> PageLinkIconToggleResult {
        let PageLinkIconTarget { page_id, block_id } = target;
        let picker = self.picker.as_ref();
        if picker.is_some_and(|picker| picker.upload_committed)
            || self.commit_in_flight(&page_id, &block_id)
        {
            return PageLinkIconToggleResult::Ignored;
        }
        if picker.is_some_and(|picker| picker.page_id == page_id && picker.block_id == block_id) {
            self.picker = None;
            return PageLinkIconToggleResult::Closed;
        }

        let ui = new_page_link_icon_picker_ui(theme, actions, &self.recent_named_icons, cx);
        let instance_id = self.next_picker_instance_id;
        self.next_picker_instance_id = instance_id
            .checked_add(1)
            .expect("page-link icon picker instance ID overflowed");
        self.picker = Some(new_page_link_icon_picker_state(
            instance_id,
            page_id,
            block_id,
            self.named_icon_preference,
            ui,
        ));
        PageLinkIconToggleResult::Opened
    }

    pub(super) fn begin_custom_emoji_library_load(
        &mut self,
    ) -> Option<PageLinkCustomEmojiLibraryRequest> {
        let picker = self.picker.as_mut()?;
        if picker.custom_emoji_library_pending {
            return None;
        }
        picker.custom_emoji_library_generation =
            picker.custom_emoji_library_generation.wrapping_add(1);
        picker.custom_emoji_library_pending = true;
        Some(PageLinkCustomEmojiLibraryRequest {
            instance_id: picker.instance_id,
            page_id: picker.page_id.clone(),
            block_id: picker.block_id.clone(),
            generation: picker.custom_emoji_library_generation,
        })
    }

    pub(super) fn finish_custom_emoji_library_load(
        &mut self,
        request: &PageLinkCustomEmojiLibraryRequest,
        result: crate::model::NotionWorkspaceResult<NotionCustomEmojiLibrary>,
    ) -> PageLinkCustomEmojiLibraryResolution {
        let Some(picker) = self.picker.as_mut() else {
            return PageLinkCustomEmojiLibraryResolution::Stale;
        };
        if !picker.matches_instance(request.instance_id, &request.page_id, &request.block_id)
            || picker.custom_emoji_library_generation != request.generation
        {
            return PageLinkCustomEmojiLibraryResolution::Stale;
        }
        picker.custom_emoji_library_pending = false;
        match result {
            Ok(library) => {
                apply_custom_emoji_library(picker, library);
                PageLinkCustomEmojiLibraryResolution::Applied
            }
            Err(error) => PageLinkCustomEmojiLibraryResolution::Failed(error),
        }
    }

    pub(super) fn set_query(&mut self, query: String) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.query == query {
            return false;
        }
        let emoji_item_count = page_link_picker_emoji_item_count(&query, &picker.custom_emojis);
        let named_icon_matches = notion_named_icon_matches(&query);
        let named_icon_item_count = page_link_picker_named_icon_item_count(
            named_icon_matches.len(),
            picker.recent_named_icons.len(),
            query.trim().is_empty(),
        );
        picker.query = query;
        picker.control_menu = None;
        picker.category_index = 0;
        picker.emoji_list_state.reset(emoji_item_count);
        picker.named_icon_matches = named_icon_matches;
        picker.named_icon_list_state.reset(named_icon_item_count);
        true
    }

    pub(super) fn set_tab(&mut self, tab: PageLinkIconPickerTab, cx: &mut App) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab == tab || picker.upload_committed {
            return false;
        }
        picker.tab = tab;
        reset_page_link_icon_picker(picker, cx);
        true
    }

    pub(super) fn jump_category(&mut self, section_index: usize, category_index: usize) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Emoji || !picker.query.trim().is_empty() {
            return false;
        }
        let Some(item_ix) = page_link_picker_emoji_section_item_index(section_index) else {
            return false;
        };
        picker.category_index = category_index;
        picker.emoji_list_state.scroll_to(gpui::ListOffset {
            item_ix,
            offset_in_item: gpui::px(0.0),
        });
        true
    }

    pub(super) fn sync_category_from_scroll(&mut self, visible_item_index: usize) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Emoji || !picker.query.trim().is_empty() {
            return false;
        }
        let Some(category_index) = page_link_picker_category_index_for_item(visible_item_index)
        else {
            return false;
        };
        if picker.category_index == category_index {
            return false;
        }
        picker.category_index = category_index;
        true
    }

    pub(super) fn dismiss_layer(&mut self) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.control_menu.take().is_some() {
            return true;
        }
        self.dismiss_picker()
    }

    pub(super) fn picker_state(
        &self,
        page_id: &str,
        block_id: &str,
    ) -> Option<PageLinkIconPickerState> {
        self.picker
            .as_ref()
            .filter(|state| state.page_id == page_id && state.block_id == block_id)
            .cloned()
    }
}
