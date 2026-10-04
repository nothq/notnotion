use std::collections::{HashMap, HashSet};

use super::{PageBlockStructuralMutation, ReorderPageBlockSubtreesRequest};
use crate::model::{CardPage, CardPageLayoutBlock};

pub(crate) struct ColumnSafePageBlockMutationSequence {
    mutations: Vec<PageBlockStructuralMutation>,
}

impl ColumnSafePageBlockMutationSequence {
    pub(crate) fn new(
        page: &CardPage,
        mutations: Vec<PageBlockStructuralMutation>,
    ) -> Result<Self, String> {
        validate_column_safe_page_block_mutations(page, &mutations)?;
        Ok(Self { mutations })
    }

    pub(crate) fn into_mutations(self) -> Vec<PageBlockStructuralMutation> {
        self.mutations
    }
}

pub(crate) fn validate_column_safe_page_block_mutations(
    page: &CardPage,
    mutations: &[PageBlockStructuralMutation],
) -> Result<(), String> {
    let mut ownership = PageBlockOwnership::new(page);
    for mutation in mutations {
        match mutation {
            PageBlockStructuralMutation::Reorder(request) => ownership.apply_reorder(request)?,
            PageBlockStructuralMutation::Delete(request) => {
                ownership.apply_deletion(&request.block_id)?
            }
        }
    }
    Ok(())
}

impl ReorderPageBlockSubtreesRequest {
    pub(crate) fn validate_column_sources(&self, page: &CardPage) -> Result<(), String> {
        PageBlockOwnership::new(page).apply_reorder(self)
    }
}

struct PageBlockOwnership {
    parent_by_block: HashMap<String, String>,
    children_by_parent: HashMap<String, HashSet<String>>,
    column_child_counts: HashMap<String, usize>,
    protected_columns: HashSet<String>,
}

impl PageBlockOwnership {
    fn new(page: &CardPage) -> Self {
        let mut column_child_counts = page
            .blocks
            .iter()
            .filter(|block| {
                matches!(
                    block.layout_content(),
                    Some(CardPageLayoutBlock::Column { .. })
                )
            })
            .map(|block| (block.block_id.clone(), 0))
            .collect::<HashMap<_, _>>();
        for block in &page.blocks {
            if let Some(count) = column_child_counts.get_mut(&block.parent_block_id) {
                *count += 1;
            }
        }
        let parent_by_block = page
            .blocks
            .iter()
            .map(|block| (block.block_id.clone(), block.parent_block_id.clone()))
            .collect::<HashMap<_, _>>();
        let mut children_by_parent = HashMap::<String, HashSet<String>>::new();
        for (block_id, parent_id) in &parent_by_block {
            children_by_parent
                .entry(parent_id.clone())
                .or_default()
                .insert(block_id.clone());
        }
        Self {
            parent_by_block,
            children_by_parent,
            column_child_counts,
            protected_columns: HashSet::new(),
        }
    }

    fn apply_reorder(&mut self, request: &ReorderPageBlockSubtreesRequest) -> Result<(), String> {
        let moved = request.block_ids.iter().collect::<HashSet<_>>();
        if moved.len() != request.block_ids.len() {
            return Err("a subtree reorder sequence contains duplicate block ids".to_string());
        }
        let outgoing = self.outgoing_counts(request)?;
        self.validate_reorder_counts(&outgoing)?;
        self.apply_counts(&outgoing, request);
        for block_id in &request.block_ids {
            self.reparent_block(block_id, &request.target_parent_block_id);
        }
        Ok(())
    }

    fn apply_deletion(&mut self, block_id: &str) -> Result<(), String> {
        let source_parent = self
            .parent_by_block
            .get(block_id)
            .cloned()
            .ok_or_else(|| format!("page block sequence does not contain block {block_id}"))?;
        if let Some(child_count) = self.column_child_counts.get_mut(&source_parent) {
            let remaining = child_count.checked_sub(1).ok_or_else(|| {
                format!("column {source_parent} has inconsistent direct content ownership")
            })?;
            if remaining == 0 && self.protected_columns.contains(&source_parent) {
                return Err(format!(
                    "page block sequence would leave source column {source_parent} without content"
                ));
            }
            *child_count = remaining;
        }
        let deleted_ids = self.subtree_ids(block_id);
        let removed = self
            .children_by_parent
            .get_mut(&source_parent)
            .is_some_and(|children| children.remove(block_id));
        debug_assert!(removed, "parent adjacency must contain every owned block");
        for deleted_id in deleted_ids {
            self.parent_by_block.remove(&deleted_id);
            self.children_by_parent.remove(&deleted_id);
            self.column_child_counts.remove(&deleted_id);
            self.protected_columns.remove(&deleted_id);
        }
        Ok(())
    }

    fn outgoing_counts(
        &self,
        request: &ReorderPageBlockSubtreesRequest,
    ) -> Result<HashMap<String, usize>, String> {
        let mut outgoing = HashMap::new();
        for block_id in &request.block_ids {
            let source_parent = self.parent_by_block.get(block_id).ok_or_else(|| {
                format!("page reorder sequence does not contain block {block_id}")
            })?;
            if source_parent != &request.target_parent_block_id {
                *outgoing.entry(source_parent.clone()).or_insert(0) += 1;
            }
        }
        Ok(outgoing)
    }

    fn validate_reorder_counts(&mut self, outgoing: &HashMap<String, usize>) -> Result<(), String> {
        for (source_parent, moved_count) in outgoing {
            let Some(child_count) = self.column_child_counts.get(source_parent) else {
                continue;
            };
            if child_count
                .checked_sub(*moved_count)
                .is_none_or(|count| count == 0)
            {
                return Err(format!(
                    "subtree reorder sequence would leave column {source_parent} without content"
                ));
            }
            self.protected_columns.insert(source_parent.clone());
        }
        Ok(())
    }

    fn subtree_ids(&self, root_id: &str) -> HashSet<String> {
        let mut subtree = HashSet::from([root_id.to_string()]);
        let mut pending = vec![root_id.to_string()];
        while let Some(parent_id) = pending.pop() {
            for child_id in self
                .children_by_parent
                .get(&parent_id)
                .into_iter()
                .flatten()
            {
                if subtree.insert(child_id.clone()) {
                    pending.push(child_id.clone());
                }
            }
        }
        subtree
    }

    fn reparent_block(&mut self, block_id: &str, target_parent: &str) {
        let source_parent = self
            .parent_by_block
            .get(block_id)
            .expect("validated reorder block must remain owned")
            .clone();
        if source_parent == target_parent {
            return;
        }
        let removed = self
            .children_by_parent
            .get_mut(&source_parent)
            .is_some_and(|children| children.remove(block_id));
        debug_assert!(removed, "parent adjacency must contain every owned block");
        self.children_by_parent
            .entry(target_parent.to_string())
            .or_default()
            .insert(block_id.to_string());
        self.parent_by_block
            .insert(block_id.to_string(), target_parent.to_string());
    }

    fn apply_counts(
        &mut self,
        outgoing: &HashMap<String, usize>,
        request: &ReorderPageBlockSubtreesRequest,
    ) {
        let moved_count = outgoing.values().sum::<usize>();
        for (source_parent, count) in outgoing {
            if let Some(child_count) = self.column_child_counts.get_mut(source_parent) {
                *child_count -= *count;
            }
        }
        if let Some(child_count) = self
            .column_child_counts
            .get_mut(&request.target_parent_block_id)
        {
            *child_count += moved_count;
        }
    }
}
