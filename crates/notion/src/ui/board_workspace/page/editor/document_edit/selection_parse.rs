use gpui_components::text_input::TextInputSnapshot;

use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::selection::VisiblePageTextSelection;
use super::{validate_endpoint, CrossBlockPageTextSelection};
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::{CardPage, LoadedCardPageData, PageTextSelection};

impl CrossBlockPageTextSelection {
    pub(super) fn parse(
        editor: &PageEditorState,
        documents: &PageDocuments,
        active_block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> Result<Option<Self>, String> {
        let Some(selection) = active_selection(editor, active_block_id) else {
            return Ok(None);
        };
        if selection.pointer_active {
            return Err("Finish selecting page text before editing it.".to_string());
        }
        let data = documents
            .page_data_containing_editable_block(&selection.anchor_block_id)
            .ok_or_else(|| "The selected page text is no longer available.".to_string())?;
        let visible_selection = VisiblePageTextSelection::new(&data, selection)
            .ok_or_else(|| "The selected page text is no longer visible.".to_string())?;
        if visible_selection.anchor_index() == visible_selection.focus_index() {
            return Ok(None);
        }
        if !active_snapshot_is_current(&data.page, selection, active_block_id, snapshot)? {
            return Ok(None);
        }
        let ordered = ordered_endpoints(
            selection,
            visible_selection.anchor_index(),
            visible_selection.focus_index(),
        );
        let selected_indices = visible_selection.block_indices().collect::<Vec<_>>();
        validate_selection_range(&data, &ordered, &selected_indices)?;
        Ok(Some(Self {
            page: data.page.clone(),
            original_selection: selection.clone(),
            selected_indices,
            collapsed_hidden_owner_indices: data.collapsed_hidden_owner_indices.clone(),
            first_index: ordered.first_index,
            last_index: ordered.last_index,
            first_offset: ordered.first_offset,
            last_offset: ordered.last_offset,
        }))
    }
}

fn active_selection<'a>(
    editor: &'a PageEditorState,
    active_block_id: &str,
) -> Option<&'a PageTextSelection> {
    let selection = editor.page_text_selection.as_ref()?;
    let active_is_endpoint =
        active_block_id == selection.anchor_block_id || active_block_id == selection.focus_block_id;
    (editor.active_page_block.as_deref() == Some(active_block_id) && active_is_endpoint)
        .then_some(selection)
}

fn active_snapshot_is_current(
    page: &CardPage,
    selection: &PageTextSelection,
    active_block_id: &str,
    snapshot: &TextInputSnapshot,
) -> Result<bool, String> {
    let active_block = page
        .blocks
        .iter()
        .find(|block| block.block_id == active_block_id)
        .ok_or_else(|| "The active page text block is missing.".to_string())?;
    let active_text = &active_block
        .editable_content()
        .ok_or_else(|| "The active page block is not editable text.".to_string())?
        .text;
    Ok(snapshot.text == *active_text
        && (active_block_id != selection.focus_block_id
            || snapshot.cursor == selection.focus_offset))
}

struct OrderedEndpoints {
    first_index: usize,
    last_index: usize,
    first_offset: usize,
    last_offset: usize,
}

fn ordered_endpoints(
    selection: &PageTextSelection,
    anchor_index: usize,
    focus_index: usize,
) -> OrderedEndpoints {
    if anchor_index < focus_index {
        OrderedEndpoints {
            first_index: anchor_index,
            last_index: focus_index,
            first_offset: selection.anchor_offset,
            last_offset: selection.focus_offset,
        }
    } else {
        OrderedEndpoints {
            first_index: focus_index,
            last_index: anchor_index,
            first_offset: selection.focus_offset,
            last_offset: selection.anchor_offset,
        }
    }
}

fn validate_selection_range(
    data: &LoadedCardPageData,
    endpoints: &OrderedEndpoints,
    selected_indices: &[usize],
) -> Result<(), String> {
    validate_selected_text_blocks(data, selected_indices)?;
    if data.page.blocks[endpoints.first_index..=endpoints.last_index]
        .iter()
        .enumerate()
        .any(|(offset, block)| {
            !block.is_editable()
                && data.collapsed_hidden_owner_indices[endpoints.first_index + offset].is_none()
        })
    {
        return Err("Structural blocks cannot participate in a page text replacement.".to_string());
    }
    let first_text = &data.page.blocks[endpoints.first_index]
        .editable_content()
        .expect("validated text selection block must be editable")
        .text;
    let last_text = &data.page.blocks[endpoints.last_index]
        .editable_content()
        .expect("validated text selection block must be editable")
        .text;
    validate_endpoint(first_text, endpoints.first_offset, "start")?;
    validate_endpoint(last_text, endpoints.last_offset, "end")
}

fn validate_selected_text_blocks(
    data: &LoadedCardPageData,
    selected_indices: &[usize],
) -> Result<(), String> {
    for index in selected_indices {
        let editable = data.page.blocks[*index].editable_content().ok_or_else(|| {
            "Structural blocks cannot participate in a page text replacement.".to_string()
        })?;
        VerifiedPageTextBlockKind::parse(editable.kind).ok_or_else(|| {
            "Code and page-link blocks cannot participate in a cross-block text edit.".to_string()
        })?;
    }
    Ok(())
}
