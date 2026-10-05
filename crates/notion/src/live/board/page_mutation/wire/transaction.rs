use super::{SaveOperation, SaveTransaction, SaveTransactionsRequest, TransactionDebug};
use crate::live::board::{
    reject_transaction_errors, FavoriteMutationContext, NotionPrivateApiEndpoint,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_in_space_with_session,
    NotionLiveError,
};

pub(in crate::live::board::page_mutation) fn submit_page_transaction(
    session: &NotionDesktopSession,
    context: &FavoriteMutationContext,
    now: u64,
    user_action: &'static str,
    operations: Vec<SaveOperation>,
) -> Result<(), NotionLiveError> {
    let request = SaveTransactionsRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        transactions: vec![SaveTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            space_id: context.space_id.clone(),
            debug: TransactionDebug {
                user_action,
                client_commit_time_ms: now,
            },
            operations,
        }],
    };
    let body = serde_json::to_value(request)
        .map_err(|error| format!("failed to serialize Notion page mutation: {error}"))?;
    let response = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::SaveTransactionsFanout,
        &context.space_id,
        &body,
    )?;
    reject_transaction_errors(&response, "page").map_err(NotionLiveError::Fatal)
}
