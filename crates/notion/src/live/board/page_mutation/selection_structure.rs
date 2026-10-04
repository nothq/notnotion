use std::collections::{HashMap, HashSet};

use super::super::page_state::PageMutationState;
use crate::model::{PageBlockPlacement, PageBlockStructuralMutation};

mod shadow;

use shadow::SelectionStructureShadow;

pub(in crate::live::board::page_mutation) type SourceParentGroup = (String, Vec<String>);
type SourceParentGroups = Vec<SourceParentGroup>;

pub(in crate::live::board::page_mutation) struct ValidatedSelectionStructure {
    steps: Vec<ValidatedSelectionStructureStep>,
    final_parent_ids: HashMap<String, String>,
}

pub(in crate::live::board::page_mutation) enum ValidatedSelectionStructureStep {
    Reorder(ValidatedSelectionReorder),
    Delete(ValidatedSelectionDeletion),
}

pub(in crate::live::board::page_mutation) struct ValidatedSelectionReorder {
    source_parent_groups: SourceParentGroups,
    source_parent_ids: Vec<String>,
    target_parent_id: String,
    block_ids: Vec<String>,
    insertion_after: Option<String>,
}

pub(in crate::live::board::page_mutation) struct ValidatedSelectionDeletion {
    block_id: String,
    parent_id: String,
}

impl ValidatedSelectionStructure {
    pub(in crate::live::board::page_mutation) fn new(
        state: &PageMutationState,
        mutations: &[PageBlockStructuralMutation],
    ) -> Result<Self, String> {
        let mut shadow = SelectionStructureShadow::new(state)?;
        let mut steps = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            let step = match mutation {
                PageBlockStructuralMutation::Reorder(request) => {
                    ValidatedSelectionStructureStep::Reorder(shadow.plan_reorder(state, request)?)
                }
                PageBlockStructuralMutation::Delete(request) => {
                    ValidatedSelectionStructureStep::Delete(
                        shadow.plan_deletion(state, &request.block_id)?,
                    )
                }
            };
            steps.push(step);
        }
        Ok(Self {
            steps,
            final_parent_ids: shadow.into_final_parent_ids(),
        })
    }

    pub(in crate::live::board::page_mutation) fn steps(
        &self,
    ) -> &[ValidatedSelectionStructureStep] {
        &self.steps
    }

    pub(in crate::live::board::page_mutation) fn final_parent_ids(
        &self,
    ) -> &HashMap<String, String> {
        &self.final_parent_ids
    }
}

impl ValidatedSelectionReorder {
    pub(in crate::live::board::page_mutation) fn source_parent_groups(
        &self,
    ) -> &[SourceParentGroup] {
        &self.source_parent_groups
    }

    pub(in crate::live::board::page_mutation) fn block_source_parents(
        &self,
    ) -> impl Iterator<Item = (&str, &str)> {
        self.block_ids
            .iter()
            .zip(&self.source_parent_ids)
            .map(|(block_id, parent_id)| (block_id.as_str(), parent_id.as_str()))
    }

    pub(in crate::live::board::page_mutation) fn target_parent_id(&self) -> &str {
        &self.target_parent_id
    }

    pub(in crate::live::board::page_mutation) fn block_ids(&self) -> &[String] {
        &self.block_ids
    }

    pub(in crate::live::board::page_mutation) fn insertion_after(&self) -> Option<&str> {
        self.insertion_after.as_deref()
    }
}

impl ValidatedSelectionDeletion {
    pub(in crate::live::board::page_mutation) fn block_id(&self) -> &str {
        &self.block_id
    }

    pub(in crate::live::board::page_mutation) fn parent_id(&self) -> &str {
        &self.parent_id
    }
}

pub(in crate::live::board::page_mutation) fn validated_insertion_after(
    content_ids: &[String],
    moved: &HashSet<String>,
    placement: &PageBlockPlacement,
) -> Result<Option<String>, String> {
    match placement {
        PageBlockPlacement::Append => Ok(content_ids
            .iter()
            .rev()
            .find(|child_id| !moved.contains(*child_id))
            .cloned()),
        PageBlockPlacement::Before(anchor) => {
            let anchor_index = validated_anchor_index(content_ids, moved, anchor)?;
            Ok(content_ids[..anchor_index]
                .iter()
                .rev()
                .find(|child_id| !moved.contains(*child_id))
                .cloned())
        }
        PageBlockPlacement::After(anchor) => {
            validated_anchor_index(content_ids, moved, anchor)?;
            Ok(Some(anchor.clone()))
        }
    }
}

pub(in crate::live::board::page_mutation) fn validate_source_order(
    content_ids: &[String],
    block_ids: &[String],
    moved: &HashSet<String>,
) -> Result<(), String> {
    if !content_ids
        .iter()
        .filter(|id| moved.contains(*id))
        .eq(block_ids)
    {
        return Err("subtree move ids must preserve their current sibling order".to_string());
    }
    Ok(())
}

fn validated_anchor_index(
    content_ids: &[String],
    moved: &HashSet<String>,
    anchor: &str,
) -> Result<usize, String> {
    if moved.contains(anchor) {
        return Err("a subtree reorder anchor cannot be one of the moved blocks".to_string());
    }
    content_ids
        .iter()
        .position(|child_id| child_id == anchor)
        .ok_or_else(|| format!("block {anchor} is not a child of the target parent"))
}
