use crate::model::{
    CardPage, CardPageBlock, CardPageBlockContent, CardPageBlockKind, CardPageCodeLanguage,
    CardPageCodeWrap, CardPageLayoutBlock, CardPageResourceBlock, CardPageSimpleTableBlock,
    CardPageSimpleTableRowBlock, CardPageStructuralBlock, CardPageUnsupportedLeafBlock,
};

use super::order::PageOrderConstraint;
use super::{FailedPageWriteDisposition, PageWriteReplayError};

pub(super) struct FixedBlockPostcondition {
    block_id: String,
    parent_id: String,
    content: FixedBlockContent,
    order: Option<PageOrderConstraint>,
}

enum FixedBlockContent {
    Editable {
        kind: CardPageBlockKind,
        text: String,
        code_language: Option<CardPageCodeLanguage>,
        code_wrap: Option<CardPageCodeWrap>,
    },
    Structural(CardPageStructuralBlock),
    Resource(CardPageResourceBlock),
    UnsupportedLeaf(CardPageUnsupportedLeafBlock),
    OpaqueUnavailable,
    Layout(CardPageLayoutBlock),
    SimpleTable(CardPageSimpleTableBlock),
    SimpleTableRow(CardPageSimpleTableRowBlock),
    Alias,
}

impl FixedBlockPostcondition {
    pub(super) fn new(
        block_id: &str,
        after: &CardPage,
        order: Option<PageOrderConstraint>,
    ) -> Result<Self, PageWriteReplayError> {
        let block = after
            .blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .ok_or_else(|| {
                PageWriteReplayError::invalid_request(
                    "fixed-ID postcondition",
                    format!("projected block {block_id} is missing"),
                )
            })?;
        Ok(Self {
            block_id: block_id.to_string(),
            parent_id: block.parent_block_id.clone(),
            content: FixedBlockContent::from(block),
            order,
        })
    }

    pub(super) fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        let Some(block) = authority
            .blocks
            .iter()
            .find(|block| block.block_id == self.block_id)
        else {
            return FailedPageWriteDisposition::NotApplied;
        };
        if self.matches(block)
            && self
                .order
                .as_ref()
                .is_none_or(|order| order.matches(authority))
        {
            FailedPageWriteDisposition::Applied
        } else {
            FailedPageWriteDisposition::Conflict
        }
    }

    fn matches(&self, block: &CardPageBlock) -> bool {
        block.parent_block_id == self.parent_id && self.content.matches(&block.content)
    }
}

impl FixedBlockContent {
    fn matches(&self, content: &CardPageBlockContent) -> bool {
        match (self, content) {
            (
                Self::Editable {
                    kind: expected_kind,
                    text: expected_text,
                    code_language: expected_language,
                    code_wrap: expected_wrap,
                },
                CardPageBlockContent::Editable(actual),
            ) => {
                actual.kind == *expected_kind
                    && actual.text == *expected_text
                    && actual.code_language() == expected_language.as_ref()
                    && actual.code_wrap() == *expected_wrap
            }
            (Self::Structural(expected), CardPageBlockContent::Structural(actual)) => {
                actual == expected
            }
            (Self::Resource(expected), CardPageBlockContent::Resource(actual)) => {
                actual == expected
            }
            (Self::UnsupportedLeaf(expected), CardPageBlockContent::UnsupportedLeaf(actual)) => {
                actual == expected
            }
            (Self::OpaqueUnavailable, CardPageBlockContent::OpaqueUnavailable { .. }) => true,
            (Self::Layout(expected), CardPageBlockContent::Layout(actual)) => actual == expected,
            (Self::SimpleTable(expected), CardPageBlockContent::Table { table }) => {
                table == expected
            }
            (Self::SimpleTableRow(expected), CardPageBlockContent::TableRow { table_row }) => {
                table_row == expected
            }
            (Self::Alias, CardPageBlockContent::Alias(_)) => true,
            _ => false,
        }
    }
}

impl From<&CardPageBlock> for FixedBlockContent {
    fn from(block: &CardPageBlock) -> Self {
        match &block.content {
            CardPageBlockContent::Editable(editable) => Self::Editable {
                kind: editable.kind,
                text: editable.text.clone(),
                code_language: editable.code_language().cloned(),
                code_wrap: editable.code_wrap(),
            },
            CardPageBlockContent::Structural(structural) => Self::Structural(structural.clone()),
            CardPageBlockContent::Resource(resource) => Self::Resource(resource.clone()),
            CardPageBlockContent::UnsupportedLeaf(unsupported) => {
                Self::UnsupportedLeaf(unsupported.clone())
            }
            CardPageBlockContent::OpaqueUnavailable { .. } => Self::OpaqueUnavailable,
            CardPageBlockContent::Layout(layout) => Self::Layout(layout.clone()),
            CardPageBlockContent::Table { table } => Self::SimpleTable(table.clone()),
            CardPageBlockContent::TableRow { table_row } => Self::SimpleTableRow(table_row.clone()),
            CardPageBlockContent::Alias(_) => Self::Alias,
        }
    }
}

pub(super) struct DeleteBlockPostcondition {
    block_id: String,
}

impl DeleteBlockPostcondition {
    pub(super) fn new(block_id: &str) -> Self {
        Self {
            block_id: block_id.to_string(),
        }
    }

    pub(super) fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        if authority
            .blocks
            .iter()
            .any(|block| block.block_id == self.block_id)
        {
            FailedPageWriteDisposition::NotApplied
        } else {
            FailedPageWriteDisposition::Applied
        }
    }
}
