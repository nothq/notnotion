use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::Context;

use crate::model::NotionWorkspaceApi;
use crate::ui::search::action::{
    QuickFindAction, QuickFindActionInput, QuickFindCompletion, QuickFindEffect, QuickFindFailure,
    QuickFindNavigation, QuickFindPresentationEffect, QuickFindRequest,
};
use crate::ui::surface::{NotionSearchState, PageMutationCoordinator};
use crate::ui::{ClipboardItem, KeyDownEvent, SurfaceState};

mod navigation;
mod preview;

impl SurfaceState {
    pub(crate) fn handle_notion_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.notion_chrome.notion_search_open {
            return false;
        }
        let Some(action) = quick_find_key_action(event) else {
            return false;
        };
        let input = quick_find_action_input(&action);
        let effects = self.notion_search.reduce_quick_find_action(action, input);
        self.execute_quick_find_effects(effects, cx);
        true
    }

    pub(crate) fn toggle_notion_search(&mut self, cx: &mut Context<Self>) {
        if self.notion_chrome.notion_search_open {
            self.execute_quick_find_effects(vec![close_effect()], cx);
        } else {
            self.activate_notion_search(cx);
        }
    }

    pub(crate) fn activate_notion_search(&mut self, cx: &mut Context<Self>) {
        if self.notion_chrome.notion_search_open {
            self.execute_quick_find_effects(
                vec![
                    QuickFindEffect::Presentation(QuickFindPresentationEffect::FocusInput),
                    QuickFindEffect::Notify,
                ],
                cx,
            );
            return;
        }
        self.page_editor.reset_page_composer();
        self.notion_chrome.ai_autofill_dialog = None;
        self.notion_chrome.toolbar_dialog = None;
        self.database_search.close();
        self.notion_chrome.notion_search_open = true;
        self.notion_search.begin_session(quick_find_session_id());
        self.execute_quick_find_effects(
            vec![
                QuickFindEffect::Request(QuickFindRequest::RefreshRecents),
                QuickFindEffect::Notify,
            ],
            cx,
        );
    }

    pub(crate) fn refresh_notion_search_after_page_mutation(&mut self, cx: &mut Context<Self>) {
        let effects = self
            .notion_search
            .refresh_quick_find_after_page_mutation(self.notion_chrome.notion_search_open);
        self.execute_quick_find_effects(effects, cx);
    }

    pub(crate) fn finish_quick_find_completion(
        &mut self,
        completion: QuickFindCompletion,
        cx: &mut Context<Self>,
    ) {
        let effects = completion.finish(
            &mut self.notion_search,
            &mut self.page_mutations,
            self.notion_chrome.notion_search_open,
        );
        self.execute_quick_find_effects(effects, cx);
    }

    pub(crate) fn execute_quick_find_effects(
        &mut self,
        effects: Vec<QuickFindEffect>,
        cx: &mut Context<Self>,
    ) {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            let workspace_api = self.notion_startup.workspace_api();
            let current_board_url = self.notion_startup.board_url();
            let current_page_id = self
                .board
                .share_target_id
                .as_ref()
                .map(|target| target.as_str().to_string());
            let outcome = QuickFindEffectHost {
                state: &mut self.notion_search,
                page_mutations: &self.page_mutations,
                search_open: &mut self.notion_chrome.notion_search_open,
                workspace_api,
                current_board_url,
                current_page_id,
            }
            .execute(effect, cx);
            match outcome {
                QuickFindHostOutcome::Done => {}
                QuickFindHostOutcome::Generated(effects) => pending.extend(effects),
                QuickFindHostOutcome::Navigate { board_url, title } => {
                    self.open_notion_workspace(board_url, title, cx);
                }
                QuickFindHostOutcome::Failure(failure) => {
                    if self.handle_notion_workspace_failure(failure.operation, failure.error, cx) {
                        return;
                    }
                    pending.extend(self.notion_search.recover_quick_find_failure(
                        failure.recovery,
                        self.notion_chrome.notion_search_open,
                    ));
                }
            }
        }
    }
}

struct QuickFindEffectHost<'a> {
    state: &'a mut NotionSearchState,
    page_mutations: &'a PageMutationCoordinator,
    search_open: &'a mut bool,
    workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    current_board_url: Option<String>,
    current_page_id: Option<String>,
}

enum QuickFindHostOutcome {
    Done,
    Generated(Vec<QuickFindEffect>),
    Navigate { board_url: String, title: String },
    Failure(Box<QuickFindFailure>),
}

impl QuickFindEffectHost<'_> {
    fn execute(
        mut self,
        effect: QuickFindEffect,
        cx: &mut Context<SurfaceState>,
    ) -> QuickFindHostOutcome {
        match effect {
            QuickFindEffect::Presentation(effect) => self.present(effect, cx),
            QuickFindEffect::Request(request) => {
                let effects = QuickFindRequestHost {
                    state: self.state,
                    page_mutations: self.page_mutations,
                    workspace_api: self.workspace_api,
                    current_board_url: self.current_board_url,
                    current_page_id: self.current_page_id,
                    search_open: *self.search_open,
                }
                .execute(request, cx);
                QuickFindHostOutcome::Generated(effects)
            }
            QuickFindEffect::Schedule(timer) => {
                preview::schedule(timer, cx);
                QuickFindHostOutcome::Done
            }
            QuickFindEffect::RecordAcceptedQuery(accepted) => {
                if let Some(api) = self.workspace_api {
                    api.record_accepted_quick_find_query_response(
                        &accepted.request,
                        &accepted.result,
                    );
                }
                QuickFindHostOutcome::Done
            }
            QuickFindEffect::Failure(failure) => QuickFindHostOutcome::Failure(failure),
            QuickFindEffect::Notify => {
                cx.notify();
                QuickFindHostOutcome::Done
            }
        }
    }

    fn present(
        &mut self,
        effect: QuickFindPresentationEffect,
        cx: &mut Context<SurfaceState>,
    ) -> QuickFindHostOutcome {
        match effect {
            QuickFindPresentationEffect::Close if *self.search_open => {
                *self.search_open = false;
                self.state.input.borrow_mut().take();
                cx.notify();
                QuickFindHostOutcome::Done
            }
            QuickFindPresentationEffect::Close => QuickFindHostOutcome::Done,
            QuickFindPresentationEffect::FocusInput => {
                if let Some(input) = self.state.input.borrow().clone() {
                    input.update(cx, |input, cx| input.request_focus(cx));
                }
                QuickFindHostOutcome::Done
            }
            QuickFindPresentationEffect::Navigate(QuickFindNavigation::Workspace {
                board_url,
                title,
            }) => QuickFindHostOutcome::Navigate { board_url, title },
            QuickFindPresentationEffect::Navigate(QuickFindNavigation::NewTab(url)) => {
                cx.open_url(&url);
                QuickFindHostOutcome::Done
            }
            QuickFindPresentationEffect::CopyLink(link) => {
                cx.write_to_clipboard(ClipboardItem::new_string(link));
                QuickFindHostOutcome::Done
            }
        }
    }
}

struct QuickFindRequestHost<'a> {
    state: &'a mut NotionSearchState,
    page_mutations: &'a PageMutationCoordinator,
    workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    current_board_url: Option<String>,
    current_page_id: Option<String>,
    search_open: bool,
}

impl QuickFindRequestHost<'_> {
    fn execute(
        mut self,
        request: QuickFindRequest,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<QuickFindEffect> {
        match request {
            QuickFindRequest::RefreshRecents => self.refresh_recents(),
            QuickFindRequest::Query(token) => {
                if let (Some(api), Some(board_url)) = (self.workspace_api, self.current_board_url) {
                    if let Some(job) = self.state.begin_quick_find_query_job(token, board_url) {
                        navigation::spawn_query(api, job, cx);
                    }
                }
                Vec::new()
            }
            QuickFindRequest::Preview(request) => {
                let Some(api) = self.workspace_api else {
                    return Vec::new();
                };
                let authority = self
                    .page_mutations
                    .latest_committed_token(&request.block_id);
                if let Some(job) = self.state.begin_quick_find_preview_job(request, authority) {
                    preview::spawn_preview(api, job, cx);
                }
                Vec::new()
            }
            QuickFindRequest::Recents(job) => {
                if let Some(api) = self.workspace_api {
                    super::recents::spawn_recents(api, *job, cx);
                }
                Vec::new()
            }
            QuickFindRequest::PersistVisit(job) => {
                if let Some(api) = self.workspace_api {
                    navigation::spawn_visit(api, *job, cx);
                }
                Vec::new()
            }
        }
    }

    fn refresh_recents(&mut self) -> Vec<QuickFindEffect> {
        let (Some(api), Some(board_url)) =
            (self.workspace_api.as_ref(), self.current_board_url.take())
        else {
            return Vec::new();
        };
        let source = super::recents::source(api, self.current_page_id.take(), board_url);
        self.state
            .begin_quick_find_recents(source, self.search_open)
    }
}

pub(super) fn quick_find_action_input(action: &QuickFindAction) -> QuickFindActionInput {
    let needs_visit_time = matches!(
        action,
        QuickFindAction::Submit { .. }
            | QuickFindAction::SelectAndOpen(_)
            | QuickFindAction::OpenSelectedInNewTab
    );
    QuickFindActionInput {
        visited_at_unix_millis: needs_visit_time.then(quick_find_visit_time).unwrap_or(0),
    }
}

fn quick_find_visit_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch for Notion Quick Find")
        .as_millis()
        .try_into()
        .expect("Notion Quick Find timestamp must fit in a u64")
}

fn quick_find_key_action(event: &KeyDownEvent) -> Option<QuickFindAction> {
    let modifiers = event.keystroke.modifiers;
    let no_modifiers = !modifiers.platform
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.function
        && !modifiers.shift;
    if event.keystroke.key == "escape" && no_modifiers {
        return Some(QuickFindAction::Close);
    }
    let platform_only = modifiers.platform
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.function
        && !modifiers.shift;
    match event.keystroke.key.as_str() {
        "l" if platform_only => Some(QuickFindAction::CopySelectedLink),
        "enter" if platform_only => Some(QuickFindAction::OpenSelectedInNewTab),
        _ => None,
    }
}

fn close_effect() -> QuickFindEffect {
    QuickFindEffect::Presentation(QuickFindPresentationEffect::Close)
}

fn quick_find_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
