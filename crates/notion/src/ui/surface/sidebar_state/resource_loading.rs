use crate::model::{
    LoadSidebarCalendarRequest, LoadSidebarChatsRequest, LoadSidebarInboxRequest,
    LoadSidebarInboxResult, MutateSidebarInboxAction, MutateSidebarInboxRequest,
    NotionSidebarInboxFilter, NotionWorkspaceOperationFailure, NotionWorkspaceResult,
    PageShellSnapshot,
};
use crate::ui::{BoardSnapshot, NotionSidebarTab};

use super::resource_job::{
    SidebarInboxCompletion, SidebarInboxLoadKind, SidebarResourceCompletion,
    SidebarResourceContext, SidebarResourceEffect, SidebarResourceEvent, SidebarResourceFailure,
    SidebarResourceFailureRecovery, SidebarResourceJob, SidebarResourceJobStart,
};
use super::{
    NotionSidebarCalendarState, NotionSidebarChatsState, NotionSidebarInboxState,
    NotionSidebarState,
};

impl NotionSidebarState {
    pub(crate) fn prepare_calendar_resource(
        &mut self,
        page_shell: Option<&PageShellSnapshot>,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJobStart> {
        if !matches!(&self.calendar, NotionSidebarCalendarState::Idle) || page_shell.is_none() {
            return None;
        }
        let (Some(workspace_api), Some(current_board_url)) =
            (&context.workspace_api, &context.board_url)
        else {
            return None;
        };
        self.calendar = NotionSidebarCalendarState::Loading;
        self.rebuild_rows(page_shell);
        Some(SidebarResourceJobStart {
            job: SidebarResourceJob::calendar(
                LoadSidebarCalendarRequest {
                    current_board_url: current_board_url.clone(),
                },
                workspace_api.clone(),
            ),
            notify: true,
        })
    }

    pub(crate) fn prepare_chats_resource(
        &mut self,
        page_shell: Option<&PageShellSnapshot>,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJobStart> {
        if self.active_tab != NotionSidebarTab::Chat
            || !matches!(&self.chats, NotionSidebarChatsState::Idle)
            || page_shell.is_none()
        {
            return None;
        }
        let (Some(workspace_api), Some(current_board_url)) =
            (&context.workspace_api, &context.board_url)
        else {
            return None;
        };
        self.chats = NotionSidebarChatsState::Loading;
        self.rebuild_rows(page_shell);
        Some(SidebarResourceJobStart {
            job: SidebarResourceJob::chats(
                LoadSidebarChatsRequest {
                    current_board_url: current_board_url.clone(),
                    cursor: None,
                },
                workspace_api.clone(),
            ),
            notify: true,
        })
    }

    pub(crate) fn prepare_inbox_resource(
        &mut self,
        page_shell: Option<&PageShellSnapshot>,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJobStart> {
        if self.active_tab != NotionSidebarTab::Inbox
            || !matches!(&self.inbox, NotionSidebarInboxState::Idle)
            || page_shell.is_none()
        {
            return None;
        }
        let job = self.inbox_job(
            context,
            self.inbox_request_size,
            SidebarInboxLoadKind::Initial,
        )?;
        self.inbox = NotionSidebarInboxState::Loading;
        Some(SidebarResourceJobStart { job, notify: true })
    }

    pub(crate) fn prepare_inbox_pagination(
        &mut self,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJobStart> {
        if !matches!(
            &self.inbox,
            NotionSidebarInboxState::Loaded { has_more: true, .. }
        ) || self.inbox_loading_more
            || self.inbox_action_in_flight
            || self.inbox_scroll_remaining() >= 200.0
        {
            return None;
        }
        let size = self.inbox_request_size.saturating_add(20);
        let job = self.inbox_job(context, size, SidebarInboxLoadKind::More)?;
        self.inbox_loading_more = true;
        self.inbox_request_size = size;
        Some(SidebarResourceJobStart { job, notify: false })
    }

    pub(crate) fn prepare_inbox_mutation(
        &mut self,
        action: MutateSidebarInboxAction,
        context: &SidebarResourceContext,
    ) -> Option<SidebarResourceJobStart> {
        if self.inbox_action_in_flight {
            return None;
        }
        let (Some(workspace_api), Some(current_board_url)) =
            (&context.workspace_api, &context.board_url)
        else {
            return None;
        };
        let request = MutateSidebarInboxRequest {
            current_board_url: current_board_url.clone(),
            action,
        };
        self.inbox_action_in_flight = true;
        self.inbox_loading_more = false;
        self.dismiss_inbox_menus_without_notify();
        self.apply_optimistic_inbox_action(&request.action);
        Some(SidebarResourceJobStart {
            job: SidebarResourceJob::inbox_mutation(request, workspace_api.clone()),
            notify: true,
        })
    }

    pub(crate) fn complete_resource(
        &mut self,
        event: SidebarResourceEvent,
        board: &mut BoardSnapshot,
        context: &SidebarResourceContext,
    ) -> SidebarResourceEffect {
        match event.into_completion() {
            SidebarResourceCompletion::Workspace(completion) => {
                self.complete_workspace_resource(completion, board, context)
            }
            SidebarResourceCompletion::Calendar(result) => {
                self.complete_calendar_resource(result, board.page_shell.as_ref())
            }
            SidebarResourceCompletion::Chats(result) => {
                self.complete_chats_resource(result, board.page_shell.as_ref())
            }
            SidebarResourceCompletion::Inbox(completion) => {
                self.complete_inbox_resource(completion)
            }
            SidebarResourceCompletion::InboxMutation(result) => {
                self.complete_inbox_mutation(result)
            }
            SidebarResourceCompletion::Children(completion) => {
                self.complete_children_resource(completion, board, context)
            }
        }
    }

    pub(crate) fn recover_resource_failure(
        &mut self,
        recovery: SidebarResourceFailureRecovery,
        page_shell: Option<&PageShellSnapshot>,
    ) -> bool {
        match recovery {
            SidebarResourceFailureRecovery::Workspace => {
                self.tree = super::NotionSidebarTreeState::Failed;
                false
            }
            SidebarResourceFailureRecovery::Calendar => {
                self.calendar = NotionSidebarCalendarState::Failed;
                self.rebuild_rows(page_shell);
                true
            }
            SidebarResourceFailureRecovery::Chats => {
                self.chats = NotionSidebarChatsState::Failed;
                self.rebuild_rows(page_shell);
                true
            }
            SidebarResourceFailureRecovery::Inbox => {
                self.inbox = NotionSidebarInboxState::Failed;
                self.inbox_loading_more = false;
                true
            }
            SidebarResourceFailureRecovery::InboxPagination
            | SidebarResourceFailureRecovery::InboxMutation => true,
            SidebarResourceFailureRecovery::Children => {
                self.reconcile_node_state(page_shell);
                self.rebuild_rows(page_shell);
                true
            }
        }
    }

    fn inbox_job(
        &self,
        context: &SidebarResourceContext,
        size: u32,
        kind: SidebarInboxLoadKind,
    ) -> Option<SidebarResourceJob> {
        let workspace_api = context.workspace_api.as_ref()?;
        let current_board_url = context.board_url.as_ref()?;
        Some(SidebarResourceJob::inbox(
            LoadSidebarInboxRequest {
                current_board_url: current_board_url.clone(),
                filter: self.inbox_filter,
                size,
            },
            workspace_api.clone(),
            kind,
        ))
    }

    fn inbox_scroll_remaining(&self) -> f32 {
        self.inbox_scroll_handle.max_offset().y.as_f32()
            + self.inbox_scroll_handle.offset().y.as_f32()
    }

    fn complete_calendar_resource(
        &mut self,
        result: NotionWorkspaceResult<crate::model::LoadSidebarCalendarResult>,
        page_shell: Option<&PageShellSnapshot>,
    ) -> SidebarResourceEffect {
        if !matches!(&self.calendar, NotionSidebarCalendarState::Loading) {
            return SidebarResourceEffect::None;
        }
        self.calendar = match result {
            Ok(result) if result.available => {
                NotionSidebarCalendarState::Loaded(result.events.into())
            }
            Ok(_) => NotionSidebarCalendarState::Unavailable,
            Err(error) => {
                return resource_failure(
                    "sidebar calendar loading failed",
                    error,
                    SidebarResourceFailureRecovery::Calendar,
                );
            }
        };
        self.rebuild_rows(page_shell);
        SidebarResourceEffect::Notify
    }

    fn complete_chats_resource(
        &mut self,
        result: NotionWorkspaceResult<crate::model::LoadSidebarChatsResult>,
        page_shell: Option<&PageShellSnapshot>,
    ) -> SidebarResourceEffect {
        if !matches!(&self.chats, NotionSidebarChatsState::Loading) {
            return SidebarResourceEffect::None;
        }
        self.chats = match result {
            Ok(result) => NotionSidebarChatsState::Loaded {
                threads: result.threads.into(),
            },
            Err(error) => {
                return resource_failure(
                    "sidebar chats failed to load",
                    error,
                    SidebarResourceFailureRecovery::Chats,
                );
            }
        };
        self.rebuild_rows(page_shell);
        SidebarResourceEffect::Notify
    }

    fn complete_inbox_resource(
        &mut self,
        completion: SidebarInboxCompletion,
    ) -> SidebarResourceEffect {
        match completion.kind {
            SidebarInboxLoadKind::Initial => self.complete_initial_inbox(completion),
            SidebarInboxLoadKind::More => self.complete_more_inbox(completion),
        }
    }

    fn complete_initial_inbox(
        &mut self,
        completion: SidebarInboxCompletion,
    ) -> SidebarResourceEffect {
        if !matches!(&self.inbox, NotionSidebarInboxState::Loading)
            || !self.inbox_request_is_current(completion.filter, completion.size)
        {
            return SidebarResourceEffect::None;
        }
        self.inbox = match completion.result {
            Ok(result) => loaded_inbox(result),
            Err(error) => {
                return resource_failure(
                    "sidebar inbox failed to load",
                    error,
                    SidebarResourceFailureRecovery::Inbox,
                );
            }
        };
        self.inbox_loading_more = false;
        SidebarResourceEffect::Notify
    }

    fn complete_more_inbox(&mut self, completion: SidebarInboxCompletion) -> SidebarResourceEffect {
        if !self.inbox_loading_more
            || !self.inbox_request_is_current(completion.filter, completion.size)
        {
            return SidebarResourceEffect::None;
        }
        self.inbox_loading_more = false;
        match completion.result {
            Ok(result) => self.inbox = loaded_inbox(result),
            Err(error) => {
                return resource_failure(
                    "sidebar inbox pagination failed",
                    error,
                    SidebarResourceFailureRecovery::InboxPagination,
                );
            }
        }
        SidebarResourceEffect::Notify
    }

    fn complete_inbox_mutation(
        &mut self,
        result: NotionWorkspaceResult<()>,
    ) -> SidebarResourceEffect {
        self.inbox_action_in_flight = false;
        match result {
            Ok(()) => {
                self.inbox = NotionSidebarInboxState::Idle;
                SidebarResourceEffect::LoadInbox
            }
            Err(error) => resource_failure(
                "sidebar inbox mutation failed",
                error,
                SidebarResourceFailureRecovery::InboxMutation,
            ),
        }
    }

    fn inbox_request_is_current(&self, filter: NotionSidebarInboxFilter, size: u32) -> bool {
        self.inbox_filter == filter && self.inbox_request_size == size
    }
}

fn loaded_inbox(result: LoadSidebarInboxResult) -> NotionSidebarInboxState {
    NotionSidebarInboxState::Loaded {
        items: result.items.into(),
        has_more: result.has_more,
    }
}

fn resource_failure(
    operation: &'static str,
    error: NotionWorkspaceOperationFailure,
    recovery: SidebarResourceFailureRecovery,
) -> SidebarResourceEffect {
    SidebarResourceEffect::Failure(Box::new(SidebarResourceFailure {
        operation,
        error,
        recovery,
    }))
}
