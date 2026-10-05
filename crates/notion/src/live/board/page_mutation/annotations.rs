use super::{
    blocks::{editable_block, metadata_operation, MutationIdentity},
    text::{plan_selection_text_edit, plan_typing_text_edit},
    wire::RecordPointer,
    with_text_metadata, CrdtClock, PageOperationSubmission,
};
use crate::{
    live::{
        board::{page_state::PageMutationState, LiveBoardMutator},
        credentials::NotionDesktopSession,
        NotionLiveError,
    },
    model::{
        EditPageBlockTextRequest, PageMutationEffect, PageMutationResult, PageTextAnnotation,
        PageTextAnnotationKind, PageTextEditTarget,
    },
};
use std::collections::HashSet;

impl LiveBoardMutator {
    pub(crate) fn apply_page_text_edit(
        &mut self,
        session: &NotionDesktopSession,
        request: &EditPageBlockTextRequest,
    ) -> Result<PageMutationResult, NotionLiveError> {
        let context = self.page_context()?.clone();
        let now = super::mutation_now_ms()?;
        let mut clock = self.take_page_crdt_clock(&request.page_block_id);
        let operations = (|| {
            let state = self
                .page_states
                .get(&request.page_block_id)
                .ok_or_else(|| {
                    format!(
                        "page {} has not been loaded for mutation",
                        request.page_block_id
                    )
                })?;
            if state.space_id != context.space_id {
                return Err(format!(
                    "page {} is loaded from a different Notion space",
                    request.page_block_id
                ));
            }
            let identity = MutationIdentity {
                space_id: &context.space_id,
                user_id: &context.user_id,
                page_block_id: &state.page_block_id,
                now,
            };
            let targets = targets_in_document_order(state, request.targets())?;
            Ok(match targets[0] {
                PageTextEditTarget::Selection { .. } => (
                    build_selection_operations(state, &identity, request, &targets, &mut clock)?,
                    selected_user_action(request),
                ),
                PageTextEditTarget::Typing { .. } => (
                    build_typing_operations(state, &identity, request, targets[0], &mut clock)?,
                    "Text.handleMutation",
                ),
            })
        })();
        self.page_crdt_clocks
            .insert(request.page_block_id.clone(), clock);
        let (operations, user_action) = operations?;
        self.submit_and_synchronize_page_operations(
            session,
            PageOperationSubmission {
                page_block_id: &request.page_block_id,
                context: &context,
                now,
                user_action,
                operations,
                effect: PageMutationEffect::NoAdditionalContext,
            },
        )
    }
}

fn build_selection_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &EditPageBlockTextRequest,
    targets: &[&PageTextEditTarget],
    clock: &mut CrdtClock,
) -> Result<Vec<super::wire::SaveOperation>, String> {
    let mut operations = Vec::new();
    let mut edited_block_ids = Vec::with_capacity(targets.len());
    for target in targets {
        let block = editable_block(state, target.block_id())?;
        operations.extend(plan_selection_text_edit(
            RecordPointer::block(&block.id, identity.space_id),
            block.title()?,
            target,
            request,
            clock,
        )?);
        edited_block_ids.push(block.id.as_str());
    }
    operations.extend(
        edited_block_ids
            .iter()
            .map(|block_id| metadata_operation(identity, block_id)),
    );
    if !edited_block_ids.contains(&identity.page_block_id) {
        operations.push(metadata_operation(identity, identity.page_block_id));
    }
    Ok(operations)
}

fn build_typing_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &EditPageBlockTextRequest,
    target: &PageTextEditTarget,
    clock: &mut CrdtClock,
) -> Result<Vec<super::wire::SaveOperation>, String> {
    let block = editable_block(state, target.block_id())?;
    let operations = plan_typing_text_edit(
        RecordPointer::block(&block.id, identity.space_id),
        block.title()?,
        target,
        request,
        clock,
    )?;
    Ok(with_text_metadata(operations, identity, &block.id))
}

fn selected_user_action(request: &EditPageBlockTextRequest) -> &'static str {
    if matches!(
        request.annotation_additions(),
        [PageTextAnnotation::TextColor(
            crate::model::PageTextColor::Default
        )]
    ) && request
        .annotation_removals()
        .contains(&PageTextAnnotationKind::TextColor)
        && request
            .annotation_removals()
            .contains(&PageTextAnnotationKind::BackgroundColor)
    {
        "clearTextFormatFromSelection"
    } else if matches!(
        request.annotation_additions(),
        [PageTextAnnotation::Link(_)]
    ) {
        "LinkMenu.handleLinkSelection"
    } else if matches!(
        request.annotation_removals(),
        [PageTextAnnotationKind::Link]
    ) {
        "linkAnnotationActions.unlinkSelection"
    } else {
        "textAnnotationActions.annotateSelection"
    }
}

fn targets_in_document_order<'a>(
    state: &PageMutationState,
    targets: &'a [PageTextEditTarget],
) -> Result<Vec<&'a PageTextEditTarget>, String> {
    let mut block_ids = Vec::new();
    append_document_block_ids(
        state,
        &state.page_block_id,
        &mut HashSet::new(),
        &mut block_ids,
    )?;
    let ordered = block_ids
        .iter()
        .filter_map(|block_id| {
            targets
                .iter()
                .find(|target| target.block_id() == block_id.as_str())
        })
        .collect::<Vec<_>>();
    if ordered.len() != targets.len() {
        if let Some(missing) = targets.iter().find(|target| {
            !block_ids
                .iter()
                .any(|block_id| block_id == target.block_id())
        }) {
            return Err(format!(
                "block {} is not editable page content in {}",
                missing.block_id(),
                state.page_block_id
            ));
        }
        return Err("page-text targets do not have a unique document order".to_string());
    }
    Ok(ordered)
}

fn append_document_block_ids(
    state: &PageMutationState,
    parent_id: &str,
    visited: &mut HashSet<String>,
    block_ids: &mut Vec<String>,
) -> Result<(), String> {
    let parent = state.block(parent_id)?;
    for child_id in &parent.content_ids {
        if !visited.insert(child_id.clone()) {
            return Err(format!(
                "page {} contains repeated content block {child_id}",
                state.page_block_id
            ));
        }
        if state.is_opaque_unavailable(child_id) {
            continue;
        }
        block_ids.push(child_id.clone());
        append_document_block_ids(state, child_id, visited, block_ids)?;
    }
    Ok(())
}
