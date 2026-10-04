use super::super::{NotionStartup, SurfaceState};
use crate::model::{NotionBootstrapApi, NotionCachedBootstrapOutcome};
use gpui::Context;
use std::{mem, sync::Arc};

mod failure;
mod jobs;
use jobs::{NotionStartupEvent, NotionStartupJob};

type NotionBootstrapApiRef = Arc<dyn NotionBootstrapApi>;

impl SurfaceState {
    pub(crate) fn handle_notion_workspace_failure(
        &mut self,
        operation: &str,
        failure: crate::model::NotionWorkspaceOperationFailure,
        cx: &mut Context<Self>,
    ) -> bool {
        let (message, rebootstrap_api) = failure.into_parts();
        self.print_notion_error(format!("{operation}: {message}"));
        let Some(rebootstrap_api) = rebootstrap_api else {
            return false;
        };
        self.restart_notion_startup(rebootstrap_api, cx);
        true
    }

    pub(crate) fn restart_notion_startup(
        &mut self,
        bootstrap_api: NotionBootstrapApiRef,
        cx: &mut Context<Self>,
    ) {
        let generation = self
            .notion_startup_generation
            .checked_add(1)
            .expect("Notion startup generation overflowed");
        let mut replacement = Self::from_startup(
            NotionStartup::Loading {
                bootstrap_api,
                request_started: false,
                cache_request_started: false,
                cached: None,
                retry_attempt: 0,
            },
            self.appearance_mode,
            self.viewport,
            self.surface_active,
            self.notion_resources.clone(),
        );
        self.page_mutations.invalidate_session();
        replacement.preview_width = self.preview_width;
        replacement.page_mutations = mem::take(&mut self.page_mutations);
        replacement.notion_startup_generation = generation;
        *self = replacement;
        cx.notify();
        self.ensure_notion_startup(cx);
    }

    pub(crate) fn ensure_notion_startup(&mut self, cx: &mut Context<Self>) {
        if !self.surface_active {
            return;
        }
        let (cache_api, bootstrap_api) = self.notion_startup.take_requests();
        if let Some(cache_api) = cache_api {
            NotionStartupJob::cached(cache_api, self.notion_startup_generation).spawn(cx);
        }
        if let Some(bootstrap_api) = bootstrap_api {
            NotionStartupJob::live(bootstrap_api, self.notion_startup_generation).spawn(cx);
        }
    }

    fn install_notion_startup_failure(
        &mut self,
        startup: NotionStartup,
        discard_previous_state: bool,
    ) {
        if !discard_previous_state {
            self.notion_startup = startup;
            return;
        }
        let mut replacement = Self::from_startup(
            startup,
            self.appearance_mode,
            self.viewport,
            self.surface_active,
            self.notion_resources.clone(),
        );
        self.page_mutations.invalidate_session();
        replacement.preview_width = self.preview_width;
        replacement.page_mutations = mem::take(&mut self.page_mutations);
        replacement.notion_startup_generation = self.notion_startup_generation;
        *self = replacement;
    }
}

fn handle_notion_startup_event(
    surface: &mut SurfaceState,
    event: NotionStartupEvent,
    cx: &mut Context<SurfaceState>,
) {
    match event {
        NotionStartupEvent::Cached { generation, result } => match result {
            NotionCachedBootstrapOutcome::Loaded(cached) => {
                surface.apply_notion_cached_bootstrap(generation, *cached, cx)
            }
            NotionCachedBootstrapOutcome::Failed(error) => {
                surface.print_notion_error(format!("workspace cache unavailable: {error}"))
            }
            NotionCachedBootstrapOutcome::Miss => {}
        },
        NotionStartupEvent::Live { generation, result } => match result {
            Ok(bootstrap) => surface.apply_notion_bootstrap(generation, *bootstrap, cx),
            Err(failure) => {
                let Some(plan) = surface.notion_startup.prepare_failure(
                    surface.notion_startup_generation,
                    generation,
                    failure,
                ) else {
                    return;
                };
                surface.notion_startup_generation = plan.retry.generation;
                surface.print_notion_error(plan.message);
                surface.install_notion_startup_failure(plan.startup, plan.discard_previous_state);
                plan.retry.spawn(cx);
                cx.notify();
            }
        },
        NotionStartupEvent::Retry(retry) => {
            if surface
                .notion_startup
                .resume_retry(surface.notion_startup_generation, retry)
            {
                cx.notify();
                surface.ensure_notion_startup(cx);
            }
        }
    }
}
