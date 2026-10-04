use super::{
    CardPageAliasBlock, CardPageBlock, CardPageBlockContent, CardPageEditableBlock,
    CardPageLayoutBlock, CardPageResourceBlock, CardPageStructuralBlock,
    CardPageUnsupportedLeafBlock,
};

impl CardPageBlock {
    pub const fn editable_content(&self) -> Option<&CardPageEditableBlock> {
        match &self.content {
            CardPageBlockContent::Editable(editable) => Some(editable),
            CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn editable_content_mut(&mut self) -> Option<&mut CardPageEditableBlock> {
        match &mut self.content {
            CardPageBlockContent::Editable(editable) => Some(editable),
            CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn alias_content(&self) -> Option<&CardPageAliasBlock> {
        match &self.content {
            CardPageBlockContent::Alias(alias) => Some(alias),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub fn alias_content_mut(&mut self) -> Option<&mut CardPageAliasBlock> {
        match &mut self.content {
            CardPageBlockContent::Alias(alias) => Some(alias),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn structural_content(&self) -> Option<&CardPageStructuralBlock> {
        match &self.content {
            CardPageBlockContent::Structural(structural) => Some(structural),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn layout_content(&self) -> Option<&CardPageLayoutBlock> {
        match &self.content {
            CardPageBlockContent::Layout(layout) => Some(layout),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn resource_content(&self) -> Option<&CardPageResourceBlock> {
        match &self.content {
            CardPageBlockContent::Resource(resource) => Some(resource),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn unsupported_leaf_content(&self) -> Option<&CardPageUnsupportedLeafBlock> {
        match &self.content {
            CardPageBlockContent::UnsupportedLeaf(unsupported) => Some(unsupported),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. }
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }
}
