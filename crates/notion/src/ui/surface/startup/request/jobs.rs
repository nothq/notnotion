use super::{handle_notion_startup_event, NotionBootstrapApiRef};
use crate::model::{
    NotionBootstrapFailure, NotionCachedBootstrapOutcome, NotionWorkspaceBootstrap,
};
use crate::ui::SurfaceState;
use gpui::Context;
use gpui_components::{spawn_background_task_for_entity, spawn_timer_task_for_entity};
use std::time::Duration;

const RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
const RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

pub(super) struct NotionStartupJob {
    api: NotionBootstrapApiRef,
    generation: u64,
    kind: NotionStartupJobKind,
}

enum NotionStartupJobKind {
    Cached,
    Live,
}

pub(super) enum NotionStartupEvent {
    Cached {
        generation: u64,
        result: NotionCachedBootstrapOutcome,
    },
    Live {
        generation: u64,
        result: Result<Box<NotionWorkspaceBootstrap>, NotionBootstrapFailure>,
    },
    Retry(NotionStartupRetry),
}

#[derive(Clone, Copy)]
pub(super) struct NotionStartupRetry {
    pub(super) generation: u64,
    pub(super) attempt: u32,
}

impl NotionStartupJob {
    pub(super) fn cached(api: NotionBootstrapApiRef, generation: u64) -> Self {
        Self {
            api,
            generation,
            kind: NotionStartupJobKind::Cached,
        }
    }

    pub(super) fn live(api: NotionBootstrapApiRef, generation: u64) -> Self {
        Self {
            api,
            generation,
            kind: NotionStartupJobKind::Live,
        }
    }

    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        if matches!(self.kind, NotionStartupJobKind::Live) {
            cx.notify();
        }
        spawn_background_task_for_entity(self, cx, Self::run, handle_notion_startup_event);
    }

    fn run(self) -> NotionStartupEvent {
        match self.kind {
            NotionStartupJobKind::Cached => NotionStartupEvent::Cached {
                generation: self.generation,
                result: self.api.load_cached_workspace(),
            },
            NotionStartupJobKind::Live => NotionStartupEvent::Live {
                generation: self.generation,
                result: self.api.bootstrap_workspace().map(Box::new),
            },
        }
    }
}

impl NotionStartupRetry {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        let exponent = self.attempt.saturating_sub(1).min(5);
        let multiplier = 1u32 << exponent;
        let delay = RETRY_BASE_DELAY
            .checked_mul(multiplier)
            .unwrap_or(RETRY_MAX_DELAY)
            .min(RETRY_MAX_DELAY);
        spawn_timer_task_for_entity(self, delay, cx, |surface, retry, cx| {
            handle_notion_startup_event(surface, NotionStartupEvent::Retry(retry), cx);
        });
    }
}
