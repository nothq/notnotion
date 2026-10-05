use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_with_session, NotionPrivateApiEndpoint},
    NotionLiveError,
};
use crate::model::{LoadCalendarItemsRequest, LoadCalendarItemsResult};

use super::LiveCollectionQueryState;

pub(crate) fn query_date_undated_count(
    session: &NotionDesktopSession,
    state: &Mutex<LiveCollectionQueryState>,
) -> Result<usize, NotionLiveError> {
    let request = {
        state
            .lock()
            .map_err(|_| "Notion collection query state lock is poisoned".to_string())?
            .prepare_undated_count_request()
    };
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::QueryCollectionInitialLoadWithAggregations,
        &request,
    )?;
    let response: DateAggregationResponse = serde_json::from_value(response).map_err(|error| {
        format!("failed to decode Notion no-date aggregation response: {error}")
    })?;
    match response.result.reducer_results.no_date_count {
        DateAggregationReducer::Aggregation {
            aggregation_result: DateAggregationResult::Number { value },
        } => Ok(value),
    }
}

pub(crate) fn query_calendar_items(
    session: &NotionDesktopSession,
    state: &Mutex<LiveCollectionQueryState>,
    request: LoadCalendarItemsRequest,
) -> Result<LoadCalendarItemsResult, NotionLiveError> {
    let prepared = {
        state
            .lock()
            .map_err(|_| "Notion collection query state lock is poisoned".to_string())?
            .prepare_calendar_items_request(request)?
    };
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::QueryCollectionInitialLoad,
        &prepared.body,
    )?;
    state
        .lock()
        .map_err(|_| "Notion collection query state lock is poisoned".to_string())?
        .merge_calendar_items_response(response, prepared.reducer_name)
        .map_err(NotionLiveError::Fatal)
}

#[derive(Clone, Debug)]
pub(super) struct CompiledCollectionRequest(Value);

impl CompiledCollectionRequest {
    pub(super) fn parse(request: Value) -> Result<Self, String> {
        request_reducers(&request)?;
        Ok(Self(request))
    }

    pub(super) fn reducer(&self, name: &str) -> Result<&Value, String> {
        request_reducers(&self.0)?
            .get(name)
            .ok_or_else(|| format!("missing loader.reducers.{name} in compiled Notion query"))
    }

    pub(super) fn with_only_reducer(&self, name: &str, reducer: Value) -> Value {
        let mut request = self.0.clone();
        let reducers = request
            .get_mut("loader")
            .expect("compiled Notion query must retain loader")
            .as_object_mut()
            .expect("compiled Notion query loader must remain an object")
            .get_mut("reducers")
            .expect("compiled Notion query loader must retain reducers")
            .as_object_mut()
            .expect("compiled Notion query reducers must remain an object");
        reducers.clear();
        reducers.insert(name.to_string(), reducer);
        request
    }
}

#[derive(Clone, Debug)]
pub(super) struct ResultsReducerTemplate {
    reducer_fields: Map<String, Value>,
    filter_fields: Map<String, Value>,
    base_filters: Vec<Value>,
}

impl ResultsReducerTemplate {
    pub(super) fn parse(reducer: &Value, date_property_id: &str) -> Result<Self, String> {
        let mut reducer_fields = reducer.as_object().cloned().ok_or_else(|| {
            "compiled Notion calendar_results reducer is not an object".to_string()
        })?;
        if reducer_fields.get("type").and_then(Value::as_str) != Some("results") {
            return Err(
                "compiled Notion calendar_results reducer is not a results reducer".to_string(),
            );
        }
        let mut filter_fields = match reducer_fields.remove("filter") {
            Some(Value::Object(filter)) => filter,
            _ => return Err("compiled Notion calendar_results filter is not an object".to_string()),
        };
        if filter_fields.get("operator").and_then(Value::as_str) != Some("and") {
            return Err("compiled Notion calendar_results filter is not an AND filter".to_string());
        }
        let filters = match filter_fields.remove("filters") {
            Some(Value::Array(filters)) => filters,
            _ => {
                return Err("compiled Notion calendar_results filters are not an array".to_string())
            }
        };
        let mut lower_bound_count = 0;
        let mut upper_bound_count = 0;
        let mut base_filters = Vec::new();
        for filter in filters {
            match date_window_bound(&filter, date_property_id) {
                Some(DateWindowBound::Lower) => lower_bound_count += 1,
                Some(DateWindowBound::Upper) => upper_bound_count += 1,
                None => base_filters.push(filter),
            }
        }
        if lower_bound_count != 1 || upper_bound_count != 1 {
            return Err(format!(
                "compiled Notion calendar_results must contain one validated lower and upper date bound, found {lower_bound_count} and {upper_bound_count}"
            ));
        }
        Ok(Self {
            reducer_fields,
            filter_fields,
            base_filters,
        })
    }

    pub(super) fn month_reducer(
        &self,
        date_property_id: &str,
        grid_start_date: &str,
        grid_end_date: &str,
    ) -> Value {
        let filter = self.filter_with([
            json!({
                "property": date_property_id,
                "filter": {
                    "operator": "date_is_on_or_after",
                    "use_end": true,
                    "value": {
                        "type": "exact",
                        "value": { "type": "date", "start_date": grid_start_date },
                    },
                },
            }),
            json!({
                "property": date_property_id,
                "filter": {
                    "operator": "date_is_on_or_before",
                    "value": {
                        "type": "exact",
                        "value": { "type": "date", "start_date": grid_end_date },
                    },
                },
            }),
        ]);
        let mut reducer = self.reducer_fields.clone();
        reducer.insert("filter".to_string(), filter);
        reducer.insert("limit".to_string(), json!(5000));
        Value::Object(reducer)
    }

    fn filter_with<const N: usize>(&self, additional_filters: [Value; N]) -> Value {
        let mut filter = self.filter_fields.clone();
        let filters = self
            .base_filters
            .iter()
            .cloned()
            .chain(additional_filters)
            .collect();
        filter.insert("filters".to_string(), Value::Array(filters));
        Value::Object(filter)
    }
}

pub(super) fn undated_results_reducer(
    date_property_id: &str,
    limit: usize,
    search_query: String,
) -> Value {
    json!({
        "type": "results",
        "filter": {
            "operator": "and",
            "filters": [{
                "property": date_property_id,
                "filter": { "operator": "is_empty" },
            }],
        },
        "limit": limit,
        "searchQuery": search_query,
    })
}

#[derive(Clone, Copy)]
enum DateWindowBound {
    Lower,
    Upper,
}

fn date_window_bound(filter: &Value, date_property_id: &str) -> Option<DateWindowBound> {
    let filter = filter.as_object()?;
    if filter.get("property").and_then(Value::as_str) != Some(date_property_id) {
        return None;
    }
    let predicate = filter.get("filter")?.as_object()?;
    let bound = match predicate.get("operator").and_then(Value::as_str)? {
        "date_is_on_or_after"
            if predicate.get("use_end").and_then(Value::as_bool) == Some(true) =>
        {
            DateWindowBound::Lower
        }
        "date_is_on_or_before" if !predicate.contains_key("use_end") => DateWindowBound::Upper,
        _ => return None,
    };
    let exact = predicate.get("value")?.as_object()?;
    if exact.get("type").and_then(Value::as_str) != Some("exact") {
        return None;
    }
    let date = exact.get("value")?.as_object()?;
    if date.get("type").and_then(Value::as_str) != Some("date")
        || date.get("start_date").and_then(Value::as_str).is_none()
    {
        return None;
    }
    Some(bound)
}

pub(super) struct PreparedCalendarItemsQuery {
    pub(super) body: Value,
    pub(super) reducer_name: &'static str,
}

#[derive(Deserialize)]
struct DateAggregationResponse {
    result: DateAggregationQueryResult,
}

#[derive(Deserialize)]
struct DateAggregationQueryResult {
    #[serde(rename = "reducerResults")]
    reducer_results: DateAggregationReducerResults,
}

#[derive(Deserialize)]
struct DateAggregationReducerResults {
    no_date_count: DateAggregationReducer,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum DateAggregationReducer {
    #[serde(rename = "aggregation")]
    Aggregation {
        #[serde(rename = "aggregationResult")]
        aggregation_result: DateAggregationResult,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum DateAggregationResult {
    #[serde(rename = "number")]
    Number { value: usize },
}

fn request_reducers(request: &Value) -> Result<&Map<String, Value>, String> {
    request
        .get("loader")
        .and_then(Value::as_object)
        .and_then(|loader| loader.get("reducers"))
        .and_then(Value::as_object)
        .ok_or_else(|| "missing loader.reducers in compiled Notion date view query".to_string())
}

pub(super) fn required_record_map_table<'a>(
    record_map: &'a Map<String, Value>,
    table: &str,
) -> Result<&'a Map<String, Value>, String> {
    record_map
        .get(table)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing recordMap.{table}"))
}

pub(super) fn optional_record_map_table<'a>(
    record_map: &'a Map<String, Value>,
    table: &str,
) -> Option<&'a Map<String, Value>> {
    record_map.get(table).and_then(Value::as_object)
}

pub(super) fn record_value_mut(entry: &mut Value) -> Option<&mut Map<String, Value>> {
    let value = entry.get_mut("value")?;
    let record = if value.get("value").is_some() {
        value.get_mut("value")?
    } else {
        value
    };
    record.as_object_mut()
}

pub(super) fn take_record_map(response: &mut Value) -> Result<Map<String, Value>, String> {
    let response = response
        .as_object_mut()
        .ok_or_else(|| "Notion query collection response is not an object".to_string())?;
    match response.remove("recordMap") {
        Some(Value::Object(record_map)) => Ok(record_map),
        _ => Err("missing recordMap object in Notion query collection response".to_string()),
    }
}
