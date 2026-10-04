use std::{path::PathBuf, sync::Arc};

use gpui::{Context, Window};

use super::super::editing::PageEditTransition;
use super::super::persistence::PageMutationAction;
use super::{
    actions::{PageLinkIconActionContext, PageLinkIconActionOutcome, PageLinkIconControllerEffect},
    commit::PageLinkIconCommitCompletion,
    picker::{PageLinkCustomEmojiLibraryRequest, PageLinkIconToggleResult},
    preview::{PageLinkIconPreviewTarget, PageLinkIconUploadTarget},
    PageLinkIconAction, PageLinkIconTarget, PageLinkIconView,
};
use crate::model::{
    NotionCustomEmojiLibrary, NotionWorkspaceApi, NotionWorkspaceOperationFailure,
    PreparedPageIconFile,
};
use crate::ui::surface::{NotionSurfaceResources, PageDocuments, PageEditorState};
use crate::ui::view_actions::{ViewActionSink, ViewNotifier};
use crate::ui::{AppearanceMode, LoadedNotionExternalIcon, PageEditFocus, SurfaceState};

mod commit;
mod completion;
mod jobs;
mod persistence;
mod preview;

use jobs::{PageLinkIconCommitJob, PageLinkIconLibraryJob};
use persistence::PageLinkIconPreparedEdit;

pub(super) enum PageLinkIconCompletion {
    CustomEmojiLibrary {
        request: PageLinkCustomEmojiLibraryRequest,
        result: crate::model::NotionWorkspaceResult<NotionCustomEmojiLibrary>,
    },
    PromptedLocalFile {
        target: PageLinkIconUploadTarget,
        path: PathBuf,
    },
    LocalPreview {
        target: PageLinkIconPreviewTarget,
        result: Result<PreparedPageIconFile, String>,
    },
    ExternalPreview {
        target: PageLinkIconPreviewTarget,
        value: String,
        result: Result<LoadedNotionExternalIcon, String>,
    },
    Commit(PageLinkIconCommitCompletion),
}

enum PageLinkIconEvent {
    Action(PageLinkIconAction),
    Completion(Box<PageLinkIconCompletion>),
}

enum PageLinkIconRootEffect {
    DismissPageBlockInteraction,
    ApplyEdit(Box<PageLinkIconPreparedEdit>),
    PageMutation(Box<PageMutationAction>),
    SpawnCommit(Box<PageLinkIconCommitJob>),
    FinishSearchMutation { page_id: String },
    WorkspaceFailure(Box<PageLinkIconWorkspaceFailure>),
    Notify,
}

pub(super) struct PageLinkIconWorkspaceFailure {
    pub(super) operation: &'static str,
    pub(super) error: NotionWorkspaceOperationFailure,
    pub(super) recovery: Option<PageLinkIconFailureRecovery>,
    pub(super) notify_after_recovery: bool,
}

pub(super) enum PageLinkIconFailureRecovery {
    FinishFailedCommit { picker_matches: bool },
}

pub(super) struct PageLinkIconEffectHost<'a> {
    pub(super) editor: &'a mut PageEditorState,
    pub(super) documents: &'a PageDocuments,
    pub(super) resources: NotionSurfaceResources,
    pub(super) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(super) cached_workspace_visible: bool,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) focus: Option<PageEditFocus>,
}

impl PageLinkIconEffectHost<'_> {
    fn resolve_action(
        &mut self,
        action: PageLinkIconAction,
        page_mutation_idle: bool,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        let context = PageLinkIconActionContext {
            appearance_mode: self.appearance_mode,
            page_mutation_idle,
        };
        let PageLinkIconActionOutcome { effect, notify } = self
            .editor
            .page_link_icons
            .handle_action(action, context, window, cx);
        let mut effects = effect.map_or_else(Vec::new, |effect| {
            self.resolve_controller_effect(effect, window, cx)
        });
        if notify {
            effects.push(PageLinkIconRootEffect::Notify);
        }
        effects
    }

    fn resolve_controller_effect(
        &mut self,
        effect: PageLinkIconControllerEffect,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        match effect {
            PageLinkIconControllerEffect::DismissPageBlockInteraction => {
                vec![PageLinkIconRootEffect::DismissPageBlockInteraction]
            }
            PageLinkIconControllerEffect::SetIcon(effect) => self.prepare_set_icon(
                effect,
                super::persistence::PageLinkIconPersistence::Queue,
                cx,
            ),
            PageLinkIconControllerEffect::PasteUpload(target) => self.paste_upload(target, cx),
            PageLinkIconControllerEffect::PromptUpload(target) => {
                self.prompt_upload(target, window, cx);
                Vec::new()
            }
            PageLinkIconControllerEffect::SaveUpload(save) => self.save_upload(save, cx),
        }
    }
}

impl SurfaceState {
    pub(in crate::ui::board_workspace::page::editor) fn toggle_page_link_icon_picker(
        &mut self,
        page_id: String,
        block_id: String,
        cx: &mut Context<Self>,
    ) {
        let actions = page_link_icon_action_sink(cx);
        let result = self.page_editor.page_link_icons.toggle_picker(
            PageLinkIconTarget { page_id, block_id },
            self.theme,
            actions,
            cx,
        );
        if matches!(result, PageLinkIconToggleResult::Opened) {
            self.page_editor.page_block_context_menu = None;
            if let Some(workspace_api) = self.notion_startup.workspace_api() {
                if let Some(request) = self
                    .page_editor
                    .page_link_icons
                    .begin_custom_emoji_library_load()
                {
                    PageLinkIconLibraryJob {
                        workspace_api,
                        request,
                    }
                    .spawn(cx);
                }
            }
        }
        if !matches!(result, PageLinkIconToggleResult::Ignored) {
            cx.notify();
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn page_link_icon_presentation(
        &self,
        cx: &Context<Self>,
    ) -> Option<super::PageLinkIconPresentation> {
        let picker = self.page_editor.page_link_icons.picker()?;
        let state = self
            .page_editor
            .page_link_icons
            .picker_state(&picker.page_id, &picker.block_id)?;
        Some(super::PageLinkIconPresentation {
            view: PageLinkIconView {
                theme: self.theme,
                appearance_mode: self.appearance_mode,
                icons: Arc::clone(&self.icons),
                resources: self.notion_resources.clone(),
                notifier: ViewNotifier::new(cx),
                actions: page_link_icon_action_sink(cx),
                page_mutation_idle: self.page_mutations.is_idle(&state.page_id),
            },
            state,
        })
    }

    fn handle_page_link_icon_event(
        &mut self,
        event: PageLinkIconEvent,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let page_mutation_idle = self
            .page_editor
            .page_link_icons
            .picker()
            .is_some_and(|picker| self.page_mutations.is_idle(&picker.page_id));
        let workspace_api = self.notion_startup.workspace_api();
        let focus = self.page_editor.current_page_edit_focus(cx);
        let mut host = PageLinkIconEffectHost {
            editor: &mut self.page_editor,
            documents: &self.page_documents,
            resources: self.notion_resources.clone(),
            workspace_api,
            cached_workspace_visible: self.notion_startup.cached_workspace_visible(),
            appearance_mode: self.appearance_mode,
            focus,
        };
        let effects = match event {
            PageLinkIconEvent::Action(action) => host.resolve_action(
                action,
                page_mutation_idle,
                window.expect("page-link icon actions require their source window"),
                cx,
            ),
            PageLinkIconEvent::Completion(completion) => host.resolve_completion(*completion, cx),
        };
        for effect in effects {
            self.apply_page_link_icon_root_effect(effect, cx);
        }
    }

    fn apply_page_link_icon_root_effect(
        &mut self,
        effect: PageLinkIconRootEffect,
        cx: &mut Context<Self>,
    ) {
        match effect {
            PageLinkIconRootEffect::DismissPageBlockInteraction => {
                self.page_editor.dismiss_page_block_interaction(cx);
            }
            PageLinkIconRootEffect::ApplyEdit(edit) => {
                let edit = *edit;
                self.apply_page_edit_transition(edit.transition, cx);
                if let Some(mutation) = edit.mutation {
                    self.dispatch_page_mutation_action(mutation, cx);
                }
                edit.finish.apply(&mut self.page_editor.page_link_icons, cx);
            }
            PageLinkIconRootEffect::PageMutation(action) => {
                self.dispatch_page_mutation_action(*action, cx);
            }
            PageLinkIconRootEffect::SpawnCommit(job) => (*job).spawn(cx),
            PageLinkIconRootEffect::FinishSearchMutation { page_id } => {
                self.dispatch_page_mutation_action(
                    PageMutationAction::FinishSearchMutation {
                        lane_page_id: page_id,
                    },
                    cx,
                );
            }
            PageLinkIconRootEffect::WorkspaceFailure(failure) => {
                let failure = *failure;
                let reauth =
                    self.handle_notion_workspace_failure(failure.operation, failure.error, cx);
                if !reauth {
                    if let Some(recovery) = failure.recovery {
                        recovery.apply(&mut self.page_editor.page_link_icons);
                    }
                    if failure.notify_after_recovery {
                        cx.notify();
                    }
                }
            }
            PageLinkIconRootEffect::Notify => cx.notify(),
        }
    }
}

fn page_link_icon_action_sink(cx: &Context<SurfaceState>) -> ViewActionSink<PageLinkIconAction> {
    ViewActionSink::new(cx, |surface, action, window, cx| {
        surface.handle_page_link_icon_event(PageLinkIconEvent::Action(action), Some(window), cx);
    })
}
