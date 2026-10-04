use std::{sync::Arc, time::Duration};

use gpui::Context;

use crate::model::{NotionWorkspaceOperationFailure, NotionWorkspaceResult};
use crate::ui::{CardPage, LoadedCardPage, LoadedCardPageData, NotionWorkspaceApi, SurfaceState};

use super::super::PageMutationRunToken;

const RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
const RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct PageHydrationController {
    state: Option<PageHydrationState>,
}

enum PageHydrationState {
    Pending {
        retry_attempt: u32,
    },
    Loading {
        page_id: String,
        initial_data: Arc<LoadedCardPageData>,
        retry_attempt: u32,
    },
    WaitingToRetry {
        page_id: String,
        initial_data: Arc<LoadedCardPageData>,
        retry_attempt: u32,
    },
}

#[derive(Clone)]
pub(crate) struct PageHydrationRequest {
    pub(crate) page_id: String,
    initial_data: Arc<LoadedCardPageData>,
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    retry_attempt: u32,
    pub(crate) authority: Option<PageMutationRunToken>,
}

pub(crate) struct PageHydrationJob(PageHydrationRequest);

pub(crate) struct PageHydrationRetry {
    request: PageHydrationRequest,
    delay: Duration,
}

pub(crate) enum PageHydrationEvent {
    Completed(Box<PageHydrationCompletion>),
    Retry(Box<PageHydrationRequest>),
}

pub(crate) struct PageHydrationCompletion {
    pub(crate) request: PageHydrationRequest,
    pub(crate) result: NotionWorkspaceResult<CardPage>,
}

pub(crate) enum PageHydrationEffect {
    None,
    Loaded(Box<PageHydrationLoaded>),
    ReportFailure(Box<PageHydrationFailure>),
}

pub(crate) struct PageHydrationLoaded {
    pub(crate) page: CardPage,
    pub(crate) authority: Option<PageMutationRunToken>,
}

pub(crate) struct PageHydrationFailure {
    pub(crate) request: PageHydrationRequest,
    pub(crate) error: NotionWorkspaceOperationFailure,
}

#[derive(Clone, Copy)]
pub(crate) struct PageHydrationReadiness {
    active_drag: bool,
    pending_mutation: bool,
}

impl PageHydrationReadiness {
    pub(crate) const fn new(active_drag: bool, pending_mutation: bool) -> Self {
        Self {
            active_drag,
            pending_mutation,
        }
    }

    fn blocked(self) -> bool {
        self.active_drag || self.pending_mutation
    }
}

impl PageHydrationController {
    pub(super) fn new(requested: bool) -> Self {
        let mut controller = Self::default();
        if requested {
            controller.request();
        }
        controller
    }

    pub(super) fn request(&mut self) {
        self.state = Some(PageHydrationState::Pending { retry_attempt: 0 });
    }

    pub(super) fn prepare(
        &mut self,
        page: Option<&LoadedCardPage>,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
        authority: Option<PageMutationRunToken>,
        readiness: PageHydrationReadiness,
    ) -> Option<PageHydrationJob> {
        if readiness.active_drag {
            return None;
        }
        let retry_attempt = match self.state.as_ref() {
            Some(PageHydrationState::Pending { retry_attempt }) => *retry_attempt,
            _ => return None,
        };
        let (Some(page), Some(workspace_api)) = (page, workspace_api) else {
            self.state = None;
            return None;
        };
        if readiness.pending_mutation {
            return None;
        }
        let request = PageHydrationRequest {
            page_id: page.data.page.block_id.clone(),
            initial_data: page.data.clone(),
            workspace_api,
            retry_attempt,
            authority,
        };
        self.state = Some(request.loading_state());
        Some(PageHydrationJob(request))
    }

    pub(super) fn complete(
        &mut self,
        page: Option<&LoadedCardPage>,
        completion: PageHydrationCompletion,
        current_api: Option<Arc<dyn NotionWorkspaceApi>>,
        readiness: PageHydrationReadiness,
    ) -> PageHydrationEffect {
        let PageHydrationCompletion {
            mut request,
            result,
        } = completion;
        if !self.loading_is_current(&request, current_api.as_ref()) {
            return PageHydrationEffect::None;
        }
        if readiness.blocked() {
            self.state = Some(PageHydrationState::Pending {
                retry_attempt: request.retry_attempt,
            });
            return PageHydrationEffect::None;
        }
        if !request.initial_page_is_current(page) {
            self.reset(page, &request.page_id);
            return PageHydrationEffect::None;
        }
        match result {
            Ok(page) => {
                self.state = None;
                PageHydrationEffect::Loaded(Box::new(PageHydrationLoaded {
                    page,
                    authority: request.authority,
                }))
            }
            Err(error) => {
                request.retry_attempt = request.retry_attempt.saturating_add(1);
                PageHydrationEffect::ReportFailure(Box::new(PageHydrationFailure {
                    request,
                    error,
                }))
            }
        }
    }

    pub(super) fn wait_to_retry(&mut self, request: PageHydrationRequest) -> PageHydrationRetry {
        self.state = Some(request.waiting_state());
        PageHydrationRetry {
            delay: retry_delay(request.retry_attempt),
            request,
        }
    }

    pub(super) fn resume(
        &mut self,
        page: Option<&LoadedCardPage>,
        request: &PageHydrationRequest,
        current_api: Option<&Arc<dyn NotionWorkspaceApi>>,
    ) -> bool {
        if !self.retry_is_current(request) {
            return false;
        }
        if !request.initial_page_is_current(page) || !request.api_is_current(current_api) {
            self.reset(page, &request.page_id);
            return false;
        }
        self.state = Some(PageHydrationState::Pending {
            retry_attempt: request.retry_attempt,
        });
        true
    }

    fn loading_is_current(
        &self,
        request: &PageHydrationRequest,
        current_api: Option<&Arc<dyn NotionWorkspaceApi>>,
    ) -> bool {
        request.api_is_current(current_api)
            && self.state.as_ref().is_some_and(|state| {
                matches!(
                    state,
                    PageHydrationState::Loading {
                        page_id,
                        initial_data,
                        retry_attempt,
                    } if page_id == &request.page_id
                        && Arc::ptr_eq(initial_data, &request.initial_data)
                        && retry_attempt == &request.retry_attempt
                )
            })
    }

    fn retry_is_current(&self, request: &PageHydrationRequest) -> bool {
        self.state.as_ref().is_some_and(|state| {
            matches!(
                state,
                PageHydrationState::WaitingToRetry {
                    page_id,
                    initial_data,
                    retry_attempt,
                } if page_id == &request.page_id
                    && Arc::ptr_eq(initial_data, &request.initial_data)
                    && retry_attempt == &request.retry_attempt
            )
        })
    }

    fn reset(&mut self, page: Option<&LoadedCardPage>, page_id: &str) {
        self.state = page
            .is_some_and(|page| page.data.page.block_id == page_id)
            .then_some(PageHydrationState::Pending { retry_attempt: 0 });
    }
}

impl PageHydrationRequest {
    fn loading_state(&self) -> PageHydrationState {
        PageHydrationState::Loading {
            page_id: self.page_id.clone(),
            initial_data: self.initial_data.clone(),
            retry_attempt: self.retry_attempt,
        }
    }

    fn waiting_state(&self) -> PageHydrationState {
        PageHydrationState::WaitingToRetry {
            page_id: self.page_id.clone(),
            initial_data: self.initial_data.clone(),
            retry_attempt: self.retry_attempt,
        }
    }

    fn api_is_current(&self, current: Option<&Arc<dyn NotionWorkspaceApi>>) -> bool {
        current.is_some_and(|current| Arc::ptr_eq(current, &self.workspace_api))
    }

    fn initial_page_is_current(&self, page: Option<&LoadedCardPage>) -> bool {
        page.is_some_and(|page| {
            page.data.page.block_id == self.page_id && Arc::ptr_eq(&page.data, &self.initial_data)
        })
    }
}

impl PageHydrationJob {
    pub(crate) fn spawn<Apply>(self, cx: &mut Context<SurfaceState>, apply: Apply)
    where
        Apply: FnOnce(&mut SurfaceState, PageHydrationEvent, &mut Context<SurfaceState>) + 'static,
    {
        let workspace_api = self.0.workspace_api.clone();
        gpui_components::spawn_background_task_for_entity(
            self.0,
            cx,
            move |request| {
                let result = workspace_api.load_card_page(&request.page_id);
                PageHydrationEvent::Completed(Box::new(PageHydrationCompletion { request, result }))
            },
            apply,
        );
    }
}

impl PageHydrationRetry {
    pub(crate) fn spawn<Apply>(self, cx: &mut Context<SurfaceState>, apply: Apply)
    where
        Apply: FnOnce(&mut SurfaceState, PageHydrationEvent, &mut Context<SurfaceState>) + 'static,
    {
        gpui_components::spawn_timer_task_for_entity(
            self.request,
            self.delay,
            cx,
            move |surface, request, cx| {
                apply(surface, PageHydrationEvent::Retry(Box::new(request)), cx);
            },
        );
    }
}

fn retry_delay(retry_attempt: u32) -> Duration {
    let exponent = retry_attempt.saturating_sub(1).min(5);
    RETRY_BASE_DELAY
        .checked_mul(1u32 << exponent)
        .unwrap_or(RETRY_MAX_DELAY)
        .min(RETRY_MAX_DELAY)
}
