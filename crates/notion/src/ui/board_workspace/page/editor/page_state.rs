use gpui::App;

use super::projection::PageEditorDragReset;
use super::support::replace_loaded_card_page;
use super::{Arc, CardPage, CardPeekState, LoadedCardPage};
use crate::model::BoardSnapshot;
use crate::ui::board_workspace::PageFocusSession;
use crate::ui::surface::{PageDocuments, PageEditorState};

mod flow_transition;
mod lookup;

use flow_transition::LoadedPageFlowTransition;

pub(in crate::ui) struct LoadedPageReplacement {
    page_id: String,
    visible_projection_changed: bool,
    standalone_replaced: bool,
}

impl LoadedPageReplacement {
    pub(in crate::ui) fn page_id(&self) -> &str {
        &self.page_id
    }

    pub(in crate::ui) fn standalone_replaced(&self) -> bool {
        self.standalone_replaced
    }
}

impl BoardSnapshot {
    pub(in crate::ui) fn synchronize_page_title(&mut self, page: &CardPage) {
        for card in self
            .columns
            .iter_mut()
            .flat_map(|column| column.cards.iter_mut())
            .filter(|card| card.block_id == page.block_id)
        {
            card.title.clone_from(&page.title);
        }
        for item in self
            .items
            .iter_mut()
            .filter(|item| item.block_id == page.block_id)
        {
            item.title.clone_from(&page.title);
        }
    }
}

impl PageEditorState {
    pub(in crate::ui) fn loaded_card_page_with_disclosure(&self, page: CardPage) -> LoadedCardPage {
        let expanded_toggle_ids = self
            .page_toggle_disclosure
            .expanded_block_ids(&page.block_id);
        LoadedCardPage::with_expanded_toggles(page, expanded_toggle_ids)
    }

    pub(in crate::ui) fn loaded_card_page_with_authority_and_disclosure(
        &self,
        page: CardPage,
        authority: Arc<CardPage>,
    ) -> LoadedCardPage {
        let expanded_toggle_ids = self
            .page_toggle_disclosure
            .expanded_block_ids(&page.block_id);
        LoadedCardPage::with_authority(page, authority, expanded_toggle_ids)
    }

    pub(in crate::ui) fn replace_loaded_page_references(
        &self,
        documents: &mut PageDocuments,
        page: &CardPage,
        authority: Option<&Arc<CardPage>>,
    ) -> LoadedPageReplacement {
        let expanded = self
            .page_toggle_disclosure
            .expanded_block_ids(&page.block_id);
        documents.replace_loaded_page_references(page, authority, expanded)
    }

    pub(in crate::ui) fn reconcile_loaded_page_replacement(
        &mut self,
        documents: &mut PageDocuments,
        replacement: &LoadedPageReplacement,
        cx: &mut App,
    ) {
        self.reconcile_loaded_page_flow_surfaces(
            &replacement.page_id,
            documents.loaded_page_flow_surfaces(&replacement.page_id),
        );
        if let Some(data) = documents.loaded_page_data_with_id(&replacement.page_id) {
            self.page_toggle_disclosure.retain_valid(&data);
            self.retain_valid_simple_table_scroll_handles(&data);
        }
        if !replacement.visible_projection_changed {
            return;
        }
        if let Some(data) = documents.active_page_data_with_id(&replacement.page_id) {
            PageFocusSession::new(self, documents).reconcile_projection(&data, None, cx);
        }
    }

    pub(in crate::ui) fn advance_loaded_page_authority(
        &mut self,
        documents: &mut PageDocuments,
        page_id: &str,
        authority: Arc<CardPage>,
    ) {
        let transitions = documents.advance_authority(page_id, authority);
        self.apply_loaded_page_flow_transitions(transitions);
        if documents.loaded_page_has_column_structure(page_id) {
            self.reset_drag(PageEditorDragReset::CancelHandleInteraction);
        }
    }
}

fn replacement_authority(
    page: &LoadedCardPage,
    authority: Option<&Arc<CardPage>>,
) -> Arc<CardPage> {
    authority
        .cloned()
        .unwrap_or_else(|| page.data.authority_arc())
}

impl PageDocuments {
    fn replace_selected_page_reference(
        &mut self,
        page: &CardPage,
        page_id: &str,
        authority: Option<&Arc<CardPage>>,
        expanded: Option<&std::collections::HashSet<String>>,
    ) -> bool {
        let Some(CardPeekState::Loaded(selected_page)) = self.selected_page.as_mut() else {
            return false;
        };
        if selected_page.data.page.block_id != page_id {
            return false;
        }
        let authority = replacement_authority(selected_page, authority);
        replace_loaded_card_page(selected_page, page.clone(), authority, expanded)
    }

    fn replace_loaded_page_references(
        &mut self,
        page: &CardPage,
        authority: Option<&Arc<CardPage>>,
        expanded: Option<&std::collections::HashSet<String>>,
    ) -> LoadedPageReplacement {
        let page_id = page.block_id.clone();
        let selected_changed =
            self.replace_selected_page_reference(page, &page_id, authority, expanded);
        let standalone_changed =
            self.replace_standalone_page_reference(page, &page_id, authority, expanded);
        LoadedPageReplacement {
            page_id,
            visible_projection_changed: selected_changed || standalone_changed.unwrap_or(false),
            standalone_replaced: standalone_changed.is_some(),
        }
    }

    fn replace_standalone_page_reference(
        &mut self,
        page: &CardPage,
        page_id: &str,
        authority: Option<&Arc<CardPage>>,
        expanded: Option<&std::collections::HashSet<String>>,
    ) -> Option<bool> {
        let standalone_page = self.standalone.as_mut()?;
        if standalone_page.data.page.block_id != page_id {
            return None;
        }
        let authority = replacement_authority(standalone_page, authority);
        let changed = replace_loaded_card_page(standalone_page, page.clone(), authority, expanded);
        Some(changed)
    }

    fn advance_authority(
        &mut self,
        page_id: &str,
        authority: Arc<CardPage>,
    ) -> Vec<LoadedPageFlowTransition> {
        let mut transitions = Vec::new();
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_mut() {
            if page.data.page.block_id == page_id {
                let was_recursive = page.data.has_column_structure();
                Arc::make_mut(&mut page.data).advance_authority(authority.clone());
                transitions.push(LoadedPageFlowTransition {
                    surface: crate::ui::surface::PageFlowSurfaceKey::SelectedPage,
                    was_recursive,
                    page: page.data.clone(),
                    list_state: page.list_state.clone(),
                });
            }
        }
        if let Some(page) = self.standalone.as_mut() {
            if page.data.page.block_id == page_id {
                let was_recursive = page.data.has_column_structure();
                Arc::make_mut(&mut page.data).advance_authority(authority);
                transitions.push(LoadedPageFlowTransition {
                    surface: crate::ui::surface::PageFlowSurfaceKey::Standalone,
                    was_recursive,
                    page: page.data.clone(),
                    list_state: page.list_state.clone(),
                });
            }
        }
        transitions
    }
}
