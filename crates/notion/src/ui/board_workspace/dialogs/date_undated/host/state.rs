use crate::ui::SurfaceState;

impl SurfaceState {
    pub(crate) fn reset_date_query_state_for_route(&mut self) {
        self.date_view.reset_for_route(&self.board.items);
        self.notion_chrome.date_undated_dialog = None;
        if self.notion_startup.workspace_api().is_some() {
            self.board.date_undated_count = None;
        }
    }
}
