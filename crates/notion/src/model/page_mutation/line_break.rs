use super::{
    validate_text_selection_block_ids, validate_text_selection_structural_mutations,
    NotionPageBlockKind, PageBlockStructuralMutation, PageMutationEffect,
    PageTextSelectionEndpoint,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageTextLineBreak {
    Enter {
        new_block_id: String,
        new_block_kind: NotionPageBlockKind,
    },
    ShiftEnter,
}

#[derive(Clone, Debug)]
pub struct BreakPageTextSelectionRequest {
    start: PageTextSelectionEndpoint,
    end: PageTextSelectionEndpoint,
    selected_block_ids: Vec<String>,
    line_break: PageTextLineBreak,
    structural_mutations: Vec<PageBlockStructuralMutation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageTextCaretTarget {
    pub block_id: String,
    pub offset_utf8: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakPageTextSelectionEffect {
    Enter {
        effects: Vec<PageMutationEffect>,
        caret: PageTextCaretTarget,
    },
    ShiftEnter {
        effects: Vec<PageMutationEffect>,
        caret: PageTextCaretTarget,
    },
}

impl BreakPageTextSelectionRequest {
    pub fn new(
        start: PageTextSelectionEndpoint,
        end: PageTextSelectionEndpoint,
        selected_block_ids: Vec<String>,
        line_break: PageTextLineBreak,
        structural_mutations: Vec<PageBlockStructuralMutation>,
    ) -> Result<Self, String> {
        if start.block_id == end.block_id {
            return Err("a cross-block line break requires distinct endpoint blocks".to_string());
        }
        if let PageTextLineBreak::Enter { new_block_id, .. } = &line_break {
            if new_block_id == &start.block_id || new_block_id == &end.block_id {
                return Err("a cross-block Enter requires a fresh destination block".to_string());
            }
        }
        validate_text_selection_block_ids(&start, &end, &selected_block_ids)?;
        validate_text_selection_structural_mutations(&structural_mutations)?;
        Ok(Self {
            start,
            end,
            selected_block_ids,
            line_break,
            structural_mutations,
        })
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

    pub fn line_break(&self) -> &PageTextLineBreak {
        &self.line_break
    }

    pub fn structural_mutations(&self) -> &[PageBlockStructuralMutation] {
        &self.structural_mutations
    }
}

impl BreakPageTextSelectionEffect {
    pub fn caret(&self) -> &PageTextCaretTarget {
        match self {
            Self::Enter { caret, .. } | Self::ShiftEnter { caret, .. } => caret,
        }
    }

    pub(crate) fn created_block_id(&self) -> Option<&str> {
        match self {
            Self::Enter { caret, .. } => Some(&caret.block_id),
            Self::ShiftEnter { .. } => None,
        }
    }
}
