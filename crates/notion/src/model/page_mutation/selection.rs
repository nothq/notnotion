use std::collections::HashSet;

use super::{PageBlockStructuralMutation, PageMutationEffect};

#[derive(Clone, Debug)]
pub struct ReplacePageTextSelectionRequest {
    start: PageTextSelectionEndpoint,
    end: PageTextSelectionEndpoint,
    selected_block_ids: Vec<String>,
    replacement: String,
    action: PageTextSelectionAction,
    structural_mutations: Vec<PageBlockStructuralMutation>,
}

#[derive(Clone, Debug)]
pub struct PastePageTextSelectionRequest {
    start: PageTextSelectionEndpoint,
    end: PageTextSelectionEndpoint,
    new_block_id: String,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageTextSelectionEndpoint {
    pub block_id: String,
    pub offset_utf8: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageTextSelectionAction {
    TextMutation,
    Paste,
    Cut,
}

/// The text that replaces a cross-block selection and the action that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageTextSelectionEdit {
    pub replacement: String,
    pub action: PageTextSelectionAction,
}

impl PastePageTextSelectionRequest {
    pub fn new(
        start: PageTextSelectionEndpoint,
        end: PageTextSelectionEndpoint,
        new_block_id: String,
        text: String,
    ) -> Result<Self, String> {
        if new_block_id == start.block_id || new_block_id == end.block_id {
            return Err("a multiline paste requires a fresh destination block".to_string());
        }
        if !text.contains('\n') {
            return Err("a multiline paste requires a line break".to_string());
        }
        Ok(Self {
            start,
            end,
            new_block_id,
            text,
        })
    }

    pub fn start(&self) -> &PageTextSelectionEndpoint {
        &self.start
    }

    pub fn end(&self) -> &PageTextSelectionEndpoint {
        &self.end
    }

    pub fn new_block_id(&self) -> &str {
        &self.new_block_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl ReplacePageTextSelectionRequest {
    pub fn new(
        start: PageTextSelectionEndpoint,
        end: PageTextSelectionEndpoint,
        selected_block_ids: Vec<String>,
        edit: PageTextSelectionEdit,
        structural_mutations: Vec<PageBlockStructuralMutation>,
    ) -> Result<Self, String> {
        let PageTextSelectionEdit {
            replacement,
            action,
        } = edit;
        if start.block_id == end.block_id {
            return Err(
                "a cross-block text selection requires distinct endpoint blocks".to_string(),
            );
        }
        if action == PageTextSelectionAction::Cut && !replacement.is_empty() {
            return Err("a cross-block cut cannot insert replacement text".to_string());
        }
        validate_text_selection_block_ids(&start, &end, &selected_block_ids)?;
        validate_text_selection_structural_mutations(&structural_mutations)?;
        Ok(Self {
            start,
            end,
            selected_block_ids,
            replacement,
            action,
            structural_mutations,
        })
    }

    pub(crate) fn set_replacement(&mut self, replacement: String) {
        self.replacement = replacement;
    }

    pub fn start(&self) -> &PageTextSelectionEndpoint {
        &self.start
    }

    pub fn end(&self) -> &PageTextSelectionEndpoint {
        &self.end
    }

    pub fn selected_block_ids(&self) -> &[String] {
        &self.selected_block_ids
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub fn action(&self) -> PageTextSelectionAction {
        self.action
    }

    pub fn structural_mutations(&self) -> &[PageBlockStructuralMutation] {
        &self.structural_mutations
    }
}

pub(super) fn validate_text_selection_structural_mutations(
    structural_mutations: &[PageBlockStructuralMutation],
) -> Result<(), String> {
    if structural_mutations.is_empty() {
        return Err("a cross-block text selection must remove at least one block".to_string());
    }
    let mut deletion_started = false;
    for mutation in structural_mutations {
        match mutation {
            PageBlockStructuralMutation::Reorder(_) if deletion_started => {
                return Err(
                    "text-selection block deletions must be the final structural mutations"
                        .to_string(),
                );
            }
            PageBlockStructuralMutation::Reorder(_) => {}
            PageBlockStructuralMutation::Delete(_) => deletion_started = true,
        }
    }
    Ok(())
}

pub(super) fn validate_text_selection_block_ids(
    start: &PageTextSelectionEndpoint,
    end: &PageTextSelectionEndpoint,
    block_ids: &[String],
) -> Result<(), String> {
    if block_ids.len() < 2 {
        return Err(
            "a cross-block text selection requires at least two selected blocks".to_string(),
        );
    }
    if block_ids.first() != Some(&start.block_id) || block_ids.last() != Some(&end.block_id) {
        return Err(
            "cross-block text-selection endpoints must match the selected block sequence"
                .to_string(),
        );
    }
    let mut unique = HashSet::with_capacity(block_ids.len());
    if let Some(block_id) = block_ids
        .iter()
        .find(|block_id| !unique.insert(block_id.as_str()))
    {
        return Err(format!(
            "cross-block text selection contains duplicate block {block_id}"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacePageTextSelectionEffect {
    pub effects: Vec<PageMutationEffect>,
}
