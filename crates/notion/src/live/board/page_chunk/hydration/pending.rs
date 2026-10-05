use std::collections::{BTreeMap, HashMap};

use super::super::block_record::{AuthoritativePageBlockUnavailable, PageBlockRequirement};

#[derive(Clone)]
pub(super) struct PendingPageBlock {
    pub(super) block_id: String,
    pub(super) space_id: String,
    provenance: PageBlockProvenance,
}

#[derive(Clone)]
enum PageBlockProvenance {
    Root,
    ContentChild { parent_block_id: String },
    ReferenceTarget,
}

impl PendingPageBlock {
    pub(super) fn root(block_id: &str, space_id: &str) -> Self {
        Self {
            block_id: block_id.to_string(),
            space_id: space_id.to_string(),
            provenance: PageBlockProvenance::Root,
        }
    }

    pub(super) fn content_child(block_id: &str, space_id: &str, parent_block_id: &str) -> Self {
        Self {
            block_id: block_id.to_string(),
            space_id: space_id.to_string(),
            provenance: PageBlockProvenance::ContentChild {
                parent_block_id: parent_block_id.to_string(),
            },
        }
    }

    pub(super) fn reference_target(block_id: &str, space_id: &str) -> Self {
        Self {
            block_id: block_id.to_string(),
            space_id: space_id.to_string(),
            provenance: PageBlockProvenance::ReferenceTarget,
        }
    }

    pub(super) const fn requirement(&self) -> PageBlockRequirement {
        match &self.provenance {
            PageBlockProvenance::Root => PageBlockRequirement::Root,
            PageBlockProvenance::ContentChild { .. } => PageBlockRequirement::ContentChild,
            PageBlockProvenance::ReferenceTarget => PageBlockRequirement::ReferenceTarget,
        }
    }

    pub(super) const fn follows_content(&self) -> bool {
        !matches!(&self.provenance, PageBlockProvenance::ReferenceTarget)
    }

    pub(super) fn content_parent_id(&self) -> Option<&str> {
        match &self.provenance {
            PageBlockProvenance::ContentChild { parent_block_id } => Some(parent_block_id),
            PageBlockProvenance::Root | PageBlockProvenance::ReferenceTarget => None,
        }
    }

    pub(super) const fn is_root(&self) -> bool {
        matches!(&self.provenance, PageBlockProvenance::Root)
    }

    pub(super) const fn is_reference_target(&self) -> bool {
        matches!(&self.provenance, PageBlockProvenance::ReferenceTarget)
    }

    pub(super) fn merge_provenance(&mut self, incoming: Self) -> Result<(), String> {
        if self.space_id != incoming.space_id {
            return Err(format!(
                "Notion page references block {} in both spaces {} and {}",
                self.block_id, self.space_id, incoming.space_id
            ));
        }
        match (&self.provenance, incoming.provenance) {
            (PageBlockProvenance::ReferenceTarget, stronger) => {
                self.provenance = stronger;
                Ok(())
            }
            (_, PageBlockProvenance::ReferenceTarget) => Ok(()),
            (PageBlockProvenance::Root, PageBlockProvenance::Root) => Ok(()),
            (
                PageBlockProvenance::ContentChild { parent_block_id },
                PageBlockProvenance::ContentChild {
                    parent_block_id: incoming_parent_id,
                },
            ) if parent_block_id == &incoming_parent_id => Ok(()),
            (PageBlockProvenance::Root, PageBlockProvenance::ContentChild { parent_block_id }) => {
                Err(format!(
                    "Notion page root {} is also referenced as content by {parent_block_id}",
                    self.block_id
                ))
            }
            (
                PageBlockProvenance::ContentChild { parent_block_id },
                PageBlockProvenance::Root,
            ) => Err(format!(
                "Notion page root {} is also referenced as content by {parent_block_id}",
                self.block_id
            )),
            (
                PageBlockProvenance::ContentChild { parent_block_id },
                PageBlockProvenance::ContentChild {
                    parent_block_id: incoming_parent_id,
                },
            ) => Err(format!(
                "Notion page references block {} as content of both {parent_block_id} and {incoming_parent_id}",
                self.block_id
            )),
        }
    }
}

pub(super) type MissingPageBlocksBySpace = BTreeMap<String, BTreeMap<String, PendingPageBlock>>;
pub(super) type UnavailableContentEdgesBySpace =
    BTreeMap<String, BTreeMap<String, BTreeMap<String, AuthoritativePageBlockUnavailable>>>;

#[derive(Default)]
pub(super) struct MissingPageBlocks {
    pub(super) by_space: MissingPageBlocksBySpace,
    pub(super) unavailable_content_edges_by_space: UnavailableContentEdgesBySpace,
    space_by_id: HashMap<String, String>,
}

impl MissingPageBlocks {
    pub(super) fn is_empty(&self) -> bool {
        self.by_space.is_empty() && self.unavailable_content_edges_by_space.is_empty()
    }

    pub(super) fn insert(&mut self, pending: PendingPageBlock) -> Result<(), String> {
        self.record_space(&pending)?;
        match self
            .by_space
            .entry(pending.space_id.clone())
            .or_default()
            .entry(pending.block_id.clone())
        {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(pending);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                entry.get_mut().merge_provenance(pending)?;
            }
        }
        Ok(())
    }

    pub(super) fn insert_unavailable_content(
        &mut self,
        pending: PendingPageBlock,
        unavailable: AuthoritativePageBlockUnavailable,
    ) -> Result<(), String> {
        self.record_space(&pending)?;
        let parent_block_id = pending.content_parent_id().ok_or_else(|| {
            format!(
                "required Notion page block {} has no content-edge provenance",
                pending.block_id
            )
        })?;
        self.unavailable_content_edges_by_space
            .entry(pending.space_id.clone())
            .or_default()
            .entry(parent_block_id.to_string())
            .or_default()
            .insert(pending.block_id, unavailable);
        Ok(())
    }

    fn record_space(&mut self, pending: &PendingPageBlock) -> Result<(), String> {
        if let Some(previous_space_id) = self.space_by_id.get(&pending.block_id) {
            if previous_space_id != &pending.space_id {
                return Err(format!(
                    "Notion page references block {} in both spaces {previous_space_id} and {}",
                    pending.block_id, pending.space_id
                ));
            }
        } else {
            self.space_by_id
                .insert(pending.block_id.clone(), pending.space_id.clone());
        }
        Ok(())
    }
}
