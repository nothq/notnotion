use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::live::board) struct ProvenOpaqueUnavailableBlock {
    block_id: String,
    parent_block_id: String,
    space_id: String,
}

pub(in crate::live::board) type ProvenOpaqueUnavailableBlocks =
    BTreeMap<String, ProvenOpaqueUnavailableBlock>;

impl ProvenOpaqueUnavailableBlock {
    pub(super) fn from_authoritative_content_edge(
        block_id: &str,
        parent_block_id: &str,
        space_id: &str,
    ) -> Self {
        Self {
            block_id: block_id.to_string(),
            parent_block_id: parent_block_id.to_string(),
            space_id: space_id.to_string(),
        }
    }

    pub(in crate::live::board) fn block_id(&self) -> &str {
        &self.block_id
    }

    pub(in crate::live::board) fn validates_content_edge(
        &self,
        block_id: &str,
        parent_block_id: &str,
        space_id: &str,
    ) -> bool {
        self.block_id == block_id
            && self.parent_block_id == parent_block_id
            && self.space_id == space_id
    }
}

pub(super) fn insert_proven_opaque_block(
    proofs: &mut ProvenOpaqueUnavailableBlocks,
    proof: ProvenOpaqueUnavailableBlock,
) -> Result<(), String> {
    match proofs.entry(proof.block_id.clone()) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(proof);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &proof => Ok(()),
        std::collections::btree_map::Entry::Occupied(entry) => Err(format!(
            "opaque Notion block {} is structural content of both {} in space {} and {} in space {}",
            proof.block_id,
            entry.get().parent_block_id,
            entry.get().space_id,
            proof.parent_block_id,
            proof.space_id
        )),
    }
}
