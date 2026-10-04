use std::collections::{HashMap, HashSet};

use super::{
    validate_source_order, validated_insertion_after, SourceParentGroups,
    ValidatedSelectionDeletion, ValidatedSelectionReorder,
};
use crate::live::board::page_state::{NotionBlockRecord, PageMutationState};
use crate::model::{NotionPageBlockKind, ReorderPageBlockSubtreesRequest};

/// Moved blocks grouped by source parent, plus each block's source parent in block order.
type GroupedSourceParents = (SourceParentGroups, Vec<String>);

pub(super) struct SelectionStructureShadow {
    parent_by_block: HashMap<String, String>,
    content_ids_by_parent: HashMap<String, Vec<String>>,
    column_ids: HashSet<String>,
    protected_columns: HashSet<String>,
    deleted_block_ids: HashSet<String>,
}

impl SelectionStructureShadow {
    pub(super) fn new(state: &PageMutationState) -> Result<Self, String> {
        let mut parent_by_block = state
            .blocks
            .iter()
            .map(|(block_id, block)| (block_id.clone(), block.parent_id.clone()))
            .collect::<HashMap<_, _>>();
        let mut content_ids_by_parent = HashMap::new();
        for parent in state.blocks.values() {
            validate_authoritative_children(parent)?;
            content_ids_by_parent.insert(parent.id.clone(), parent.content_ids.clone());
            for child_id in &parent.content_ids {
                if let Some(existing) = parent_by_block.get(child_id) {
                    if existing != &parent.id {
                        return Err(format!(
                            "block {child_id} has conflicting authoritative direct parents"
                        ));
                    }
                } else {
                    parent_by_block.insert(child_id.clone(), parent.id.clone());
                }
            }
        }
        let column_ids = state
            .blocks
            .values()
            .filter(|block| {
                block.alive
                    && matches!(&block.kind, NotionPageBlockKind::Other(kind) if kind == "column")
            })
            .map(|block| block.id.clone())
            .collect();
        Ok(Self {
            parent_by_block,
            content_ids_by_parent,
            column_ids,
            protected_columns: HashSet::new(),
            deleted_block_ids: HashSet::new(),
        })
    }

    pub(super) fn plan_reorder(
        &mut self,
        state: &PageMutationState,
        request: &ReorderPageBlockSubtreesRequest,
    ) -> Result<ValidatedSelectionReorder, String> {
        let moved = self.validate_moved_roots(state, &request.block_ids)?;
        let (source_parent_groups, source_parent_ids) =
            self.source_parent_groups(&request.block_ids)?;
        self.validate_source_groups(state, &source_parent_groups, &moved)?;
        editable_parent(state, &request.target_parent_block_id)?;
        if self
            .deleted_block_ids
            .contains(&request.target_parent_block_id)
        {
            return Err("text-selection reorder targets a deleted block".to_string());
        }
        self.validate_target_outside_subtrees(&request.target_parent_block_id, &moved)?;
        self.validate_column_sources(&source_parent_groups, &request.target_parent_block_id)?;
        let insertion_after = validated_insertion_after(
            self.content_ids(&request.target_parent_block_id)?,
            &moved,
            &request.placement,
        )?;
        let plan = ValidatedSelectionReorder {
            source_parent_groups,
            source_parent_ids,
            target_parent_id: request.target_parent_block_id.clone(),
            block_ids: request.block_ids.clone(),
            insertion_after,
        };
        self.apply_reorder(&plan)?;
        Ok(plan)
    }

    pub(super) fn plan_deletion(
        &mut self,
        state: &PageMutationState,
        block_id: &str,
    ) -> Result<ValidatedSelectionDeletion, String> {
        if block_id == state.page_block_id {
            return Err("the page root cannot be retired by a text selection".to_string());
        }
        editable_record(state, block_id)?;
        if self.deleted_block_ids.contains(block_id) {
            return Err(format!(
                "text-selection block {block_id} is already deleted"
            ));
        }
        let parent_id = self.parent_id(block_id)?.to_string();
        editable_parent(state, &parent_id)?;
        if !self
            .content_ids(&parent_id)?
            .iter()
            .any(|child| child == block_id)
        {
            return Err(format!("block {block_id} is not a child of {parent_id}"));
        }
        self.validate_column_deletion(&parent_id)?;
        let plan = ValidatedSelectionDeletion {
            block_id: block_id.to_string(),
            parent_id,
        };
        self.apply_deletion(&plan)?;
        Ok(plan)
    }

    pub(super) fn into_final_parent_ids(self) -> HashMap<String, String> {
        self.parent_by_block
    }

    fn validate_moved_roots(
        &self,
        state: &PageMutationState,
        block_ids: &[String],
    ) -> Result<HashSet<String>, String> {
        if block_ids.is_empty() {
            return Err("a subtree move requires at least one block".to_string());
        }
        let moved = block_ids.iter().cloned().collect::<HashSet<_>>();
        if moved.len() != block_ids.len() {
            return Err("a subtree move contains duplicate block ids".to_string());
        }
        for block_id in block_ids {
            let block = editable_record(state, block_id)?;
            if block.id == state.page_block_id || block.parent_table != "block" {
                return Err(format!("block {block_id} cannot be moved as page content"));
            }
            if self.deleted_block_ids.contains(block_id) {
                return Err(format!(
                    "text-selection block {block_id} is already deleted"
                ));
            }
        }
        Ok(moved)
    }

    fn source_parent_groups(&self, block_ids: &[String]) -> Result<GroupedSourceParents, String> {
        let mut groups = SourceParentGroups::new();
        let mut group_indices = HashMap::<&str, usize>::new();
        let mut source_parent_ids = Vec::with_capacity(block_ids.len());
        for block_id in block_ids {
            let parent_id = self.parent_id(block_id)?;
            source_parent_ids.push(parent_id.to_string());
            let group_index = if let Some(index) = group_indices.get(parent_id).copied() {
                index
            } else {
                let index = groups.len();
                group_indices.insert(parent_id, index);
                groups.push((parent_id.to_string(), vec![block_id.clone()]));
                continue;
            };
            groups[group_index].1.push(block_id.clone());
        }
        Ok((groups, source_parent_ids))
    }

    fn validate_source_groups(
        &self,
        state: &PageMutationState,
        groups: &SourceParentGroups,
        moved: &HashSet<String>,
    ) -> Result<(), String> {
        for (parent_id, child_ids) in groups {
            editable_parent(state, parent_id)?;
            validate_source_order(self.content_ids(parent_id)?, child_ids, moved)?;
        }
        Ok(())
    }

    fn validate_target_outside_subtrees(
        &self,
        target_parent_id: &str,
        moved: &HashSet<String>,
    ) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut pending = moved.iter().cloned().collect::<Vec<_>>();
        while let Some(block_id) = pending.pop() {
            if block_id == target_parent_id {
                return Err(format!(
                    "target parent {target_parent_id} is inside a moved subtree"
                ));
            }
            if visited.insert(block_id.clone()) {
                pending.extend(
                    self.content_ids_by_parent
                        .get(&block_id)
                        .into_iter()
                        .flatten()
                        .cloned(),
                );
            }
        }
        Ok(())
    }

    fn validate_column_sources(
        &mut self,
        groups: &SourceParentGroups,
        target_parent_id: &str,
    ) -> Result<(), String> {
        for (source_parent, moved_ids) in groups {
            if source_parent == target_parent_id || !self.column_ids.contains(source_parent) {
                continue;
            }
            if self.content_ids(source_parent)?.len() <= moved_ids.len() {
                return Err(format!(
                    "text-selection replacement would leave source column {source_parent} without authoritative content"
                ));
            }
            self.protected_columns.insert(source_parent.clone());
        }
        Ok(())
    }

    fn validate_column_deletion(&self, parent_id: &str) -> Result<(), String> {
        if self.protected_columns.contains(parent_id) && self.content_ids(parent_id)?.len() == 1 {
            return Err(format!(
                "text-selection replacement would leave source column {parent_id} without authoritative content"
            ));
        }
        Ok(())
    }

    fn apply_reorder(&mut self, plan: &ValidatedSelectionReorder) -> Result<(), String> {
        let moved = plan
            .block_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        for (source_parent, _) in &plan.source_parent_groups {
            self.content_ids_by_parent
                .get_mut(source_parent)
                .ok_or_else(|| format!("block {source_parent} has no authoritative content"))?
                .retain(|child_id| !moved.contains(child_id.as_str()));
        }
        let target_content = self
            .content_ids_by_parent
            .get_mut(&plan.target_parent_id)
            .ok_or_else(|| {
                format!(
                    "block {} has no authoritative content",
                    plan.target_parent_id
                )
            })?;
        let insertion_index = plan.insertion_after.as_ref().map_or(Ok(0), |after| {
            target_content
                .iter()
                .position(|child_id| child_id == after)
                .map(|index| index + 1)
                .ok_or_else(|| format!("reorder predecessor {after} disappeared from shadow"))
        })?;
        target_content.splice(
            insertion_index..insertion_index,
            plan.block_ids.iter().cloned(),
        );
        for block_id in &plan.block_ids {
            self.parent_by_block
                .insert(block_id.clone(), plan.target_parent_id.clone());
        }
        Ok(())
    }

    fn apply_deletion(&mut self, plan: &ValidatedSelectionDeletion) -> Result<(), String> {
        let deleted_ids = self.subtree_ids(&plan.block_id);
        self.content_ids_by_parent
            .get_mut(&plan.parent_id)
            .ok_or_else(|| format!("block {} has no authoritative content", plan.parent_id))?
            .retain(|child_id| child_id != &plan.block_id);
        for deleted_id in &deleted_ids {
            self.content_ids_by_parent.remove(deleted_id);
            self.column_ids.remove(deleted_id);
            self.protected_columns.remove(deleted_id);
        }
        self.deleted_block_ids.extend(deleted_ids);
        Ok(())
    }

    fn subtree_ids(&self, root_id: &str) -> HashSet<String> {
        let mut subtree = HashSet::from([root_id.to_string()]);
        let mut pending = vec![root_id.to_string()];
        while let Some(parent_id) = pending.pop() {
            if let Some(children) = self.content_ids_by_parent.get(&parent_id) {
                for child_id in children {
                    if subtree.insert(child_id.clone()) {
                        pending.push(child_id.clone());
                    }
                }
            }
        }
        subtree
    }

    fn parent_id(&self, block_id: &str) -> Result<&str, String> {
        self.parent_by_block
            .get(block_id)
            .map(String::as_str)
            .ok_or_else(|| format!("text-selection block {block_id} has no direct parent"))
    }

    fn content_ids(&self, parent_id: &str) -> Result<&[String], String> {
        self.content_ids_by_parent
            .get(parent_id)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("block {parent_id} has no authoritative content"))
    }
}

fn validate_authoritative_children(parent: &NotionBlockRecord) -> Result<(), String> {
    let unique = parent.content_ids.iter().collect::<HashSet<_>>();
    if unique.len() != parent.content_ids.len() {
        return Err(format!(
            "block {} contains duplicate authoritative direct children",
            parent.id
        ));
    }
    Ok(())
}

fn editable_parent<'a>(
    state: &'a PageMutationState,
    block_id: &str,
) -> Result<&'a NotionBlockRecord, String> {
    let block = editable_record(state, block_id)?;
    if block.space_id != state.space_id {
        return Err(format!("block {block_id} belongs to another space"));
    }
    Ok(block)
}

fn editable_record<'a>(
    state: &'a PageMutationState,
    block_id: &str,
) -> Result<&'a NotionBlockRecord, String> {
    let block = state.block(block_id)?;
    if !block.alive {
        return Err(format!("block {block_id} is not alive"));
    }
    if block.version == 0 {
        return Err(format!("block {block_id} has an invalid zero version"));
    }
    Ok(block)
}
