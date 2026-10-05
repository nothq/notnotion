use serde_json::{json, Value};

use crate::model::{
    DatabaseAdvancedFilterState, DatabaseFilterGroup, DatabaseFilterGroupOperator,
    DatabaseFilterNode, DatabaseFilterRelationPage, DatabasePropertyFilter,
    DatabasePropertyFilterCondition, DatabaseSimpleFilter, DatabaseSimpleFilterState,
    DatabaseSimpleFiltersState, DatabaseViewFilterState, LoadDatabaseFilterUsersResult,
    NotionDatabaseFilterId, NotionDatabasePropertyId, NotionFilterPageId,
    SearchDatabaseFilterRelationPagesRequest, SearchDatabaseFilterRelationPagesResult,
};

use super::super::{
    block_value, record_map_table, required_string, title_property_allow_empty,
    unwrap_record_value, LiveWorkspaceContext, NotionPrivateApiEndpoint,
};
use super::sidebar::{combined_block_records, load_missing_sidebar_blocks, page_shell_icon};

mod parse;
mod picker;
mod query;
mod serialize;
mod value;

pub(super) use parse::database_view_filter_state;
pub(crate) use picker::{
    load_database_filter_users, load_visible_workspace_users, search_database_filter_relation_pages,
};
use query::{
    compiled_filter_owner, editable_filter_values, strip_filter_values,
    validate_preserved_unsupported_filters,
};
pub(in crate::live::board) use serialize::{filter_group_value, property_filter_value};
use value::{
    exact_value, parse_date_point_filter, parse_date_range_filter, parse_person_filter,
    parse_relation_filter_values, parse_select_filter_values, parse_status_filter_values,
    parse_text_filter,
};

#[derive(Clone, Debug)]
pub(crate) struct LiveDatabaseQueryState {
    compiled_request: Value,
    reducer_name: &'static str,
    view_id: crate::model::NotionCollectionViewId,
    base_filter_state: DatabaseViewFilterState,
}

impl LiveDatabaseQueryState {
    pub(super) fn from_initial_query_response(
        response: &Value,
        reducer_name: &'static str,
        view_id: crate::model::NotionCollectionViewId,
        base_filter_state: DatabaseViewFilterState,
    ) -> Option<Self> {
        Some(Self {
            compiled_request: response.get("compiledRequest")?.clone(),
            reducer_name,
            view_id,
            base_filter_state,
        })
    }

    pub(crate) fn query_request(
        &self,
        expected_view_id: &crate::model::NotionCollectionViewId,
        filter_state: &DatabaseViewFilterState,
    ) -> Result<Value, String> {
        if expected_view_id != &self.view_id {
            return Err(format!(
                "Notion database filter query expected view {}, active query view is {}",
                expected_view_id.as_str(),
                self.view_id.as_str()
            ));
        }
        validate_preserved_unsupported_filters(&self.base_filter_state, filter_state)?;
        let mut base_filters = editable_filter_values(&self.base_filter_state)?;
        let desired_filters = editable_filter_values(filter_state)?;
        let mut request = self.compiled_request.clone();
        let filter_owner = compiled_filter_owner(&mut request, self.reducer_name)?;
        let structural_filter = filter_owner
            .remove("filter")
            .and_then(|filter| strip_filter_values(filter, &mut base_filters));
        if !base_filters.is_empty() {
            return Err(format!(
                "compiled Notion database query omitted {} persisted filter predicate(s)",
                base_filters.len()
            ));
        }
        let filters = structural_filter
            .into_iter()
            .chain(desired_filters)
            .collect::<Vec<_>>();
        if filters.is_empty() {
            filter_owner.remove("filter");
        } else {
            filter_owner.insert(
                "filter".to_string(),
                json!({ "operator": "and", "filters": filters }),
            );
        }
        Ok(request)
    }
}
