use super::jobs::NotionStartupRetry;
use crate::model::NotionBootstrapFailure;
use crate::ui::surface::{NotionStartup, NotionStartupCacheRetry};

pub(super) struct NotionStartupFailurePlan {
    pub(super) startup: NotionStartup,
    pub(super) discard_previous_state: bool,
    pub(super) message: String,
    pub(super) retry: NotionStartupRetry,
}

impl NotionStartup {
    pub(in crate::ui::surface::startup::request) fn prepare_failure(
        &mut self,
        mut current_generation: u64,
        request_generation: u64,
        failure: NotionBootstrapFailure,
    ) -> Option<NotionStartupFailurePlan> {
        if current_generation != request_generation {
            return None;
        }
        let (message, disposition) = failure.into_parts();
        let (bootstrap_api, mut cached, previous_attempt) = match self {
            Self::Loading {
                bootstrap_api,
                cached,
                retry_attempt,
                ..
            } => (bootstrap_api.clone(), cached.take(), *retry_attempt),
            Self::Ready(_) | Self::Error { .. } => return None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => return None,
        };
        let discard_previous_state = disposition.discards_previous_state();
        let reload_cache =
            disposition.reloads_workspace_cache() && (discard_previous_state || cached.is_none());
        if reload_cache {
            cached = None;
            current_generation = current_generation
                .checked_add(1)
                .expect("Notion startup generation overflowed");
        }
        let attempt = previous_attempt.saturating_add(1);
        Some(NotionStartupFailurePlan {
            startup: Self::Error {
                bootstrap_api,
                cached,
                retry_attempt: attempt,
                cache_retry: if reload_cache {
                    NotionStartupCacheRetry::Reload
                } else {
                    NotionStartupCacheRetry::Preserve
                },
            },
            discard_previous_state,
            message: format!("failed to start: {message}"),
            retry: NotionStartupRetry {
                generation: current_generation,
                attempt,
            },
        })
    }

    pub(in crate::ui::surface::startup::request) fn resume_retry(
        &mut self,
        generation: u64,
        retry: NotionStartupRetry,
    ) -> bool {
        if generation != retry.generation {
            return false;
        }
        let Self::Error {
            bootstrap_api,
            cached,
            retry_attempt,
            cache_retry,
        } = self
        else {
            return false;
        };
        if *retry_attempt != retry.attempt {
            return false;
        }
        *self = Self::Loading {
            bootstrap_api: bootstrap_api.clone(),
            request_started: false,
            cache_request_started: cache_retry.request_already_started(),
            cached: cached.clone(),
            retry_attempt: retry.attempt,
        };
        true
    }
}
