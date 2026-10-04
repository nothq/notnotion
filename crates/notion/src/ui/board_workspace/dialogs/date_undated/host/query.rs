use crate::ui::board_workspace::PageDocumentAction;
use std::collections::VecDeque;

use gpui::{AppContext, Context, WeakEntity};
use gpui_components::text_input::TextInput;

use super::super::{
    DateUndatedDialogCommand, DateUndatedDialogSource, DateUndatedDialogState, DateViewContext,
    DateViewEffect, DateViewRequest,
};
use crate::model::{BoardItem, NotionWorkspaceApi};
use crate::ui::surface::{
    DatabaseSearchState, NotionChromeState, NotionDateViewState, NotionSearchState,
};
use crate::ui::{Arc, Card, SurfaceState, Theme};

impl SurfaceState {
    pub(in crate::ui::board_workspace) fn execute_date_view_effects(
        &mut self,
        effects: Vec<DateViewEffect>,
        cx: &mut Context<Self>,
    ) {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            let workspace_api = self.notion_startup.workspace_api();
            let context = DateViewContext::from_board(&self.board, workspace_api.is_some());
            let page_host = self.presentation.page_host.clone();
            let outcome = DateViewEffectHost {
                state: &mut self.date_view,
                board_items: &self.board.items,
                undated_count: &mut self.board.date_undated_count,
                chrome: &mut self.notion_chrome,
                database_search: &mut self.database_search,
                notion_search: &self.notion_search,
                page_host,
                theme: self.theme,
                context,
                workspace_api,
            }
            .execute(effect, cx);
            match outcome {
                DateViewHostOutcome::Done => {}
                DateViewHostOutcome::Generated(effects) => pending.extend(effects),
                DateViewHostOutcome::OpenItem(item) => {
                    self.dispatch_page_document_action(PageDocumentAction::OpenBoardItem(*item), cx)
                }
                DateViewHostOutcome::OpenCard(block_id) => self.dispatch_page_document_action(
                    PageDocumentAction::OpenCard(Card {
                        title: String::new(),
                        block_id,
                        height: 40.0,
                        has_content: false,
                        icon: None,
                        fill_override: None,
                    }),
                    cx,
                ),
            }
        }
    }
}

struct DateViewEffectHost<'a> {
    state: &'a mut NotionDateViewState,
    board_items: &'a [BoardItem],
    undated_count: &'a mut Option<usize>,
    chrome: &'a mut NotionChromeState,
    database_search: &'a mut DatabaseSearchState,
    notion_search: &'a NotionSearchState,
    page_host: Option<WeakEntity<SurfaceState>>,
    theme: Theme,
    context: DateViewContext,
    workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
}

enum DateViewHostOutcome {
    Done,
    Generated(Vec<DateViewEffect>),
    OpenItem(Box<BoardItem>),
    OpenCard(String),
}

impl DateViewEffectHost<'_> {
    fn execute(
        &mut self,
        effect: DateViewEffect,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        match effect {
            DateViewEffect::Notify => {
                cx.notify();
                DateViewHostOutcome::Done
            }
            DateViewEffect::OpenExternalCalendar => {
                cx.open_url("https://calendar.notion.so/");
                DateViewHostOutcome::Done
            }
            DateViewEffect::OpenBoardItem(item) => DateViewHostOutcome::OpenItem(item),
            DateViewEffect::OpenCard(block_id) => DateViewHostOutcome::OpenCard(block_id),
            DateViewEffect::ClearUndatedCount => {
                *self.undated_count = None;
                DateViewHostOutcome::Done
            }
            DateViewEffect::SetUndatedCount(count) => {
                *self.undated_count = Some(count);
                DateViewHostOutcome::Done
            }
            DateViewEffect::ClearUndatedDialog(session) => {
                self.clear_dialog(&session, cx);
                DateViewHostOutcome::Done
            }
            DateViewEffect::ToggleUndatedDialog(command) => self.toggle_dialog(command, cx),
            DateViewEffect::CloseHostedUndatedDialog => {
                self.dismiss_hosted_dialog(cx);
                DateViewHostOutcome::Done
            }
            DateViewEffect::CloseUndatedDialogFromSource => {
                self.close_dialog_from_source(cx);
                DateViewHostOutcome::Done
            }
            DateViewEffect::OpenUndatedSource(session) => self.open_current_source(session, cx),
            DateViewEffect::CloseUndatedSource(session) => {
                self.close_current_source(&session, cx);
                DateViewHostOutcome::Done
            }
            DateViewEffect::NotifyDateDragHosts => {
                self.notify_drag_hosts(cx);
                DateViewHostOutcome::Done
            }
            DateViewEffect::Request(request) => self.execute_request(*request, cx),
        }
    }

    fn execute_request(
        &mut self,
        request: DateViewRequest,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        match request {
            DateViewRequest::CalendarMonth(month) => return self.load_calendar_month(month, cx),
            DateViewRequest::CreateCalendarPage(job) => {
                job.spawn(self.required_workspace_api(), cx)
            }
            DateViewRequest::ResizeCalendarRange(job) => {
                job.spawn(self.required_workspace_api(), cx)
            }
            DateViewRequest::AssignCalendarDate(job) => {
                job.spawn(self.required_workspace_api(), cx)
            }
            DateViewRequest::UndatedItems(job) => {
                job.spawn(self.required_workspace_api(), cx);
                cx.notify();
            }
            DateViewRequest::UndatedCount(job) => job.spawn(self.required_workspace_api(), cx),
            DateViewRequest::UndatedCountRetry(retry) => retry.spawn(cx),
        }
        DateViewHostOutcome::Done
    }

    fn required_workspace_api(&self) -> Arc<dyn NotionWorkspaceApi> {
        self.workspace_api
            .clone()
            .expect("prepared Notion date request must retain its workspace API")
    }

    fn load_calendar_month(
        &mut self,
        month: crate::ui::CivilDate,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        if !self.context.calendar_is_active() {
            return DateViewHostOutcome::Done;
        }
        let load = self.state.prepare_month_load(
            month,
            self.context.active_view_identity.clone(),
            self.board_items,
            self.workspace_api.is_some(),
        );
        if let Some(load) = load {
            let workspace_api = self
                .workspace_api
                .clone()
                .expect("prepared Notion Calendar month request must retain its workspace API");
            load.spawn(workspace_api, cx);
        }
        cx.notify();
        DateViewHostOutcome::Done
    }

    fn toggle_dialog(
        &mut self,
        command: DateUndatedDialogCommand,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        let already_open = self
            .chrome
            .date_undated_dialog
            .as_ref()
            .is_some_and(|state| state.source.matches_session(command.source.session()));
        if already_open {
            self.dismiss_hosted_dialog(cx);
            return DateViewHostOutcome::Done;
        }
        self.dismiss_hosted_dialog(cx);
        self.close_competing_dialogs();
        self.notion_search.input.borrow_mut().take();
        self.chrome.date_undated_dialog = Some(DateUndatedDialogState {
            anchor: command.anchor,
            source: command.source.clone(),
        });
        let outcome = self.open_source(&command.source, cx);
        cx.notify();
        outcome
    }

    fn dismiss_hosted_dialog(&mut self, cx: &mut Context<SurfaceState>) {
        let Some(state) = self.chrome.date_undated_dialog.take() else {
            return;
        };
        self.close_source(&state.source, cx);
        cx.notify();
    }

    fn open_source(
        &mut self,
        source: &DateUndatedDialogSource,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        match source {
            DateUndatedDialogSource::Full { session } => {
                self.open_current_source(session.clone(), cx)
            }
            DateUndatedDialogSource::Inline { session, surface } => {
                let session = session.clone();
                let _ = surface.update(cx, move |source, cx| {
                    source.execute_date_view_effects(
                        vec![DateViewEffect::OpenUndatedSource(session)],
                        cx,
                    );
                });
                DateViewHostOutcome::Done
            }
        }
    }

    fn close_source(&mut self, source: &DateUndatedDialogSource, cx: &mut Context<SurfaceState>) {
        match source {
            DateUndatedDialogSource::Full { session } => self.close_current_source(session, cx),
            DateUndatedDialogSource::Inline { session, surface } => {
                let session = session.clone();
                let _ = surface.update(cx, move |source, cx| {
                    source.execute_date_view_effects(
                        vec![DateViewEffect::CloseUndatedSource(session)],
                        cx,
                    );
                });
            }
        }
    }

    fn open_current_source(
        &mut self,
        session: Arc<()>,
        cx: &mut Context<SurfaceState>,
    ) -> DateViewHostOutcome {
        if !Arc::ptr_eq(&session, &self.state.query_session) {
            return DateViewHostOutcome::Done;
        }
        let Some(view_identity) = self.context.active_view_identity.clone() else {
            return DateViewHostOutcome::Done;
        };
        let mut effects =
            if self.state.undated.view_identity.as_deref() == Some(view_identity.as_str()) {
                Vec::new()
            } else {
                self.state.reset_undated_for_view_effects(
                    Some(view_identity),
                    self.context.workspace_available,
                )
            };
        self.close_competing_dialogs();
        self.state.undated.open();
        let actions = super::date_undated_action_sink(cx, session.clone());
        let props = super::super::dialog::date_undated_search_input_props(self.theme, actions);
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.state.undated.input.borrow_mut().replace(input);
        if let Some(query) = self.state.undated.prepare_query_effect(
            &self.context,
            session,
            super::super::DATE_UNDATED_PAGE_SIZE,
            false,
        ) {
            effects.push(query);
        }
        DateViewHostOutcome::Generated(effects)
    }

    fn close_current_source(&mut self, session: &Arc<()>, cx: &mut Context<SurfaceState>) {
        if Arc::ptr_eq(session, &self.state.query_session) {
            self.state.undated.close();
            cx.notify();
        }
    }

    fn close_dialog_from_source(&mut self, cx: &mut Context<SurfaceState>) {
        let session = self.state.query_session.clone();
        self.state.undated.close();
        cx.notify();
        if let Some(parent) = self.page_host.clone() {
            let _ = parent.update(cx, move |parent, cx| {
                if parent
                    .notion_chrome
                    .clear_date_undated_dialog_for_source_session(&session)
                {
                    cx.notify();
                }
            });
        } else if self
            .chrome
            .clear_date_undated_dialog_for_source_session(&session)
        {
            cx.notify();
        }
    }

    fn clear_dialog(&mut self, session: &Arc<()>, cx: &mut Context<SurfaceState>) {
        if self
            .chrome
            .clear_date_undated_dialog_for_source_session(session)
        {
            cx.notify();
        }
        if let Some(parent) = self.page_host.clone() {
            let session = session.clone();
            let _ = parent.update(cx, move |parent, cx| {
                if parent
                    .notion_chrome
                    .clear_date_undated_dialog_for_source_session(&session)
                {
                    cx.notify();
                }
            });
        }
    }

    fn close_competing_dialogs(&mut self) {
        self.chrome.close_competing_date_undated_dialogs();
        self.database_search.close();
    }

    fn notify_drag_hosts(&self, cx: &mut Context<SurfaceState>) {
        cx.notify();
        if let Some(parent) = self.page_host.as_ref() {
            let _ = parent.update(cx, |_, cx| cx.notify());
        }
    }
}
