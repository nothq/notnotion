use serde_json::{json, Value};

use super::{reject_transaction_errors, FavoriteMutationContext, NotionPrivateApiEndpoint};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_in_space_with_session,
    NotionLiveError,
};

pub(super) fn submit_transactions(
    session: &NotionDesktopSession,
    context: &FavoriteMutationContext,
    request: TransactionSubmitRequest<'_>,
) -> Result<(), NotionLiveError> {
    let response = post_private_api_in_space_with_session(
        session,
        request.endpoint,
        &context.space_id,
        &json!({
            "requestId": uuid::Uuid::new_v4().to_string(),
            "transactions": [{
                "id": uuid::Uuid::new_v4().to_string(),
                "spaceId": context.space_id,
                "debug": {
                    "userAction": request.user_action,
                    "navParentClientGateState": "on",
                    "clientCommitTimeMs": request.client_commit_time_ms,
                },
                "operations": request.operations,
            }],
        }),
    )?;
    reject_transaction_errors(&response, "database").map_err(NotionLiveError::Fatal)
}

pub(super) struct TransactionSubmitRequest<'a> {
    pub(super) user_action: &'a str,
    pub(super) client_commit_time_ms: u64,
    pub(super) operations: Vec<Value>,
    pub(super) endpoint: NotionPrivateApiEndpoint,
}
