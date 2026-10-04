use super::UserContext;

impl UserContext {
    pub(in crate::live::board) fn set_page_bookmarked(
        &mut self,
        space_id: &str,
        block_id: &str,
        is_bookmarked: bool,
    ) -> Result<(), String> {
        let space_view_id = self
            .space_view_ids_by_space_id
            .get(space_id)
            .ok_or_else(|| format!("missing space_view id for space {space_id}"))?
            .clone();
        let bookmarked_pages = self
            .bookmarked_pages_by_space_view_id
            .entry(space_view_id)
            .or_default();
        let sidebar = self
            .sidebar_by_space_id
            .get_mut(space_id)
            .ok_or_else(|| format!("missing sidebar context for space {space_id}"))?;
        if is_bookmarked {
            bookmarked_pages.insert(block_id.to_string());
            sidebar
                .favorite_page_ids
                .retain(|page_id| page_id != block_id);
            sidebar.favorite_page_ids.insert(0, block_id.to_string());
        } else {
            bookmarked_pages.remove(block_id);
            sidebar
                .favorite_page_ids
                .retain(|page_id| page_id != block_id);
        }
        Ok(())
    }
}
