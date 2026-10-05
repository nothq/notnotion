use crate::ui::search::action::{
    QuickFindEffect, QuickFindFailure, QuickFindFailureRecovery, QuickFindPreviewCompletion,
    QuickFindPreviewJob, QuickFindPreviewRequest, QuickFindRequest, QuickFindTimer,
};
use crate::ui::surface::{PageMutationCoordinator, PageMutationRunToken};
use crate::ui::{LoadedCardPage, NotionSearchPreviewState};

use super::NotionSearchState;

impl NotionSearchState {
    pub(super) fn preview_after_settle(&mut self) -> Vec<QuickFindEffect> {
        let Some(block_id) = self.selected_block_id() else {
            self.preview = NotionSearchPreviewState::Idle;
            return Vec::new();
        };
        if self.preview_is_cached(&block_id) || self.pending_preview_request(&block_id).is_some() {
            return self.load_selected_quick_find_preview();
        }
        vec![QuickFindEffect::Schedule(
            QuickFindTimer::PreviewSelection { block_id },
        )]
    }

    pub(super) fn load_selected_quick_find_preview(&mut self) -> Vec<QuickFindEffect> {
        let Some(block_id) = self.selected_block_id() else {
            self.preview = NotionSearchPreviewState::Idle;
            return Vec::new();
        };
        if self.page_mutation_is_dirty(&block_id) {
            self.preview = NotionSearchPreviewState::Idle;
            return Vec::new();
        }
        if preview_matches_block(&self.preview, &block_id) {
            return Vec::new();
        }
        if let Some(page) = self.cached_preview(&block_id) {
            reset_preview_list(&page);
            self.preview = NotionSearchPreviewState::Loaded {
                block_id: block_id.clone().into(),
                page,
            };
            return self.preview_revalidation_effect(block_id);
        }
        if let Some(request_token) = self.pending_preview_request(&block_id) {
            self.preview = NotionSearchPreviewState::Loading {
                block_id: block_id.into(),
                request_token,
            };
            return Vec::new();
        }
        vec![preview_request(block_id, true)]
    }

    pub(crate) fn begin_quick_find_preview_job(
        &mut self,
        request: QuickFindPreviewRequest,
        authority: Option<PageMutationRunToken>,
    ) -> Option<QuickFindPreviewJob> {
        if self.page_mutation_is_dirty(&request.block_id)
            || self.pending_preview_request(&request.block_id).is_some()
        {
            return None;
        }
        let request_token = self.begin_preview_request(request.block_id.clone());
        if request.show_loading {
            self.preview = NotionSearchPreviewState::Loading {
                block_id: request.block_id.clone().into(),
                request_token,
            };
        }
        Some(QuickFindPreviewJob {
            block_id: request.block_id,
            request_token,
            authority,
            show_loading: request.show_loading,
        })
    }

    pub(super) fn finish_quick_find_preview_timer(
        &mut self,
        timer: QuickFindTimer,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        let (block_id, revalidate) = match timer {
            QuickFindTimer::PreviewSelection { block_id } => (block_id, false),
            QuickFindTimer::PreviewRevalidation { block_id } => (block_id, true),
            QuickFindTimer::Query { .. } => return Vec::new(),
        };
        if !search_open || self.selected_block_id().as_deref() != Some(block_id.as_str()) {
            return Vec::new();
        }
        if revalidate {
            vec![preview_request(block_id, false)]
        } else {
            self.load_selected_quick_find_preview()
        }
    }

    pub(super) fn complete_quick_find_preview(
        &mut self,
        completion: QuickFindPreviewCompletion,
        page_mutations: &mut PageMutationCoordinator,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        let job = completion.job;
        if !self.finish_preview_request(&job.block_id, job.request_token)
            || self.page_mutation_is_dirty(&job.block_id)
        {
            return Vec::new();
        }
        let visible = self.preview_request_is_visible(&job, search_open);
        match completion.result {
            Ok(mut page) => {
                let authority = page_mutations.reconcile_external_load(&mut page, job.authority);
                let mut loaded = LoadedCardPage::with_authority(page, authority, None);
                initialize_preview_list(&mut loaded);
                self.cache_preview(job.block_id.clone(), loaded.clone());
                if !visible {
                    return Vec::new();
                }
                self.preview = NotionSearchPreviewState::Loaded {
                    block_id: job.block_id.into(),
                    page: loaded,
                };
                vec![QuickFindEffect::Notify]
            }
            Err(error) if job.show_loading && visible => {
                vec![QuickFindEffect::Failure(Box::new(QuickFindFailure {
                    operation: "search preview failed",
                    error,
                    recovery: QuickFindFailureRecovery::Preview {
                        block_id: job.block_id,
                    },
                }))]
            }
            Err(_) => Vec::new(),
        }
    }

    pub(super) fn fail_quick_find_preview(
        &mut self,
        block_id: String,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        if !search_open || self.selected_block_id().as_deref() != Some(block_id.as_str()) {
            return Vec::new();
        }
        self.preview = NotionSearchPreviewState::Failed {
            block_id: block_id.into(),
        };
        vec![QuickFindEffect::Notify]
    }

    fn selected_block_id(&self) -> Option<String> {
        self.visible_results()
            .get(self.selected_index)
            .map(|result| result.block_id.clone())
    }

    fn preview_revalidation_effect(&self, block_id: String) -> Vec<QuickFindEffect> {
        if self.pending_preview_request(&block_id).is_some() {
            return Vec::new();
        }
        vec![QuickFindEffect::Schedule(
            QuickFindTimer::PreviewRevalidation { block_id },
        )]
    }

    fn preview_request_is_visible(&self, job: &QuickFindPreviewJob, search_open: bool) -> bool {
        search_open
            && self.selected_block_id().as_deref() == Some(job.block_id.as_str())
            && (!job.show_loading
                || matches!(
                    &self.preview,
                    NotionSearchPreviewState::Loading {
                        block_id,
                        request_token,
                    } if block_id.as_ref() == job.block_id.as_str()
                        && *request_token == job.request_token
                ))
    }
}

fn preview_matches_block(preview: &NotionSearchPreviewState, block_id: &str) -> bool {
    matches!(
        preview,
        NotionSearchPreviewState::Loading { block_id: pending, .. }
            | NotionSearchPreviewState::Loaded { block_id: pending, .. }
            if pending.as_ref() == block_id
    )
}

fn preview_request(block_id: String, show_loading: bool) -> QuickFindEffect {
    QuickFindEffect::Request(QuickFindRequest::Preview(QuickFindPreviewRequest {
        block_id,
        show_loading,
    }))
}

fn initialize_preview_list(page: &mut LoadedCardPage) {
    let item_count = page
        .data
        .visible_rows
        .len()
        .checked_add(1)
        .expect("Notion Quick Find preview row count overflowed");
    page.list_state = gpui::ListState::new(item_count, gpui::ListAlignment::Top, gpui::px(128.0));
}

fn reset_preview_list(page: &LoadedCardPage) {
    let item_count = page
        .data
        .visible_rows
        .len()
        .checked_add(1)
        .expect("Notion Quick Find preview row count overflowed");
    page.list_state.reset(item_count);
    page.list_state.scroll_to(gpui::ListOffset::default());
}
