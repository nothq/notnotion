use serde::{Deserialize, Serialize};

use super::{CardPageBlock, CardPageBlockContent};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageOpaqueUnavailableBlock {
    #[serde(skip)]
    _private: (),
}

impl CardPageBlock {
    pub(crate) fn opaque_unavailable(
        block_id: impl Into<String>,
        parent_block_id: impl Into<String>,
        depth: usize,
    ) -> Self {
        Self::new(
            block_id,
            parent_block_id,
            depth,
            CardPageBlockContent::OpaqueUnavailable {
                opaque_unavailable: CardPageOpaqueUnavailableBlock { _private: () },
            },
        )
    }

    pub const fn opaque_unavailable_content(&self) -> Option<&CardPageOpaqueUnavailableBlock> {
        match &self.content {
            CardPageBlockContent::OpaqueUnavailable { opaque_unavailable } => {
                Some(opaque_unavailable)
            }
            _ => None,
        }
    }

    pub const fn is_opaque_unavailable(&self) -> bool {
        matches!(
            &self.content,
            CardPageBlockContent::OpaqueUnavailable { .. }
        )
    }
}
