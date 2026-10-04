use std::collections::HashSet;

use super::super::{page::card_page_preview_from_record_map, CardPage, NotionPrivateApiEndpoint};
use super::preview::{preview_record_closure, preview_record_key, preview_requests};
use super::{
    InitialSyncRecordRequest, InitialSyncRequest, InitialSyncResponse, InitialSyncSpacePointer,
    QuickFindRecords, SharedRecordMap, QUICK_FIND_PREVIEW_MAX_WAVES,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_in_space_with_session,
    NotionLiveError,
};

impl QuickFindRecords {
    pub(in super::super) fn load_preview(
        &self,
        session: &NotionDesktopSession,
        block_id: &str,
    ) -> Result<CardPage, NotionLiveError> {
        let generation = self.preview_root_generation(block_id)?;
        let mut record_map = self.preview_root_snapshot(session, block_id, generation)?;
        let mut requested = HashSet::new();
        for wave in 0..=QUICK_FIND_PREVIEW_MAX_WAVES {
            let preview_records = preview_record_closure(&record_map, &self.space_id, block_id)?;
            let requests = preview_requests(&record_map, &preview_records, &requested);
            if requests.is_empty() {
                if !self.preview_root_is_hydrated(block_id)? {
                    return Err(NotionLiveError::Fatal(format!(
                        "Notion Quick Find preview root {block_id} changed while loading"
                    )));
                }
                self.touch_preview_records(block_id, &preview_records)?;
                return card_page_preview_from_record_map(block_id, &record_map)
                    .map_err(NotionLiveError::Fatal);
            }
            if wave == QUICK_FIND_PREVIEW_MAX_WAVES {
                break;
            }
            self.touch_preview_records(block_id, &[])?;
            requested.extend(
                requests
                    .iter()
                    .map(|request| preview_record_key(&request.pointer)),
            );
            drop(record_map);
            let response =
                request_initial_records(session, initial_sync_request(&self.space_id, requests))?;
            let _ =
                self.merge_preview_response(response.record_map, block_id, generation, false)?;
            record_map = self.record_map_snapshot()?;
        }
        Err(NotionLiveError::Fatal(format!(
            "Notion Quick Find preview {block_id} exceeded {QUICK_FIND_PREVIEW_MAX_WAVES} record waves"
        )))
    }

    fn preview_root_snapshot(
        &self,
        session: &NotionDesktopSession,
        block_id: &str,
        generation: u64,
    ) -> Result<SharedRecordMap, NotionLiveError> {
        if self.preview_root_is_hydrated(block_id)? {
            return self.record_map_snapshot().map_err(NotionLiveError::Fatal);
        }
        let response = request_initial_records(
            session,
            initial_sync_request(
                &self.space_id,
                vec![block_request(&self.space_id, block_id, -1)],
            ),
        )?;
        if !self.merge_preview_response(response.record_map, block_id, generation, true)? {
            return Err(NotionLiveError::Fatal(format!(
                "Notion Quick Find preview root {block_id} changed while hydrating"
            )));
        }
        self.record_map_snapshot().map_err(NotionLiveError::Fatal)
    }
}

pub(super) fn initial_sync_request(
    space_id: &str,
    requests: Vec<InitialSyncRecordRequest>,
) -> InitialSyncRequest {
    InitialSyncRequest {
        requests,
        space_pointer: InitialSyncSpacePointer {
            table: "space",
            id: space_id.to_string(),
        },
    }
}

pub(super) fn block_request(
    space_id: &str,
    block_id: &str,
    version: i64,
) -> InitialSyncRecordRequest {
    InitialSyncRecordRequest {
        pointer: super::preview::record_pointer("block", block_id, space_id),
        version,
    }
}

fn request_initial_records(
    session: &NotionDesktopSession,
    request: InitialSyncRequest,
) -> Result<InitialSyncResponse, NotionLiveError> {
    post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpaceInitial,
        &request.space_pointer.id,
        &request,
    )
}
