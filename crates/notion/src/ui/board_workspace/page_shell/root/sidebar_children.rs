use crate::ui::surface::{
    SidebarResourceContext, SidebarResourceEffect, SidebarResourceEvent,
    SidebarWorkspacePreparation, SidebarWorkspaceReadiness,
};
use crate::ui::{Context, NotionStartup, SurfaceState};

impl SurfaceState {
    pub(crate) fn ensure_notion_page_shell_resources(&mut self, cx: &mut Context<Self>) {
        let context = SidebarResourceContext::new(
            self.notion_startup.board_url(),
            self.notion_startup.workspace_api(),
            self.notion_chrome.notion_navigation_request_id,
        );
        let readiness = SidebarWorkspaceReadiness {
            surface_active: self.surface_active,
            startup_ready: matches!(self.notion_startup, NotionStartup::Ready(_)),
            has_page_shell: self.board.page_shell.is_some(),
        };
        match self
            .notion_sidebar
            .prepare_workspace_resource(readiness, &context)
        {
            SidebarWorkspacePreparation::None => {}
            SidebarWorkspacePreparation::PageHydrationOnly => {
                self.start_notion_page_hydration(cx);
            }
            SidebarWorkspacePreparation::Start {
                job,
                board_url,
                workspace_api,
            } => {
                job.spawn(cx, handle_sidebar_resource_event);
                if let Some(start) = self
                    .notion_sidebar
                    .prepare_calendar_resource(self.board.page_shell.as_ref(), &context)
                {
                    start.spawn(cx, handle_sidebar_resource_event);
                }
                self.load_notion_page_presence(
                    board_url,
                    context.navigation_request_id,
                    workspace_api,
                    cx,
                );
                self.start_notion_page_hydration(cx);
            }
        }
        if let Some(start) = self
            .notion_sidebar
            .prepare_chats_resource(self.board.page_shell.as_ref(), &context)
        {
            start.spawn(cx, handle_sidebar_resource_event);
        }
        if let Some(start) = self
            .notion_sidebar
            .prepare_inbox_resource(self.board.page_shell.as_ref(), &context)
        {
            start.spawn(cx, handle_sidebar_resource_event);
        }
    }
}

pub(in crate::ui::board_workspace::page_shell) fn handle_sidebar_resource_event(
    surface: &mut SurfaceState,
    event: SidebarResourceEvent,
    cx: &mut Context<SurfaceState>,
) {
    let context = SidebarResourceContext::new(
        surface.notion_startup.board_url(),
        surface.notion_startup.workspace_api(),
        surface.notion_chrome.notion_navigation_request_id,
    );
    let effect = surface
        .notion_sidebar
        .complete_resource(event, &mut surface.board, &context);
    match effect {
        SidebarResourceEffect::None => {}
        SidebarResourceEffect::Notify => cx.notify(),
        SidebarResourceEffect::SpawnAndNotify(jobs) => {
            for job in jobs {
                job.spawn(cx, handle_sidebar_resource_event);
            }
            cx.notify();
        }
        SidebarResourceEffect::LoadInbox => {
            if let Some(start) = surface
                .notion_sidebar
                .prepare_inbox_resource(surface.board.page_shell.as_ref(), &context)
            {
                start.spawn(cx, handle_sidebar_resource_event);
            }
        }
        SidebarResourceEffect::Failure(failure) => {
            let (operation, error, recovery) = (*failure).into_parts();
            if surface.handle_notion_workspace_failure(operation, error, cx) {
                return;
            }
            if surface
                .notion_sidebar
                .recover_resource_failure(recovery, surface.board.page_shell.as_ref())
            {
                cx.notify();
            }
        }
    }
}
