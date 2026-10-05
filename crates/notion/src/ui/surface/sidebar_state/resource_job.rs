use std::sync::Arc;

use gpui::Context;

use crate::model::{
    LoadSidebarCalendarRequest, LoadSidebarCalendarResult, LoadSidebarChatsRequest,
    LoadSidebarChatsResult, LoadSidebarChildrenRequest, LoadSidebarChildrenResult,
    LoadSidebarInboxRequest, LoadSidebarInboxResult, MutateSidebarInboxRequest,
    NotionSidebarInboxFilter, NotionWorkspaceOperationFailure, NotionWorkspaceResult,
};
use crate::ui::{NotionSidebarNodeKey, NotionWorkspaceApi, SurfaceState};

#[derive(Clone)]
pub(crate) struct SidebarResourceContext {
    pub(crate) board_url: Option<String>,
    pub(crate) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(crate) navigation_request_id: u64,
}

impl SidebarResourceContext {
    pub(crate) fn new(
        board_url: Option<String>,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
        navigation_request_id: u64,
    ) -> Self {
        Self {
            board_url,
            workspace_api,
            navigation_request_id,
        }
    }
}

pub(crate) struct SidebarResourceJob(SidebarResourceJobKind);

pub(crate) struct SidebarResourceJobStart {
    pub(crate) job: SidebarResourceJob,
    pub(crate) notify: bool,
}

pub(crate) struct SidebarResourceEvent(SidebarResourceCompletion);

pub(crate) enum SidebarResourceEffect {
    None,
    Notify,
    SpawnAndNotify(Vec<SidebarResourceJob>),
    LoadInbox,
    Failure(Box<SidebarResourceFailure>),
}

pub(crate) struct SidebarResourceFailure {
    pub(super) operation: &'static str,
    pub(super) error: NotionWorkspaceOperationFailure,
    pub(super) recovery: SidebarResourceFailureRecovery,
}

pub(crate) enum SidebarResourceFailureRecovery {
    Workspace,
    Calendar,
    Chats,
    Inbox,
    InboxPagination,
    InboxMutation,
    Children,
}

enum SidebarResourceJobKind {
    Workspace(SidebarWorkspaceJob),
    Calendar(SidebarCalendarJob),
    Chats(SidebarChatsJob),
    Inbox(SidebarInboxJob),
    InboxMutation(SidebarInboxMutationJob),
    Children(SidebarChildrenJob),
}

pub(super) enum SidebarResourceCompletion {
    Workspace(SidebarWorkspaceCompletion),
    Calendar(NotionWorkspaceResult<LoadSidebarCalendarResult>),
    Chats(NotionWorkspaceResult<LoadSidebarChatsResult>),
    Inbox(SidebarInboxCompletion),
    InboxMutation(NotionWorkspaceResult<()>),
    Children(SidebarChildrenCompletion),
}

struct SidebarWorkspaceJob {
    board_url: String,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
}

pub(super) struct SidebarWorkspaceCompletion {
    pub(super) board_url: String,
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) result: NotionWorkspaceResult<()>,
}

struct SidebarCalendarJob {
    request: LoadSidebarCalendarRequest,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
}

struct SidebarChatsJob {
    request: LoadSidebarChatsRequest,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
}

struct SidebarInboxJob {
    request: LoadSidebarInboxRequest,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    kind: SidebarInboxLoadKind,
}

#[derive(Clone, Copy)]
pub(super) enum SidebarInboxLoadKind {
    Initial,
    More,
}

pub(super) struct SidebarInboxCompletion {
    pub(super) filter: NotionSidebarInboxFilter,
    pub(super) size: u32,
    pub(super) kind: SidebarInboxLoadKind,
    pub(super) result: NotionWorkspaceResult<LoadSidebarInboxResult>,
}

struct SidebarInboxMutationJob {
    request: MutateSidebarInboxRequest,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
}

struct SidebarChildrenJob {
    request: LoadSidebarChildrenRequest,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    navigation_request_id: u64,
    node_key: NotionSidebarNodeKey,
}

pub(super) struct SidebarChildrenCompletion {
    pub(super) board_url: String,
    pub(super) navigation_request_id: u64,
    pub(super) node_key: NotionSidebarNodeKey,
    pub(super) result: NotionWorkspaceResult<LoadSidebarChildrenResult>,
}

impl SidebarResourceJob {
    pub(super) fn workspace(board_url: String, workspace_api: Arc<dyn NotionWorkspaceApi>) -> Self {
        Self(SidebarResourceJobKind::Workspace(SidebarWorkspaceJob {
            board_url,
            workspace_api,
        }))
    }

    pub(super) fn calendar(
        request: LoadSidebarCalendarRequest,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
    ) -> Self {
        Self(SidebarResourceJobKind::Calendar(SidebarCalendarJob {
            request,
            workspace_api,
        }))
    }

    pub(super) fn chats(
        request: LoadSidebarChatsRequest,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
    ) -> Self {
        Self(SidebarResourceJobKind::Chats(SidebarChatsJob {
            request,
            workspace_api,
        }))
    }

    pub(super) fn inbox(
        request: LoadSidebarInboxRequest,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        kind: SidebarInboxLoadKind,
    ) -> Self {
        Self(SidebarResourceJobKind::Inbox(SidebarInboxJob {
            request,
            workspace_api,
            kind,
        }))
    }

    pub(super) fn inbox_mutation(
        request: MutateSidebarInboxRequest,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
    ) -> Self {
        Self(SidebarResourceJobKind::InboxMutation(
            SidebarInboxMutationJob {
                request,
                workspace_api,
            },
        ))
    }

    pub(super) fn children(
        request: LoadSidebarChildrenRequest,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        navigation_request_id: u64,
        node_key: NotionSidebarNodeKey,
    ) -> Self {
        Self(SidebarResourceJobKind::Children(SidebarChildrenJob {
            request,
            workspace_api,
            navigation_request_id,
            node_key,
        }))
    }

    pub(crate) fn spawn<Apply>(self, cx: &mut Context<SurfaceState>, apply: Apply)
    where
        Apply:
            FnOnce(&mut SurfaceState, SidebarResourceEvent, &mut Context<SurfaceState>) + 'static,
    {
        gpui_components::spawn_background_task_for_entity(self, cx, SidebarResourceJob::run, apply);
    }

    fn run(self) -> SidebarResourceEvent {
        let completion = match self.0 {
            SidebarResourceJobKind::Workspace(job) => job.run(),
            SidebarResourceJobKind::Calendar(job) => job.run(),
            SidebarResourceJobKind::Chats(job) => job.run(),
            SidebarResourceJobKind::Inbox(job) => job.run(),
            SidebarResourceJobKind::InboxMutation(job) => job.run(),
            SidebarResourceJobKind::Children(job) => job.run(),
        };
        SidebarResourceEvent(completion)
    }
}

impl SidebarResourceJobStart {
    pub(crate) fn spawn<Apply>(self, cx: &mut Context<SurfaceState>, apply: Apply)
    where
        Apply:
            FnOnce(&mut SurfaceState, SidebarResourceEvent, &mut Context<SurfaceState>) + 'static,
    {
        if self.notify {
            cx.notify();
        }
        self.job.spawn(cx, apply);
    }
}

impl SidebarResourceEvent {
    pub(super) fn into_completion(self) -> SidebarResourceCompletion {
        self.0
    }
}

impl SidebarResourceFailure {
    pub(crate) fn into_parts(
        self,
    ) -> (
        &'static str,
        NotionWorkspaceOperationFailure,
        SidebarResourceFailureRecovery,
    ) {
        (self.operation, self.error, self.recovery)
    }
}

impl SidebarWorkspaceJob {
    fn run(self) -> SidebarResourceCompletion {
        let result = self.workspace_api.hydrate_sidebar();
        SidebarResourceCompletion::Workspace(SidebarWorkspaceCompletion {
            board_url: self.board_url,
            workspace_api: self.workspace_api,
            result,
        })
    }
}

impl SidebarCalendarJob {
    fn run(self) -> SidebarResourceCompletion {
        SidebarResourceCompletion::Calendar(self.workspace_api.load_sidebar_calendar(self.request))
    }
}

impl SidebarChatsJob {
    fn run(self) -> SidebarResourceCompletion {
        SidebarResourceCompletion::Chats(self.workspace_api.load_sidebar_chats(self.request))
    }
}

impl SidebarInboxJob {
    fn run(self) -> SidebarResourceCompletion {
        let filter = self.request.filter;
        let size = self.request.size;
        let result = self.workspace_api.load_sidebar_inbox(self.request);
        SidebarResourceCompletion::Inbox(SidebarInboxCompletion {
            filter,
            size,
            kind: self.kind,
            result,
        })
    }
}

impl SidebarInboxMutationJob {
    fn run(self) -> SidebarResourceCompletion {
        SidebarResourceCompletion::InboxMutation(
            self.workspace_api.mutate_sidebar_inbox(self.request),
        )
    }
}

impl SidebarChildrenJob {
    fn run(self) -> SidebarResourceCompletion {
        let board_url = self.request.current_board_url.clone();
        let result = self.workspace_api.load_sidebar_children(self.request);
        SidebarResourceCompletion::Children(SidebarChildrenCompletion {
            board_url,
            navigation_request_id: self.navigation_request_id,
            node_key: self.node_key,
            result,
        })
    }
}
