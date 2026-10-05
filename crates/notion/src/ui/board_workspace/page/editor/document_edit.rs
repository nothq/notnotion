use gpui::ClipboardItem;
use gpui_components::text_input::{floor_grapheme_boundary, TextInputSnapshot};
use serde::Serialize;

use super::editing::{PageEditEffect, PageEditSession};
use super::{Arc, CardPage, CardPageBlockKind};

mod actions;
mod line_break;
mod replacement;
mod selection_parse;

pub(super) use actions::{
    page_block_on_pre_mutation_action, PageTextPreMutationAction, PageTextPreMutationActionSink,
    PageTextPreMutationRequest, PageTextPreMutationResponse,
};
pub(super) use line_break::CrossBlockPageLineBreak;
pub(crate) use replacement::PagePendingCrossBlockComposition;

#[derive(Clone)]
pub(super) struct CrossBlockPageTextSelection {
    pub(super) page: CardPage,
    pub(super) original_selection: crate::ui::PageTextSelection,
    pub(super) selected_indices: Vec<usize>,
    pub(super) collapsed_hidden_owner_indices: Arc<[Option<usize>]>,
    pub(super) first_index: usize,
    pub(super) last_index: usize,
    pub(super) first_offset: usize,
    pub(super) last_offset: usize,
}

impl CrossBlockPageTextSelection {
    fn clipboard_item(&self) -> Result<ClipboardItem, String> {
        let blocks = self.page.blocks[self.first_index..=self.last_index]
            .iter()
            .filter(|block| block.is_editable())
            .collect::<Vec<_>>();
        let first_depth = i64::try_from(blocks[0].depth)
            .map_err(|_| "The selected block depth is too large to copy.".to_string())?;
        let mut fragments = Vec::with_capacity(blocks.len());
        let mut metadata_blocks = Vec::with_capacity(blocks.len());
        for (relative_index, block) in blocks.iter().enumerate() {
            let editable = block
                .editable_content()
                .expect("validated text selection block must be editable");
            let fragment = if relative_index == 0 {
                &editable.text[self.first_offset..]
            } else if relative_index + 1 == blocks.len() {
                &editable.text[..self.last_offset]
            } else {
                editable.text.as_str()
            };
            fragments.push(fragment);
            let depth = i64::try_from(block.depth)
                .map_err(|_| "The selected block depth is too large to copy.".to_string())?;
            metadata_blocks.push(PageTextClipboardBlock {
                kind: editable.kind,
                relative_depth: depth - first_depth,
                selected_utf8_bytes: fragment.len(),
            });
        }
        let metadata = PageTextClipboardMetadata {
            schema: "notnotion.notion.page_editor.page_text_selection",
            version: 1,
            blocks: metadata_blocks,
            endpoints: PageTextClipboardEndpoints {
                start_offset_utf8: self.first_offset,
                end_offset_utf8: self.last_offset,
            },
        };
        Ok(ClipboardItem::new_string_with_json_metadata(
            fragments.join("\n"),
            metadata,
        ))
    }
}

impl PageEditSession<'_> {
    fn prepare_cross_block_page_text_selection(
        &mut self,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> Result<Option<CrossBlockPageTextSelection>, ()> {
        CrossBlockPageTextSelection::parse(self.editor, self.documents, block_id, snapshot).map_err(
            |error| {
                self.effects.push(PageEditEffect::Error(error));
                self.effects.push(PageEditEffect::Notify);
            },
        )
    }
}

fn validate_endpoint(text: &str, offset: usize, label: &str) -> Result<(), String> {
    if floor_grapheme_boundary(text, offset) == offset {
        return Ok(());
    }
    Err(format!(
        "The {label} of the page text selection is not a valid character boundary."
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageTextClipboardMetadata {
    schema: &'static str,
    version: u8,
    blocks: Vec<PageTextClipboardBlock>,
    endpoints: PageTextClipboardEndpoints,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageTextClipboardBlock {
    kind: CardPageBlockKind,
    relative_depth: i64,
    selected_utf8_bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageTextClipboardEndpoints {
    start_offset_utf8: usize,
    end_offset_utf8: usize,
}
