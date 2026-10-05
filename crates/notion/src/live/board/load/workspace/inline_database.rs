use std::collections::{BTreeMap, BTreeSet};

use crate::live::board::{
    block_value, page_chunk::hydrate_collection_views, record_map_table, required_string,
    required_string_array, BoardTarget, Value,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

pub(super) fn hydrate_inline_database_views(
    session: &NotionDesktopSession,
    target: &BoardTarget,
    bootstrap: &mut Value,
) -> Result<(), NotionLiveError> {
    let root = block_value(
        record_map_table(bootstrap, "block")?,
        &target.collection_view_block_id,
    )?;
    let space_id = required_string(root, "space_id")?.to_string();
    let view_ids = required_string_array(root, "view_ids")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    hydrate_collection_views(session, &BTreeMap::from([(space_id, view_ids)]), bootstrap)
}
