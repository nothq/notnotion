use std::time::Duration;

use crate::ui::{Context, SurfaceState};

impl SurfaceState {
    pub(crate) fn update_notion_workspace_prefetch(
        &mut self,
        board_url: String,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if !hovered {
            if self.notion_sidebar.prefetch_target.as_deref() == Some(board_url.as_str()) {
                self.notion_sidebar.prefetch_target = None;
            }
            return;
        }
        if self.notion_startup.board_url().as_deref() == Some(board_url.as_str())
            || self.notion_sidebar.prefetch_target.as_deref() == Some(board_url.as_str())
        {
            return;
        }
        self.notion_sidebar.prefetch_target = Some(board_url.clone());
        self.spawn_timer_task(
            board_url,
            Duration::from_millis(120),
            cx,
            |this, board_url, cx| {
                if this.notion_sidebar.prefetch_target.as_deref() != Some(board_url.as_str()) {
                    return;
                }
                this.notion_sidebar.prefetch_target = None;
                let Some(workspace_api) = this.notion_startup.workspace_api() else {
                    return;
                };
                this.spawn_background_task(
                    board_url,
                    cx,
                    move |board_url| workspace_api.prefetch_notion_workspace(&board_url),
                    |this, result, cx| {
                        if let Err(error) = result {
                            this.handle_notion_workspace_failure(
                                "workspace prefetch failed",
                                error,
                                cx,
                            );
                        }
                    },
                );
            },
        );
    }
}
