use crate::ui::board_workspace::PageDocumentAction;
use gpui::{App, AppContext, Context};
use gpui_components::text_input::{TextInput, TextInputProps, TextInputStyle};

use super::super::document_edit::{
    PageTextPreMutationAction, PageTextPreMutationActionSink, PageTextPreMutationRequest,
    PageTextPreMutationResponse,
};
use super::super::mention::PageMentionAction;
use super::super::rich_text::{
    page_rich_text_link_on_change, PageRichTextDialog, PageRichTextInputAction,
};
use super::{
    PageCodeSettingsStage, PageEditEffect, PageEditHostEffect, PageEditTransition,
    PageEditWriteEffect, PageEditorEffect,
};
use crate::ui::board_workspace::{PageFocusSession, PageMutationAction, PageMutationPlan};
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::SurfaceState;

pub(in super::super) fn page_text_pre_mutation_action_sink(
    cx: &Context<SurfaceState>,
) -> PageTextPreMutationActionSink {
    let surface = cx.entity().downgrade();
    PageTextPreMutationActionSink::new(move |action, cx| {
        surface
            .update(cx, |surface, cx| {
                surface.handle_page_text_pre_mutation_action(action, cx);
            })
            .ok();
    })
}

impl SurfaceState {
    fn handle_page_text_pre_mutation_action(
        &mut self,
        action: PageTextPreMutationAction,
        cx: &mut Context<Self>,
    ) {
        match action {
            PageTextPreMutationAction::Prepare(request) => {
                let PageTextPreMutationRequest {
                    block_id,
                    action,
                    response,
                } = request;
                let decision = if self
                    .page_editor
                    .capture_page_rich_text_pre_mutation(&block_id, &action)
                {
                    PageTextPreMutationResponse::Continue
                } else {
                    let transition = {
                        let mut session = super::PageEditSession::new(
                            &mut self.page_editor,
                            &self.page_documents,
                        );
                        let decision = session.prepare_page_text_pre_mutation(&block_id, action);
                        session.finish(decision)
                    };
                    self.apply_page_edit_transition(transition, cx)
                };
                *response.borrow_mut() = Some(decision);
            }
            PageTextPreMutationAction::Apply(prepared) => {
                let transition = {
                    let mut session =
                        super::PageEditSession::new(&mut self.page_editor, &self.page_documents);
                    (*prepared).apply(&mut session);
                    session.finish(())
                };
                self.apply_page_edit_transition(transition, cx);
            }
        }
    }

    pub(crate) fn apply_page_edit_transition<T>(
        &mut self,
        transition: PageEditTransition<T>,
        cx: &mut Context<Self>,
    ) -> T {
        let (result, effects) = transition.into_parts();
        for effect in effects {
            match effect {
                PageEditEffect::Write(effect) => self.dispatch_page_mutation_action(
                    PageMutationAction::ApplyPlan(effect.into_plan(&self.page_documents)),
                    cx,
                ),
                PageEditEffect::Host(effect) => apply_page_edit_host_effect(self, effect, cx),
                PageEditEffect::Editor(effect) => {
                    effect.apply(&mut self.page_editor, &self.page_documents, cx)
                }
                PageEditEffect::WriteClipboard(item) => cx.write_to_clipboard(item),
                PageEditEffect::ReplaceLoadedPage(page) => {
                    self.dispatch_page_document_action(PageDocumentAction::replace_loaded(page), cx)
                }
                PageEditEffect::FocusBlock { block_id, offset } => {
                    PageFocusSession::new(&mut self.page_editor, &mut self.page_documents)
                        .focus_block(block_id, offset)
                }
                PageEditEffect::ReconcileTextMenus {
                    block_id,
                    snapshot,
                    change_requires_notify,
                } => {
                    let transition =
                        super::PageEditSession::new(&mut self.page_editor, &self.page_documents)
                            .reconcile_slash_menu(&block_id, &snapshot);
                    let slash_changed = self.apply_page_edit_transition(transition, cx);
                    let action =
                        PageMentionAction::reconcile(&self.page_documents, &block_id, &snapshot);
                    let mention_changed = self.dispatch_page_mention_action(action, cx).changed;
                    if change_requires_notify || slash_changed || mention_changed {
                        cx.notify();
                    }
                }
                PageEditEffect::UpdateSlashMenu { block_id, snapshot } => {
                    let transition =
                        super::PageEditSession::new(&mut self.page_editor, &self.page_documents)
                            .reconcile_slash_menu(&block_id, &snapshot);
                    self.apply_page_edit_transition(transition, cx);
                }
                PageEditEffect::OpenRichTextLinkInput => self.open_page_rich_text_link_input(cx),
                PageEditEffect::Error(error) => self.print_notion_error(error),
                PageEditEffect::Notify => cx.notify(),
            }
        }
        result
    }

    fn open_page_rich_text_link_input(&mut self, cx: &mut Context<Self>) {
        self.page_editor.page_rich_text_link_value.clear();
        let actions = ViewActionSink::new(cx, |surface, action, _window, cx| match action {
            PageRichTextInputAction::LinkChanged(value) => {
                surface.page_editor.page_rich_text_link_value = value;
                cx.notify();
            }
        });
        let props = TextInputProps::single_line("")
            .placeholder("Paste link")
            .request_focus(true)
            .style(TextInputStyle {
                background: gpui::Hsla::from(gpui::rgb(self.theme.dialog_input_bg.hex))
                    .opacity(self.theme.dialog_input_bg.opacity),
                text: gpui::rgb(self.theme.text_primary).into(),
                placeholder: gpui::rgb(self.theme.text_hint).into(),
                ..TextInputStyle::default()
            })
            .on_change(page_rich_text_link_on_change(actions));
        *self.page_editor.page_rich_text_link_input.borrow_mut() =
            Some(cx.new(|cx| TextInput::new(props, cx)));
        self.page_editor.page_rich_text_dialog = Some(PageRichTextDialog::Link);
        cx.notify();
    }
}

impl PageEditWriteEffect {
    fn into_plan(self, documents: &PageDocuments) -> PageMutationPlan {
        match self {
            Self::ApplyMutationPlan(plan) => plan,
            Self::EnqueueMutation { page_id, mutation } => {
                PageMutationPlan::mutation(page_id, mutation)
            }
            Self::EnqueueMutationWithProjection {
                page_id,
                mutation,
                projection,
            } => PageMutationPlan::projected_mutation(page_id, mutation, projection),
            Self::EnqueueRichTextEdit(request) => PageMutationPlan::rich_text(request),
            Self::EnqueueCompletedCrossBlockComposition {
                page_id,
                survivor_id,
                removed_ids,
                mutation,
            } => {
                let projection = documents
                    .loaded_page_text_projection(&page_id, &[survivor_id])
                    .with_retired_blocks(removed_ids);
                PageMutationPlan::projected_mutation(page_id, mutation, projection)
            }
        }
    }
}

fn apply_page_edit_host_effect(
    surface: &mut SurfaceState,
    effect: PageEditHostEffect,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        PageEditHostEffect::MarkPageHasContent(page_id) => surface
            .board
            .mark_page_has_content(&page_id, &mut surface.columns),
        PageEditHostEffect::OpenPageMention {
            block_id,
            trigger_offset,
            query,
        } => {
            surface.dispatch_page_mention_action(
                PageMentionAction::OpenMenu(crate::ui::PageMentionMenuState {
                    block_id,
                    trigger_offset,
                    query,
                    selected_index: 0,
                }),
                cx,
            );
        }
        PageEditHostEffect::ReplaceTextBlockWithCode {
            page,
            block_index,
            source_text,
        } => {
            let settings = surface.page_code_settings.current();
            let transition = {
                let mut session =
                    super::PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                session.replace_verified_page_text_block_with_code(
                    page,
                    block_index,
                    source_text,
                    settings,
                );
                session.finish(())
            };
            surface.apply_page_edit_transition(transition, cx);
        }
        PageEditHostEffect::StageCodeSettings(stage) => match stage {
            PageCodeSettingsStage::Language(language) => {
                surface.page_code_settings.stage_language(language);
            }
            PageCodeSettingsStage::Wrap(wrap) => surface.page_code_settings.stage_wrap(wrap),
        },
        PageEditHostEffect::RestorePageHistoryState(state) => {
            let transition =
                PageFocusSession::new(&mut surface.page_editor, &mut surface.page_documents)
                    .restore_history(state, cx);
            surface.apply_page_edit_transition(transition, cx);
        }
    }
}

impl PageEditorEffect {
    fn apply(self, editor: &mut PageEditorState, documents: &PageDocuments, cx: &mut App) {
        match self {
            Self::CompleteComposer(completion) => completion.apply(editor),
            Self::SimpleTable(effect) => effect.apply(editor),
            Self::FinishCrossBlockReplacement { removed_ids } => {
                editor.finish_cross_block_page_text_replacement(&removed_ids);
            }
            Self::BeginCrossBlockComposition {
                removed_ids,
                pending,
                survivor_id,
                marked_range,
            } => editor.begin_cross_block_page_composition(
                &removed_ids,
                *pending,
                survivor_id,
                marked_range,
            ),
            Self::FinishMultilinePaste => editor.finish_multiline_page_text_paste(),
            Self::FinishMarkdownDivider { block_id } => {
                editor.finish_page_markdown_divider(&block_id);
            }
            Self::FinishEditableMarkdown { block_id } => {
                editor.finish_editable_page_markdown(&block_id);
            }
            Self::SynchronizeTextInputSelections { excluded_block_id } => editor
                .synchronize_page_text_input_selections(
                    documents,
                    excluded_block_id.as_deref(),
                    cx,
                ),
            Self::CloseRichTextDialog => editor.close_page_rich_text_dialog(),
            Self::DismissRichTextColorDialog => editor.page_rich_text_dialog = None,
            Self::ClearBlockContextMenu => editor.page_block_context_menu = None,
            Self::ClearBlockContextMenuAndSelection => {
                editor.clear_page_block_context_menu_selection()
            }
            Self::FinishTextBlockCodeReplacement { source_block_id } => {
                editor.finish_page_text_block_code_replacement(&source_block_id);
            }
            Self::ClearPageHistoryFocus => editor.clear_page_history_focus(),
            Self::ClearRemovedBlockEditorState { removed_ids } => {
                editor.clear_removed_page_block_editor_state(&removed_ids);
            }
            Self::RemoveBlockInput { block_id } => editor.remove_page_block_input(&block_id),
            Self::DeferInputSelection {
                input,
                range,
                reversed,
            } => cx.defer(move |cx| {
                input.update(cx, |input, cx| {
                    input.set_selection(range, reversed, cx);
                });
            }),
        }
    }
}

impl PageEditorState {
    fn remove_page_block_input(&mut self, block_id: &str) {
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .remove(block_id);
    }
}
