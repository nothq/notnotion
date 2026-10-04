use crate::ui::{
    CardPageBlockColor, CardPageBlockColorValue, CardPageBlockKind, LoadedCardPageData,
};

use super::{
    HashMap, HashSet, PageBlockContextMenuState, PageBlockDragAutoScroll, PageBlockDragLayouts,
    PageBlockDragTarget, PageBlockFocusRequest, PageBlockSelection, PageCodeSyntaxCache,
    PageComposerHandoff, PageComposerState, PageEditHistory, PageForcedTextAnnotations,
    PageLinkIconController, PagePendingCrossBlockComposition, PagePendingRichTextComposition,
    PagePendingRichTextTyping, PageRichTextDialog, PageSlashMenuState, PageTextSelection, RefCell,
    ScrollHandle, TextInput, VecDeque,
};

mod column_resize;
mod defaults;
mod flow_observations;
mod flow_virtualizer;
mod inputs;
mod reset;
mod runtime;
mod simple_table;

use inputs::PageTextInputRegistry;
pub(crate) use inputs::{PageBlockInputPropsKey, PageDocumentFocusRegistryEntry};
pub(crate) use runtime::{
    PageDocumentDragRuntime, PageDocumentFlowRuntime, PageInputResources, PageInputRuntime,
    PageSimpleTableRuntime,
};

pub(crate) use column_resize::{
    PageColumnResizeGeometry, PageColumnResizeOrigin, PageColumnResizeSession,
    PageColumnResizeSource,
};
pub(crate) use flow_observations::{
    PageFlowAnchorTarget, PageFlowCapturedSemanticAnchor, PageFlowDragAuthority,
    PageFlowDropAnchor, PageFlowLaneObservation, PageFlowLayoutAuthority, PageFlowMountAuthority,
    PageFlowNodeObservation, PageFlowObservationToken, PageFlowObservations,
    PageFlowUnitObservation,
};

pub(crate) use flow_virtualizer::{
    allocate_page_flow_lane_widths, PageFlowExactLayoutWidth, PageFlowLayoutCommit,
    PageFlowLayoutCommitSchedule, PageFlowLayoutFrameToken, PageFlowLayoutWidth,
    PageFlowLogicalViewportBasis, PageFlowOuterExtentAuthority, PageFlowOuterItemExtent,
    PageFlowOuterItemMatch, PageFlowOuterItemRemap, PageFlowOuterLocalY, PageFlowPinSet,
    PageFlowPinTarget, PageFlowRenderGeneration, PageFlowRenderSpan, PageFlowSectionMeasurement,
    PageFlowSemanticAnchorBaseline, PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorProjection,
    PageFlowSemanticAnchorTransaction, PageFlowSequenceWidth, PageFlowSurfaceKey, PageFlowViewport,
    PageFlowViewportRelativeY, PageFlowVirtualizer,
};

pub(crate) use simple_table::{
    PageSimpleTableCellCompositionBaseline, PageSimpleTableCellEditorState,
    PageSimpleTableCellFocusMode, PageSimpleTableCellForcedAnnotations,
    PageSimpleTableCellGeneration, PageSimpleTableCellObservedReplacement,
    PageSimpleTableCellReplacementOrigin, PageSimpleTableCellReplacementPlan,
};

#[derive(Clone)]
pub(crate) struct PageFlowCommittedFocus {
    pub(crate) authority: PageFlowMountAuthority,
    pub(crate) target: PageFlowPinTarget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageToggleDisclosure {
    Collapsed,
    Expanded,
}

impl PageToggleDisclosure {
    pub(crate) fn is_expanded(self) -> bool {
        self == Self::Expanded
    }
}

#[derive(Clone, Default)]
pub(crate) struct PageToggleDisclosureState {
    expanded_block_ids_by_page: HashMap<String, HashSet<String>>,
}

impl PageToggleDisclosureState {
    pub(crate) fn disclosure(&self, page_id: &str, block_id: &str) -> PageToggleDisclosure {
        if self
            .expanded_block_ids_by_page
            .get(page_id)
            .is_some_and(|block_ids| block_ids.contains(block_id))
        {
            PageToggleDisclosure::Expanded
        } else {
            PageToggleDisclosure::Collapsed
        }
    }

    pub(crate) fn toggle(&mut self, page_id: &str, block_id: &str) -> PageToggleDisclosure {
        let was_expanded = self
            .expanded_block_ids_by_page
            .get_mut(page_id)
            .is_some_and(|block_ids| block_ids.remove(block_id));
        if was_expanded {
            if self
                .expanded_block_ids_by_page
                .get(page_id)
                .is_some_and(HashSet::is_empty)
            {
                self.expanded_block_ids_by_page.remove(page_id);
            }
            PageToggleDisclosure::Collapsed
        } else {
            self.expanded_block_ids_by_page
                .entry(page_id.to_string())
                .or_default()
                .insert(block_id.to_string());
            PageToggleDisclosure::Expanded
        }
    }

    pub(crate) fn expanded_block_ids(&self, page_id: &str) -> Option<&HashSet<String>> {
        self.expanded_block_ids_by_page.get(page_id)
    }

    pub(crate) fn expand(&mut self, page_id: &str, block_id: &str) -> bool {
        self.expanded_block_ids_by_page
            .entry(page_id.to_string())
            .or_default()
            .insert(block_id.to_string())
    }

    pub(crate) fn retain_valid(&mut self, data: &LoadedCardPageData) {
        let Some(expanded_ids) = self.expanded_block_ids_by_page.get_mut(&data.page.block_id)
        else {
            return;
        };
        expanded_ids.retain(|block_id| {
            data.editable_block_indices
                .get(block_id)
                .and_then(|index| data.page.blocks[*index].editable_content())
                .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
        });
        if expanded_ids.is_empty() {
            self.expanded_block_ids_by_page.remove(&data.page.block_id);
        }
    }
}

const PAGE_SIMPLE_TABLE_SCROLL_HANDLE_LIMIT: usize = 64;

#[derive(Default)]
struct PageSimpleTableScrollState {
    handles_by_page: HashMap<String, HashMap<String, ScrollHandle>>,
    insertion_order: VecDeque<(String, String)>,
}

impl PageSimpleTableScrollState {
    fn handle(&mut self, page_id: &str, table_block_id: &str) -> ScrollHandle {
        if let Some(handle) = self
            .handles_by_page
            .get(page_id)
            .and_then(|handles| handles.get(table_block_id))
            .cloned()
        {
            return handle;
        }

        let handle = ScrollHandle::new();
        let page_id = page_id.to_string();
        let table_block_id = table_block_id.to_string();
        self.handles_by_page
            .entry(page_id.clone())
            .or_default()
            .insert(table_block_id.clone(), handle.clone());
        self.insertion_order.push_back((page_id, table_block_id));
        while self.insertion_order.len() > PAGE_SIMPLE_TABLE_SCROLL_HANDLE_LIMIT {
            let Some((oldest_page_id, oldest_table_id)) = self.insertion_order.pop_front() else {
                break;
            };
            let remove_page =
                self.handles_by_page
                    .get_mut(&oldest_page_id)
                    .is_some_and(|handles| {
                        handles.remove(&oldest_table_id);
                        handles.is_empty()
                    });
            if remove_page {
                self.handles_by_page.remove(&oldest_page_id);
            }
        }
        handle
    }

    fn retain_valid(&mut self, data: &LoadedCardPageData) {
        let valid_table_ids = data
            .page
            .blocks
            .iter()
            .filter(|block| block.simple_table_content().is_some())
            .map(|block| block.block_id.as_str())
            .collect::<HashSet<_>>();
        let remove_page = self
            .handles_by_page
            .get_mut(&data.page.block_id)
            .is_some_and(|handles| {
                handles.retain(|block_id, _| valid_table_ids.contains(block_id.as_str()));
                handles.is_empty()
            });
        if remove_page {
            self.handles_by_page.remove(&data.page.block_id);
        }
        let handles_by_page = &self.handles_by_page;
        self.insertion_order.retain(|(page_id, table_id)| {
            handles_by_page
                .get(page_id)
                .is_some_and(|handles| handles.contains_key(table_id))
        });
    }
}

pub(crate) struct PageEditorState {
    pub(crate) input: PageInputRuntime,
    pub(crate) flow: PageDocumentFlowRuntime,
    pub(crate) drag: PageDocumentDragRuntime,
    pub(crate) tables: PageSimpleTableRuntime,
    pub(crate) active_page_block: Option<String>,
    pub(crate) hovered_page_block: Option<String>,
    pub(crate) page_block_compositions: HashSet<String>,
    pub(crate) page_block_context_menu: Option<PageBlockContextMenuState>,
    pub(crate) page_link_icons: PageLinkIconController,
    pub(crate) last_used_page_block_color: CardPageBlockColor,
    pub(crate) page_toggle_disclosure: PageToggleDisclosureState,
    pub(crate) page_slash_menu: Option<PageSlashMenuState>,
    pub(crate) mention: crate::ui::board_workspace::PageMentionController,
    pub(crate) page_text_selection: Option<PageTextSelection>,
    pub(crate) page_forced_text_annotations: Option<PageForcedTextAnnotations>,
    pub(crate) page_pending_rich_text_typing: Option<PagePendingRichTextTyping>,
    pub(crate) page_pending_rich_text_composition: Option<PagePendingRichTextComposition>,
    pub(crate) page_pending_cross_block_composition: Option<PagePendingCrossBlockComposition>,
    pub(crate) page_rich_text_dialog: Option<PageRichTextDialog>,
    pub(crate) page_rich_text_link_value: String,
    pub(crate) page_rich_text_link_input: RefCell<Option<gpui::Entity<TextInput>>>,
    pub(crate) page_block_selection: PageBlockSelection,
    pub(crate) page_edit_histories: HashMap<String, PageEditHistory>,
}

impl PageEditorState {
    pub(crate) fn inherit_session_state(&mut self, previous: &Self) {
        self.page_link_icons
            .inherit_session_state(&previous.page_link_icons);
        self.last_used_page_block_color = previous.last_used_page_block_color;
        self.page_toggle_disclosure = previous.page_toggle_disclosure.clone();
    }

    pub(crate) fn take_cached_state_from(&mut self, previous: &mut Self) {
        self.input.take_cached_state_from(&mut previous.input);
        self.flow.inherit_cached_state(&previous.flow);
        self.tables = previous.tables.clone();
        self.active_page_block = previous.active_page_block.take();
        self.hovered_page_block = previous.hovered_page_block.take();
        self.page_block_compositions = std::mem::take(&mut previous.page_block_compositions);
        self.page_block_context_menu = previous.page_block_context_menu.take();
        self.page_link_icons
            .take_cached_state_from(&mut previous.page_link_icons);
        self.last_used_page_block_color = previous.last_used_page_block_color;
        self.page_slash_menu = previous.page_slash_menu.take();
        self.page_text_selection = previous.page_text_selection.take();
        self.page_forced_text_annotations = previous.page_forced_text_annotations.take();
        self.page_pending_rich_text_typing = previous.page_pending_rich_text_typing.take();
        self.page_pending_rich_text_composition =
            previous.page_pending_rich_text_composition.take();
        self.page_pending_cross_block_composition =
            previous.page_pending_cross_block_composition.take();
        self.page_rich_text_dialog = previous.page_rich_text_dialog.take();
        self.page_rich_text_link_value = std::mem::take(&mut previous.page_rich_text_link_value);
        self.page_rich_text_link_input = std::mem::take(&mut previous.page_rich_text_link_input);
        self.page_block_selection = std::mem::take(&mut previous.page_block_selection);
        self.page_edit_histories = std::mem::take(&mut previous.page_edit_histories);
    }

    pub(crate) fn retain_valid_simple_table_scroll_handles(&self, data: &LoadedCardPageData) {
        self.tables.retain_valid_scroll_handles(data);
    }
}
