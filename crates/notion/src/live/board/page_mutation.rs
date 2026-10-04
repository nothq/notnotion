mod annotations;
mod blocks;
mod clock;
mod commit;
mod plan;
mod selection_structure;
mod single;
mod text;
mod text_selection;
mod wire;

use super::{
    block_value, normalize_uuid, page::card_page_snapshot_from_response,
    page_state::PageMutationState, CompletePageResponse, FavoriteMutationContext, HashMap,
    LiveBoardMutator, Map, MutationContext, SystemTime, Value, UNIX_EPOCH,
};
use blocks::{editable_block, metadata_operation, MutationIdentity};
pub(super) use clock::CrdtClock;
use commit::{
    block_commit_expectations, record_value_mut, wait_for_committed_block_records,
    BlockCommitExpectation,
};
use plan::build_mutation;
use wire::{submit_page_transaction, RecordPointer, SaveOperation};

use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use crate::model::{PageMutationEffect, PageMutationRequest, PageMutationResult};

struct BuiltPageMutation {
    operations: Vec<SaveOperation>,
    effect: PageMutationEffect,
    user_action: &'static str,
}

struct PageOperationSubmission<'a> {
    page_block_id: &'a str,
    context: &'a FavoriteMutationContext,
    now: u64,
    user_action: &'static str,
    operations: Vec<SaveOperation>,
    effect: PageMutationEffect,
}

impl LiveBoardMutator {
    pub(in crate::live) fn register_page_snapshot(
        &mut self,
        state: PageMutationState,
        response: CompletePageResponse,
    ) {
        self.page_responses
            .insert(state.page_block_id.clone(), response);
        self.page_states.insert(state.page_block_id.clone(), state);
    }

    pub(crate) fn page_icon_upload_target(
        &self,
        page_block_id: &str,
        block_id: &str,
    ) -> Result<(String, String), String> {
        let page_block_id = normalize_uuid(page_block_id);
        let block_id = normalize_uuid(block_id);
        let context = self.page_context()?;
        let state = self.page_states.get(&page_block_id).ok_or_else(|| {
            format!("page {page_block_id} has not been loaded for a page-icon upload")
        })?;
        if state.space_id != context.space_id {
            return Err(format!(
                "page {page_block_id} is loaded from a different Notion space"
            ));
        }
        let block = editable_block(state, &block_id)?;
        if block.id == state.page_block_id
            || !matches!(
                &block.kind,
                crate::model::NotionPageBlockKind::Page
                    | crate::model::NotionPageBlockKind::LinkToPage
                    | crate::model::NotionPageBlockKind::Callout
            )
        {
            return Err(format!(
                "block {block_id} is not a mutable Notion icon block"
            ));
        }
        Ok((block.id.clone(), state.space_id.clone()))
    }

    pub(crate) fn cached_page_bootstrap_containing_block(
        &self,
        block_id: &str,
    ) -> Result<Value, String> {
        let block_id = normalize_uuid(block_id);
        let mut matched_page_id = None;
        for (page_id, state) in &self.page_states {
            if state.is_opaque_unavailable(&block_id) {
                return Err(format!(
                    "opaque unavailable block {block_id} cannot be opened directly"
                ));
            }
            if !state.contains_block(&block_id) {
                continue;
            }
            if let Some(previous_page_id) = matched_page_id {
                return Err(format!(
                    "Notion block {block_id} is loaded in multiple cached pages: {previous_page_id} and {page_id}"
                ));
            }
            matched_page_id = Some(page_id.as_str());
        }
        let page_id = matched_page_id.ok_or_else(|| {
            format!("Notion block {block_id} is not loaded in any cached page response")
        })?;
        let response = self.page_responses.get(page_id).ok_or_else(|| {
            format!("Notion page {page_id} does not have its required cached response")
        })?;
        let record_map = response
            .as_value()
            .get("recordMap")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("Notion page {page_id} does not have a cached recordMap"))?
            .clone();
        Ok(Value::Object(
            [("recordMap".to_string(), Value::Object(record_map))]
                .into_iter()
                .collect(),
        ))
    }

    pub(crate) fn apply_page_mutation(
        &mut self,
        session: &NotionDesktopSession,
        request: &PageMutationRequest,
    ) -> Result<PageMutationResult, NotionLiveError> {
        let context = self.page_context()?.clone();
        let now = mutation_now_ms()?;
        let mut clock = self.take_page_crdt_clock(&request.page_block_id);
        let built = (|| {
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
            build_mutation(state, &identity, &request.mutation, &mut clock)
        })();
        self.page_crdt_clocks
            .insert(request.page_block_id.clone(), clock);
        let built = built?;
        self.submit_and_synchronize_page_operations(
            session,
            PageOperationSubmission {
                page_block_id: &request.page_block_id,
                context: &context,
                now,
                user_action: built.user_action,
                operations: built.operations,
                effect: built.effect,
            },
        )
    }

    fn page_context(&self) -> Result<&FavoriteMutationContext, String> {
        match &self.context {
            MutationContext::Page(context) => Ok(context),
            MutationContext::Database(context) => Ok(&context.favorite),
        }
    }

    fn take_page_crdt_clock(&mut self, page_block_id: &str) -> CrdtClock {
        self.page_crdt_clocks
            .remove(page_block_id)
            .unwrap_or_else(CrdtClock::new)
    }

    fn submit_and_synchronize_page_operations(
        &mut self,
        session: &NotionDesktopSession,
        submission: PageOperationSubmission<'_>,
    ) -> Result<PageMutationResult, NotionLiveError> {
        let PageOperationSubmission {
            page_block_id,
            context,
            now,
            user_action,
            operations,
            effect,
        } = submission;
        if !operations.is_empty() {
            let state = self
                .page_states
                .get(page_block_id)
                .ok_or_else(|| format!("page {page_block_id} has not been loaded for mutation"))?;
            let expectations = block_commit_expectations(state, &operations);
            submit_page_transaction(session, context, now, user_action, operations)?;
            let committed_blocks =
                wait_for_committed_block_records(session, &context.space_id, &expectations)?;
            self.apply_committed_block_records(page_block_id, &expectations, &committed_blocks)?;
        }

        let snapshot = card_page_snapshot_from_response(
            page_block_id,
            self.page_responses
                .get(page_block_id)
                .ok_or_else(|| {
                    format!("page {page_block_id} does not have a cached Notion response")
                })?
                .clone(),
        )?;
        if effect
            .created_block_id()
            .is_some_and(|block_id| !snapshot.mutation_state.contains_block(block_id))
        {
            return Err(NotionLiveError::Fatal(
                "Notion committed records did not contain the newly created page block".to_string(),
            ));
        }
        self.page_responses
            .insert(page_block_id.to_string(), snapshot.response);
        self.page_states
            .insert(page_block_id.to_string(), snapshot.mutation_state);
        Ok(PageMutationResult {
            page: snapshot.page,
            effect,
        })
    }

    fn apply_committed_block_records(
        &mut self,
        page_block_id: &str,
        expectations: &HashMap<String, BlockCommitExpectation>,
        committed_blocks: &Map<String, Value>,
    ) -> Result<(), String> {
        let cached_blocks = self
            .page_responses
            .get_mut(page_block_id)
            .ok_or_else(|| format!("page {page_block_id} has no cached page response"))?
            .record_map_mut()?
            .get_mut("block")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("page {page_block_id} has no cached recordMap.block"))?;
        for (block_id, expectation) in expectations {
            let entry = committed_blocks
                .get(block_id)
                .ok_or_else(|| format!("Notion did not commit updated block {block_id}"))?;
            if expectation.overlay_record {
                cached_blocks.insert(block_id.clone(), entry.clone());
                continue;
            }
            let committed = block_value(committed_blocks, block_id)?;
            let cached = cached_blocks
                .get_mut(block_id)
                .and_then(record_value_mut)
                .ok_or_else(|| format!("page {page_block_id} has no cached block {block_id}"))?;
            for field in [
                "version",
                "last_edited_time",
                "last_edited_by_id",
                "last_edited_by_table",
            ] {
                if let Some(value) = committed.get(field) {
                    cached.insert(field.to_string(), value.clone());
                }
            }
        }
        Ok(())
    }
}

fn with_text_metadata(
    mut operations: Vec<SaveOperation>,
    identity: &MutationIdentity<'_>,
    block_id: &str,
) -> Vec<SaveOperation> {
    if operations.is_empty() {
        return operations;
    }
    operations.push(metadata_operation(identity, block_id));
    if block_id != identity.page_block_id {
        operations.push(metadata_operation(identity, identity.page_block_id));
    }
    operations
}

fn validated_new_block_id<'a>(
    state: &PageMutationState,
    block_id: &'a str,
) -> Result<&'a str, String> {
    if state.blocks.contains_key(block_id) {
        return Err(format!(
            "block {block_id} already exists in the loaded page"
        ));
    }
    let bytes = block_id.as_bytes();
    let canonical = bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        });
    if !canonical {
        return Err(format!(
            "new block id {block_id} is not a canonical lowercase UUID"
        ));
    }
    Ok(block_id)
}

fn mutation_now_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))
}
