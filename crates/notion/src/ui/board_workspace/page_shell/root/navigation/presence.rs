use crate::ui::{Arc, Context, NotionWorkspaceApi, SurfaceState};

impl SurfaceState {
    pub(crate) fn load_notion_page_presence(
        &mut self,
        board_url: String,
        request_id: u64,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<Self>,
    ) {
        let requested_board_url = board_url.clone();
        self.spawn_background_task(
            board_url,
            cx,
            move |_| workspace_api.load_page_presence(),
            move |this, result, cx| {
                if this.notion_chrome.notion_navigation_request_id != request_id
                    || this.notion_startup.board_url().as_deref()
                        != Some(requested_board_url.as_str())
                {
                    return;
                }
                match result {
                    Ok(Some(presence)) => {
                        this.replace_notion_page_presence(presence);
                        cx.notify();
                    }
                    Ok(None) => {}
                    Err(error) => {
                        this.handle_notion_workspace_failure(
                            "page presence loading failed",
                            error,
                            cx,
                        );
                    }
                }
            },
        );
    }
}
