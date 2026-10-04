use serde::{Deserialize, Serialize};

use super::{
    CardPageBlockApiType, CardPageBlockColor, CardPageBlockKind, CardPageCodeSettings,
    CardPageColumnRatio, CardPageEditableBlock, CardPageResourceBlock, CardPageSimpleTableBlock,
    CardPageSimpleTableRowBlock, CardPageUnsupportedLeafBlock, PageShellIcon,
};

mod access;
mod opaque;
mod table;

pub use opaque::CardPageOpaqueUnavailableBlock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "structural", rename_all = "snake_case")]
pub enum CardPageStructuralBlock {
    Divider,
    CollectionView { title: Option<String> },
    CollectionViewPage { title: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageAliasBlock {
    pub target_block_id: String,
    pub target_space_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copied_from_block_id: Option<String>,
    pub title: String,
    pub icon: PageShellIcon,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "layout", rename_all = "snake_case")]
pub enum CardPageLayoutBlock {
    ColumnList,
    Column {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ratio: Option<CardPageColumnRatio>,
    },
    Passthrough {
        api_type: CardPageBlockApiType,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CardPageBlockContent {
    Editable(CardPageEditableBlock),
    Alias(CardPageAliasBlock),
    Structural(CardPageStructuralBlock),
    Resource(CardPageResourceBlock),
    UnsupportedLeaf(CardPageUnsupportedLeafBlock),
    OpaqueUnavailable {
        opaque_unavailable: CardPageOpaqueUnavailableBlock,
    },
    Layout(CardPageLayoutBlock),
    Table {
        table: CardPageSimpleTableBlock,
    },
    TableRow {
        table_row: CardPageSimpleTableRowBlock,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageBlock {
    pub block_id: String,
    pub parent_block_id: String,
    pub depth: usize,
    #[serde(default)]
    pub color: CardPageBlockColor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PageShellIcon>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_edited: Option<CardPageBlockLastEdited>,
    #[serde(flatten)]
    pub content: CardPageBlockContent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageBlockLastEdited {
    pub editor_name: String,
    pub timestamp_ms: u64,
}

impl CardPageBlock {
    pub fn editable(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        kind: CardPageBlockKind,
        text: impl Into<String>,
    ) -> Self {
        Self::from_editable(
            block_id,
            parent_block_id,
            depth,
            CardPageEditableBlock::new(kind, text, Vec::new()),
        )
    }

    pub(crate) fn from_editable(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        editable: CardPageEditableBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Editable(editable),
        )
    }

    pub(crate) fn code(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        settings: CardPageCodeSettings,
    ) -> Self {
        Self::from_editable(
            block_id,
            parent_block_id,
            depth,
            CardPageEditableBlock::code_with_settings("", Vec::new(), settings),
        )
    }

    pub fn structural(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        structural: CardPageStructuralBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Structural(structural),
        )
    }

    pub fn alias(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        alias: CardPageAliasBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Alias(alias),
        )
    }

    pub fn layout(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        layout: CardPageLayoutBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Layout(layout),
        )
    }

    pub fn resource(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        resource: CardPageResourceBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Resource(resource),
        )
    }

    pub fn unsupported_leaf(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        unsupported: CardPageUnsupportedLeafBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::UnsupportedLeaf(unsupported),
        )
    }

    fn new(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        content: CardPageBlockContent,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            parent_block_id: parent_block_id.into(),
            depth,
            color: CardPageBlockColor::default(),
            icon: None,
            last_edited: None,
            content,
        }
    }

    pub const fn is_layout_container(&self) -> bool {
        matches!(&self.content, CardPageBlockContent::Layout(_))
    }

    pub const fn is_editable(&self) -> bool {
        matches!(&self.content, CardPageBlockContent::Editable(_))
    }

    pub const fn can_accept_children(&self) -> bool {
        matches!(
            self.editable_content(),
            Some(editable)
                if !matches!(
                    editable.kind,
                    CardPageBlockKind::PageLink | CardPageBlockKind::Code
                )
        )
    }

    pub const fn accepts_content_children(&self) -> bool {
        self.can_accept_children()
            || matches!(
                self.layout_content(),
                Some(CardPageLayoutBlock::Column { .. } | CardPageLayoutBlock::Passthrough { .. })
            )
    }
}
