use std::sync::Arc;

use crate::ui::board_workspace::{PageDocumentColumn, PageDocumentLayout};
use crate::ui::surface::PageDocuments;
use crate::ui::{CardPage, CardPeekState, LoadedCardPageData};

type PageDataAndFlowSurface = (
    Arc<LoadedCardPageData>,
    crate::ui::surface::PageFlowSurfaceKey,
);

impl PageDocuments {
    /// The column of the page `page_id` where these documents show it; a page
    /// they do not show lays out as a default full page.
    pub(in crate::ui) fn hosted_page_column(
        &self,
        page_id: &str,
        main_pane_width: f32,
    ) -> PageDocumentColumn {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == page_id {
                return PageDocumentColumn::new(
                    PageDocumentLayout::SelectedPage,
                    page.data.page.format,
                    main_pane_width,
                );
            }
        }
        let format = self
            .standalone
            .as_ref()
            .filter(|page| page.data.page.block_id == page_id)
            .map(|page| page.data.page.format)
            .unwrap_or_default();
        PageDocumentColumn::new(PageDocumentLayout::Standalone, format, main_pane_width)
    }

    pub(in crate::ui) fn page_containing_block(&self, block_id: &str) -> Option<CardPage> {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page
                .data
                .page
                .blocks
                .iter()
                .any(|block| block.block_id == block_id)
            {
                return Some(page.data.page.clone());
            }
        }
        self.standalone
            .as_ref()
            .filter(|page| {
                page.data
                    .page
                    .blocks
                    .iter()
                    .any(|block| block.block_id == block_id)
            })
            .map(|page| page.data.page.clone())
    }

    pub(in crate::ui) fn page_data_containing_editable_block(
        &self,
        block_id: &str,
    ) -> Option<Arc<LoadedCardPageData>> {
        self.page_data_and_flow_surface_containing_editable_block(block_id)
            .map(|(data, _)| data)
    }

    pub(in crate::ui) fn page_data_and_flow_surface_containing_editable_block(
        &self,
        block_id: &str,
    ) -> Option<PageDataAndFlowSurface> {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.editable_block_indices.contains_key(block_id) {
                return Some((
                    page.data.clone(),
                    crate::ui::surface::PageFlowSurfaceKey::SelectedPage,
                ));
            }
        }
        self.standalone
            .as_ref()
            .filter(|page| page.data.editable_block_indices.contains_key(block_id))
            .map(|page| {
                (
                    page.data.clone(),
                    crate::ui::surface::PageFlowSurfaceKey::Standalone,
                )
            })
    }

    pub(in crate::ui) fn page_with_id(&self, page_id: &str) -> Option<CardPage> {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == page_id {
                return Some(page.data.page.clone());
            }
        }
        self.standalone
            .as_ref()
            .filter(|page| page.data.page.block_id == page_id)
            .map(|page| page.data.page.clone())
    }

    pub(in crate::ui) fn page_authority_with_id(&self, page_id: &str) -> Option<Arc<CardPage>> {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == page_id {
                return Some(page.data.authority_arc());
            }
        }
        self.standalone
            .as_ref()
            .filter(|page| page.data.page.block_id == page_id)
            .map(|page| page.data.authority_arc())
    }

    pub(in crate::ui) fn loaded_page_data_with_id(
        &self,
        page_id: &str,
    ) -> Option<Arc<LoadedCardPageData>> {
        self.selected_page
            .as_ref()
            .and_then(|selected| match selected {
                CardPeekState::Loaded(loaded) if loaded.data.page.block_id == page_id => {
                    Some(loaded.data.clone())
                }
                _ => None,
            })
            .or_else(|| {
                self.standalone
                    .as_ref()
                    .filter(|loaded| loaded.data.page.block_id == page_id)
                    .map(|loaded| loaded.data.clone())
            })
    }

    pub(in crate::ui) fn loaded_page_has_column_structure(&self, page_id: &str) -> bool {
        self.selected_page.as_ref().is_some_and(|page| match page {
            CardPeekState::Loaded(page) => {
                page.data.page.block_id == page_id && page.data.has_column_structure()
            }
            CardPeekState::Loading { .. } | CardPeekState::Error { .. } => false,
        }) || self.standalone.as_ref().is_some_and(|page| {
            page.data.page.block_id == page_id && page.data.has_column_structure()
        })
    }

    pub(in crate::ui) fn active_page_data_with_id(
        &self,
        page_id: &str,
    ) -> Option<Arc<LoadedCardPageData>> {
        match self.selected_page.as_ref() {
            Some(CardPeekState::Loaded(page)) if page.data.page.block_id == page_id => {
                Some(page.data.clone())
            }
            Some(_) => None,
            None => self
                .standalone
                .as_ref()
                .filter(|page| page.data.page.block_id == page_id)
                .map(|page| page.data.clone()),
        }
    }
}
