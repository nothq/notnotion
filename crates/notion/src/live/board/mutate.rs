use super::{
    load::{filter_group_value, property_filter_value},
    notion_block_id, notion_title_crdt_token, reject_transaction_errors, CalendarDateMutation,
    CalendarPageCreation, DatabaseMutationContext, FavoriteMutationContext, LiveBoardMutator,
    MoveCardMutationRequest, MutationContext, NotionPrivateApiEndpoint,
};
use crate::live::{
    credentials::{current_notion_desktop_session, NotionDesktopSession},
    NotionLiveError,
};
use crate::model::{DatabaseStatusPropertyMutationScope, SetDatabaseStatusPropertyRequest};
use crate::model::{
    DatabaseViewControlMutationRequest, DatabaseViewFilterMutationRequest,
    DatabaseViewFilterSaveRequest,
};
mod favorite;
mod filter;
mod page;
mod transaction;
mod view_controls;

use filter::{database_filter_operation, database_filter_save_operations};
use page::{
    calendar_date_operations, create_calendar_page_operations, create_page_operations,
    move_card_operations, mutation_now_ms, new_page_mutation, status_property_update_operation,
};
use transaction::{submit_transactions, TransactionSubmitRequest};
use view_controls::prepare_database_view_control_mutation;

impl MutationContext {
    fn favorite(&self) -> &FavoriteMutationContext {
        match self {
            Self::Page(context) => context,
            Self::Database(context) => &context.favorite,
        }
    }

    fn database(&self) -> Result<&DatabaseMutationContext, String> {
        match self {
            Self::Page(_) => Err("Notion database mutation is unavailable for a page".to_string()),
            Self::Database(context) => Ok(context),
        }
    }

    fn database_mut(&mut self) -> Result<&mut DatabaseMutationContext, String> {
        match self {
            Self::Page(_) => Err("Notion database mutation is unavailable for a page".to_string()),
            Self::Database(context) => Ok(context),
        }
    }
}

impl LiveBoardMutator {
    pub fn create_page_in_column(&self, target_column_title: &str) -> Result<String, String> {
        let session = current_notion_desktop_session().map_err(|error| error.to_string())?;
        self.create_page_in_column_with_session(&session, target_column_title)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn create_page_in_column_with_session(
        &self,
        session: &NotionDesktopSession,
        target_column_title: &str,
    ) -> Result<String, NotionLiveError> {
        let context = self.context.database()?;
        let status_property_id = context
            .board_group()?
            .property_for_new_card(target_column_title)?;
        let mutation = new_page_mutation(context)?;
        let operations =
            create_page_operations(context, &mutation, status_property_id, target_column_title);
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "BoardGroupAddItemButton.handleClick",
                client_commit_time_ms: mutation.now,
                operations,
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsMain,
            },
        )?;
        Ok(mutation.block_id)
    }

    pub(crate) fn create_calendar_page(
        &self,
        session: &NotionDesktopSession,
        creation: &CalendarPageCreation,
    ) -> Result<String, NotionLiveError> {
        let context = self.context.database()?;
        let mutation = new_page_mutation(context)?;
        let operations = create_calendar_page_operations(context, &mutation, creation);
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "Calendar.createBlockInDay",
                client_commit_time_ms: mutation.now,
                operations,
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsMain,
            },
        )?;
        Ok(mutation.block_id)
    }

    pub fn move_card(&self, request: MoveCardMutationRequest<'_>) -> Result<(), String> {
        let session = current_notion_desktop_session().map_err(|error| error.to_string())?;
        self.move_card_with_session(&session, request)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn move_card_with_session(
        &self,
        session: &NotionDesktopSession,
        request: MoveCardMutationRequest<'_>,
    ) -> Result<(), NotionLiveError> {
        let context = self.context.database()?;
        let now = mutation_now_ms();
        let operations = move_card_operations(context, request, now)?;
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "notnotion.board.drop",
                client_commit_time_ms: now,
                operations,
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )?;
        Ok(())
    }

    pub(crate) fn mutate_database_view_filter(
        &mut self,
        session: &NotionDesktopSession,
        request: DatabaseViewFilterMutationRequest,
    ) -> Result<(), NotionLiveError> {
        let (expected_view_id, mutation) = request.into_parts();
        let context = self.context.database_mut()?;
        if expected_view_id.as_str() != context.collection_view_id {
            return Err(format!(
                "Notion database filter request expected view {}, active view is {}",
                expected_view_id.as_str(),
                context.collection_view_id
            )
            .into());
        }
        let next_filter_state = context.filter_state.applying(&mutation)?;
        let operation = database_filter_operation(context, &mutation);
        let query2_filter =
            database_query2_filter_from_operations(std::slice::from_ref(&operation));
        let now = mutation_now_ms();
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "notnotion.inline_database.filter",
                client_commit_time_ms: now,
                operations: vec![operation],
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )?;
        context.filter_state = next_filter_state;
        if let Some(filter) = query2_filter {
            context
                .view_controls
                .query2
                .insert("filter".to_string(), filter);
        }
        Ok(())
    }

    pub(crate) fn save_database_view_filter_state(
        &mut self,
        session: &NotionDesktopSession,
        request: DatabaseViewFilterSaveRequest,
    ) -> Result<(), NotionLiveError> {
        let (expected_view_id, filter_state) = request.into_parts();
        let context = self.context.database_mut()?;
        if expected_view_id.as_str() != context.collection_view_id {
            return Err(format!(
                "Notion database filter save expected view {}, active view is {}",
                expected_view_id.as_str(),
                context.collection_view_id
            )
            .into());
        }
        let operations = database_filter_save_operations(context, &filter_state)?;
        let query2_filter = database_query2_filter_from_operations(&operations);
        let now = mutation_now_ms();
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "CollectionSettingsSaveControl.handleSaveInCurrentViewClick",
                client_commit_time_ms: now,
                operations,
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )?;
        context.filter_state = filter_state;
        if let Some(filter) = query2_filter {
            context
                .view_controls
                .query2
                .insert("filter".to_string(), filter);
        }
        Ok(())
    }

    pub(crate) fn mutate_database_view_control(
        &mut self,
        session: &NotionDesktopSession,
        request: DatabaseViewControlMutationRequest,
    ) -> Result<(), NotionLiveError> {
        let (expected_view_id, mutation) = request.into_parts();
        let context = self.context.database_mut()?;
        if expected_view_id.as_str() != context.collection_view_id {
            return Err(format!(
                "Notion database control request expected view {}, active view is {}",
                expected_view_id.as_str(),
                context.collection_view_id
            )
            .into());
        }
        let prepared = prepare_database_view_control_mutation(context, mutation)?;
        let now = mutation_now_ms();
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action: "notnotion.database.view_control",
                client_commit_time_ms: now,
                operations: vec![prepared.operation],
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )?;
        context.view_controls = prepared.next;
        Ok(())
    }

    pub(crate) fn set_calendar_page_date(
        &self,
        session: &NotionDesktopSession,
        mutation: &CalendarDateMutation,
    ) -> Result<(), NotionLiveError> {
        let context = self.context.database()?;
        let now = mutation_now_ms();
        let user_action = match mutation.source {
            crate::model::CalendarDateAssignmentSource::NoDateClick => {
                "CollectionViewBlock.insertedStore"
            }
            crate::model::CalendarDateAssignmentSource::CalendarDayDrop
            | crate::model::CalendarDateAssignmentSource::CalendarEndpointResize => {
                "Calendar.renderDay"
            }
        };
        submit_transactions(
            session,
            &context.favorite,
            TransactionSubmitRequest {
                user_action,
                client_commit_time_ms: now,
                operations: calendar_date_operations(context, mutation, now),
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )
    }

    pub(crate) fn set_database_status_property(
        &self,
        session: &NotionDesktopSession,
        request: SetDatabaseStatusPropertyRequest,
    ) -> Result<(), NotionLiveError> {
        let (page_id, scope, property_id, option) = request.into_parts();
        match scope {
            DatabaseStatusPropertyMutationScope::DatabaseView(expected_view_id) => {
                let context = self.context.database()?;
                if expected_view_id.as_str() != context.collection_view_id {
                    return Err(format!(
                        "Notion status property request expected view {}, active view is {}",
                        expected_view_id.as_str(),
                        context.collection_view_id
                    )
                    .into());
                }
            }
            DatabaseStatusPropertyMutationScope::LoadedPage => {
                if !self.page_responses.contains_key(&page_id) {
                    return Err(format!(
                        "Notion status property request requires loaded page {page_id}"
                    )
                    .into());
                }
            }
        }
        let favorite = self.context.favorite();
        let now = mutation_now_ms();
        submit_transactions(
            session,
            favorite,
            TransactionSubmitRequest {
                user_action: "notnotion.database.status_property",
                client_commit_time_ms: now,
                operations: vec![status_property_update_operation(
                    favorite,
                    &page_id,
                    property_id.as_str(),
                    &option.value,
                )],
                endpoint: NotionPrivateApiEndpoint::SaveTransactionsFanout,
            },
        )
    }
}

fn database_query2_filter_from_operations(
    operations: &[serde_json::Value],
) -> Option<serde_json::Value> {
    operations.iter().find_map(|operation| {
        let path = operation.get("path")?.as_array()?;
        if path.len() != 1 || path[0].as_str() != Some("query2") {
            return None;
        }
        operation.get("args")?.get("filter").cloned()
    })
}
