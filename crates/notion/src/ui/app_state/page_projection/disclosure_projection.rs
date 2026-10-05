use std::collections::HashSet;
use std::sync::Arc;

use super::LoadedCardPage;

impl LoadedCardPage {
    pub(crate) fn rebuild_visible_projection(
        &mut self,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> bool {
        let Some(replacement) = self.data.rebuilt_visible_projection(expanded_toggle_ids) else {
            return false;
        };
        self.data = Arc::new(replacement);
        if self.data.has_column_structure() {
            return true;
        }
        self.list_state.remeasure();
        true
    }
}
