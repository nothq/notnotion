use serde::{Deserialize, Serialize};

use crate::model::NotionCollectionViewId;

use super::state::{DatabaseFilterGroup, DatabaseSimpleFilter, DatabaseViewFilterState};
use super::NotionDatabaseFilterId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseFilterGroupOperator {
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct DatabaseViewFilterMutationRequest {
    expected_view_id: NotionCollectionViewId,
    mutation: DatabaseViewFilterMutation,
}

impl DatabaseViewFilterMutationRequest {
    pub fn new(
        expected_view_id: NotionCollectionViewId,
        mutation: DatabaseViewFilterMutation,
    ) -> Self {
        Self {
            expected_view_id,
            mutation,
        }
    }

    pub(crate) fn into_parts(self) -> (NotionCollectionViewId, DatabaseViewFilterMutation) {
        (self.expected_view_id, self.mutation)
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseViewFilterSaveRequest {
    expected_view_id: NotionCollectionViewId,
    filter_state: DatabaseViewFilterState,
}

impl DatabaseViewFilterSaveRequest {
    pub fn new(
        expected_view_id: NotionCollectionViewId,
        filter_state: DatabaseViewFilterState,
    ) -> Self {
        Self {
            expected_view_id,
            filter_state,
        }
    }

    pub(crate) fn into_parts(self) -> (NotionCollectionViewId, DatabaseViewFilterState) {
        (self.expected_view_id, self.filter_state)
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseViewFilterQueryRequest {
    expected_view_id: NotionCollectionViewId,
    filter_state: DatabaseViewFilterState,
}

impl DatabaseViewFilterQueryRequest {
    pub fn new(
        expected_view_id: NotionCollectionViewId,
        filter_state: DatabaseViewFilterState,
    ) -> Self {
        Self {
            expected_view_id,
            filter_state,
        }
    }

    pub(crate) fn into_parts(self) -> (NotionCollectionViewId, DatabaseViewFilterState) {
        (self.expected_view_id, self.filter_state)
    }
}

#[derive(Clone, Debug)]
pub enum DatabaseViewFilterMutation {
    Simple(DatabaseSimpleFilterMutation),
    Advanced(DatabaseAdvancedFilterMutation),
}

#[derive(Clone, Debug)]
pub enum DatabaseSimpleFilterMutation {
    Add {
        filter: DatabaseSimpleFilter,
        placement: DatabaseSimpleFilterPlacement,
    },
    Update(DatabaseSimpleFilter),
    Remove(NotionDatabaseFilterId),
}

#[derive(Clone, Debug)]
pub enum DatabaseSimpleFilterPlacement {
    End,
    Before(NotionDatabaseFilterId),
}

#[derive(Clone, Debug)]
pub enum DatabaseAdvancedFilterMutation {
    Set(DatabaseFilterGroup),
    Clear,
}
