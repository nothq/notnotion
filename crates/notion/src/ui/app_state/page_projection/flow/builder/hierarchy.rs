use std::{collections::HashMap, sync::Arc};

use crate::ui::CardPage;

use super::LoadedCardPageVisibleRow;

pub(super) struct BlockHierarchy {
    pub(super) root_children: Arc<[usize]>,
    pub(super) children: Vec<Arc<[usize]>>,
}

pub(super) fn build_block_hierarchy(page: &CardPage) -> BlockHierarchy {
    let mut root_children = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); page.blocks.len()];
    let mut preceding_indices: HashMap<&str, usize> = HashMap::with_capacity(page.blocks.len());
    for (block_index, block) in page.blocks.iter().enumerate() {
        if block.parent_block_id == page.block_id {
            root_children.push(block_index);
        } else {
            let parent_index = *preceding_indices
                .get(block.parent_block_id.as_str())
                .expect("validated Notion page hierarchy must be parent-closed");
            children[parent_index].push(block_index);
        }
        let previous = preceding_indices.insert(block.block_id.as_str(), block_index);
        assert!(previous.is_none(), "Notion block IDs must be unique");
    }
    BlockHierarchy {
        root_children: root_children.into(),
        children: children.into_iter().map(Arc::from).collect(),
    }
}

pub(super) fn visible_row_indices(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
) -> Vec<Option<usize>> {
    let mut indices = vec![None; page.blocks.len()];
    for (visible_row_index, row) in rows.iter().enumerate() {
        let slot = indices
            .get_mut(row.block_index)
            .expect("canonical visible row must reference an existing Notion block");
        assert!(
            slot.replace(visible_row_index).is_none(),
            "Notion block cannot own two canonical visible rows"
        );
    }
    indices
}
