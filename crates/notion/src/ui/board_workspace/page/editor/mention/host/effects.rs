use std::sync::Arc;

use gpui::{AppContext, Context};
use gpui_components::text_input::TextInput;

use super::super::super::editing::{PageEditSession, PageEditTransition};
use super::super::actions::{PageMentionEffect, PageMentionMutationEffect};
use super::super::callbacks::page_mention_action_sink;
use super::super::picker::{page_mention_picker_field_props, prepare_page_mention_picker_open};
use super::super::session::PageMentionEditFinish;
use super::super::state::PageMentionPickerIdentity;
use super::loads::{
    PageMentionPeopleJob, PageMentionRecentPagesJob, PageMentionSearchDelay, PageMentionSearchJob,
};
use crate::model::{NotionWorkspaceApi, SearchWorkspaceRequest, SearchWorkspaceScope};
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::{SurfaceState, Theme};

use super::super::data::{PageMentionPageSchedule, MENTION_PAGE_RESULT_LIMIT};

pub(super) enum PageMentionHostOutcome {
    Done,
    Edit(Box<PageMentionEditOutcome>),
}

pub(super) struct PageMentionEditOutcome {
    pub(super) transition: PageEditTransition<()>,
    pub(super) finish: PageMentionEditFinish,
}

pub(super) struct PageMentionEffectHost<'a> {
    pub(super) editor: &'a mut PageEditorState,
    pub(super) documents: &'a PageDocuments,
    pub(super) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(super) current_board_url: Option<String>,
    pub(super) theme: Theme,
}

impl PageMentionEffectHost<'_> {
    pub(super) fn execute(
        &mut self,
        effect: PageMentionEffect,
        cx: &mut Context<SurfaceState>,
    ) -> PageMentionHostOutcome {
        match effect {
            PageMentionEffect::EnsurePeople => self.ensure_people(cx),
            PageMentionEffect::SchedulePages(schedule) => self.schedule_pages(schedule, cx),
            PageMentionEffect::BeginPageSearch { generation, query } => {
                self.begin_page_search(generation, query, cx)
            }
            PageMentionEffect::OpenPickerAtAtom {
                block_id,
                atom_index,
                anchor,
            } => {
                self.open_picker(block_id, atom_index, anchor, cx);
                PageMentionHostOutcome::Done
            }
            PageMentionEffect::Mutation(effect) => self.execute_mutation(*effect),
        }
    }

    fn ensure_people(&mut self, cx: &mut Context<SurfaceState>) -> PageMentionHostOutcome {
        let Some(workspace_api) = self.workspace_api.clone() else {
            return PageMentionHostOutcome::Done;
        };
        if self.editor.mention.begin_people_load() {
            PageMentionPeopleJob { workspace_api }.spawn(cx);
        }
        PageMentionHostOutcome::Done
    }

    fn schedule_pages(
        &mut self,
        schedule: PageMentionPageSchedule,
        cx: &mut Context<SurfaceState>,
    ) -> PageMentionHostOutcome {
        match schedule {
            PageMentionPageSchedule::Recents { generation } => {
                self.schedule_recent_pages(generation, cx)
            }
            PageMentionPageSchedule::Search { generation, query } => {
                PageMentionSearchDelay { generation, query }.spawn(cx);
            }
        }
        PageMentionHostOutcome::Done
    }

    fn schedule_recent_pages(&mut self, generation: u64, cx: &mut Context<SurfaceState>) {
        let (Some(workspace_api), Some(current_board_url)) =
            (self.workspace_api.clone(), self.current_board_url.clone())
        else {
            return;
        };
        if self.editor.mention.begin_recent_pages_load(generation) {
            PageMentionRecentPagesJob {
                workspace_api,
                current_board_url,
                generation,
            }
            .spawn(cx);
        }
    }

    fn begin_page_search(
        &mut self,
        generation: u64,
        query: String,
        cx: &mut Context<SurfaceState>,
    ) -> PageMentionHostOutcome {
        let (Some(workspace_api), Some(current_board_url)) =
            (self.workspace_api.clone(), self.current_board_url.clone())
        else {
            return PageMentionHostOutcome::Done;
        };
        let Some(sequence) = self.editor.mention.begin_page_search(generation) else {
            return PageMentionHostOutcome::Done;
        };
        let request = SearchWorkspaceRequest {
            current_board_url,
            query: query.clone(),
            scope: SearchWorkspaceScope::TitleOnly,
            limit: MENTION_PAGE_RESULT_LIMIT,
            search_session_id: sequence.search_session_id,
            flow_number: sequence.flow_number,
            recent_pages_for_boosting: Vec::new(),
            excluded_block_ids: Vec::new(),
        };
        PageMentionSearchJob {
            workspace_api,
            generation,
            query,
            request,
        }
        .spawn(cx);
        PageMentionHostOutcome::Done
    }

    fn execute_mutation(&mut self, effect: PageMentionMutationEffect) -> PageMentionHostOutcome {
        let (transition, finish) = {
            let mut session = PageEditSession::new(self.editor, self.documents);
            let finish = session.apply_page_mention_mutation(effect);
            (session.finish(()), finish)
        };
        PageMentionHostOutcome::Edit(Box::new(PageMentionEditOutcome { transition, finish }))
    }

    fn open_picker(
        &mut self,
        block_id: String,
        atom_index: usize,
        anchor: gpui::Bounds<gpui::Pixels>,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(page) = self.documents.page_containing_block(&block_id) else {
            return;
        };
        let Some(picker) = prepare_page_mention_picker_open(&page, block_id, atom_index, anchor)
        else {
            return;
        };
        let props = page_mention_picker_field_props(
            &picker.field_text,
            self.theme,
            page_mention_action_sink(cx),
            PageMentionPickerIdentity::new(&picker),
        );
        let input = cx.new(|cx| TextInput::new(props, cx));
        input.update(cx, |input, cx| input.select_all(cx));
        self.editor.mention.open_picker(picker, input);
        cx.notify();
    }
}
