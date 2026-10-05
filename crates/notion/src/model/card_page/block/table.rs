use super::{
    CardPageBlock, CardPageBlockContent, CardPageSimpleTableBlock, CardPageSimpleTableRowBlock,
};

impl CardPageBlock {
    pub fn simple_table(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        table: CardPageSimpleTableBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::Table { table },
        )
    }

    pub fn simple_table_row(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
        table_row: CardPageSimpleTableRowBlock,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::TableRow { table_row },
        )
    }

    pub const fn simple_table_content(&self) -> Option<&CardPageSimpleTableBlock> {
        match &self.content {
            CardPageBlockContent::Table { table } => Some(table),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::TableRow { .. } => None,
        }
    }

    pub const fn simple_table_row_content(&self) -> Option<&CardPageSimpleTableRowBlock> {
        match &self.content {
            CardPageBlockContent::TableRow { table_row } => Some(table_row),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. } => None,
        }
    }

    pub(crate) fn simple_table_row_content_mut(
        &mut self,
    ) -> Option<&mut CardPageSimpleTableRowBlock> {
        match &mut self.content {
            CardPageBlockContent::TableRow { table_row } => Some(table_row),
            CardPageBlockContent::Editable(_)
            | CardPageBlockContent::Alias(_)
            | CardPageBlockContent::Structural(_)
            | CardPageBlockContent::Resource(_)
            | CardPageBlockContent::UnsupportedLeaf(_)
            | CardPageBlockContent::OpaqueUnavailable { .. }
            | CardPageBlockContent::Layout(_)
            | CardPageBlockContent::Table { .. } => None,
        }
    }
}
