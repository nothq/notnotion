use crate::model::{
    CalendarItemsQuery, CreateCalendarPageRequest, LoadCalendarItemsRequest,
    LoadCalendarItemsResult, SetCalendarPageDateRangeRequest, SetCalendarPageDateRequest,
};
use serde_json::{json, Map, Value};

use crate::live::board::record_map::merge_record_map;

mod query;
mod value;

use query::{
    optional_record_map_table, record_value_mut, required_record_map_table, take_record_map,
    undated_results_reducer, CompiledCollectionRequest, PreparedCalendarItemsQuery,
    ResultsReducerTemplate,
};
pub(crate) use query::{query_calendar_items, query_date_undated_count};
use value::{
    decode_date_property_value, move_calendar_date_value, new_calendar_date_value,
    resize_calendar_date_value,
};

use super::super::{
    block_value, board_item_snapshot, collection_entry, required_string_array,
    CalendarDateMutation, CalendarPageCreation, PropertyLookup,
};

const CALENDAR_RESULTS_REDUCER: &str = "calendar_results";
const UNDATED_RESULTS_REDUCER: &str = "empty_organize_by_results";

#[derive(Clone, Debug)]
pub(crate) struct LiveCollectionQueryState {
    compiled_request: CompiledCollectionRequest,
    record_map: Map<String, Value>,
    collection_id: String,
    date_property_id: String,
    calendar_results: Option<ResultsReducerTemplate>,
}

/// The Calendar date property schema and the page's current date value.
type CalendarDateMutationInput<'a> = (&'a Map<String, Value>, Option<Value>);

impl LiveCollectionQueryState {
    pub(super) fn from_initial_query_response(
        response: &Value,
        collection_id: String,
        date_property_id: String,
        calendar_active: bool,
    ) -> Result<Self, String> {
        let compiled_request = response.get("compiledRequest").cloned().ok_or_else(|| {
            "missing compiledRequest in Notion date view query response".to_string()
        })?;
        let compiled_request = CompiledCollectionRequest::parse(compiled_request)?;
        let record_map = response
            .get("recordMap")
            .and_then(Value::as_object)
            .cloned()
            .ok_or_else(|| "missing recordMap in completed Notion date view query".to_string())?;
        collection_entry(
            required_record_map_table(&record_map, "collection")?,
            &collection_id,
        )?;
        required_record_map_table(&record_map, "block")?;
        let calendar_results = calendar_active
            .then(|| {
                ResultsReducerTemplate::parse(
                    compiled_request.reducer(CALENDAR_RESULTS_REDUCER)?,
                    &date_property_id,
                )
            })
            .transpose()?;
        Ok(Self {
            compiled_request,
            record_map,
            collection_id,
            date_property_id,
            calendar_results,
        })
    }

    pub(crate) fn prepare_calendar_date_mutation(
        &self,
        request: &SetCalendarPageDateRequest,
    ) -> Result<CalendarDateMutation, String> {
        let (date_property, current_date) =
            self.calendar_date_mutation_input(request.block_id())?;
        let date_value = match current_date {
            Some(date_value) => move_calendar_date_value(date_value, request.target_date())?,
            None => new_calendar_date_value(request.target_date(), date_property),
        };
        Ok(CalendarDateMutation {
            block_id: request.block_id().to_string(),
            date_property_id: self.date_property_id.clone(),
            date_value,
            source: request.source(),
        })
    }

    pub(crate) fn prepare_calendar_page_creation(
        &self,
        request: &CreateCalendarPageRequest,
    ) -> Result<CalendarPageCreation, String> {
        if self.calendar_results.is_none() {
            return Err(
                "Notion Calendar page creation requires an active Calendar view".to_string(),
            );
        }
        let date_property = self.calendar_date_property()?;
        Ok(CalendarPageCreation {
            date_property_id: self.date_property_id.clone(),
            date_value: new_calendar_date_value(request.target_date(), date_property),
        })
    }

    pub(crate) fn prepare_calendar_date_range_mutation(
        &self,
        request: &SetCalendarPageDateRangeRequest,
    ) -> Result<CalendarDateMutation, String> {
        let (_, current_date) = self.calendar_date_mutation_input(request.block_id())?;
        let current_date = current_date.ok_or_else(|| {
            format!(
                "Notion Calendar page {} has no existing date value to resize",
                request.block_id()
            )
        })?;
        let date_value = resize_calendar_date_value(
            current_date,
            request.inclusive_start_date(),
            request.inclusive_end_date(),
        )?;
        Ok(CalendarDateMutation {
            block_id: request.block_id().to_string(),
            date_property_id: self.date_property_id.clone(),
            date_value,
            source: request.source(),
        })
    }

    fn calendar_date_mutation_input<'a>(
        &'a self,
        block_id: &str,
    ) -> Result<CalendarDateMutationInput<'a>, String> {
        let blocks = required_record_map_table(&self.record_map, "block")?;
        let block = block_value(blocks, block_id)?;
        let date_property = self.calendar_date_property()?;
        let properties = block
            .get("properties")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("Notion Calendar page {block_id} has no properties"))?;
        let current_date = decode_date_property_value(
            properties.get(&self.date_property_id),
            block_id,
            &self.date_property_id,
        )?;
        Ok((date_property, current_date))
    }

    fn calendar_date_property(&self) -> Result<&Map<String, Value>, String> {
        let collections = required_record_map_table(&self.record_map, "collection")?;
        let collection = collection_entry(collections, &self.collection_id)?;
        let date_property = collection
            .get("schema")
            .and_then(Value::as_object)
            .and_then(|schema| schema.get(&self.date_property_id))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                format!(
                    "Notion Calendar collection {} has no date property {}",
                    self.collection_id, self.date_property_id
                )
            })?;
        if date_property.get("type").and_then(Value::as_str) != Some("date") {
            return Err(format!(
                "Notion Calendar property {} is not a date property",
                self.date_property_id
            ));
        }
        Ok(date_property)
    }

    pub(crate) fn commit_calendar_date_mutation(
        &mut self,
        mutation: &CalendarDateMutation,
    ) -> Result<(), String> {
        if mutation.date_property_id != self.date_property_id {
            return Err("Notion Calendar mutation property changed before commit".to_string());
        }
        let blocks = self
            .record_map
            .get_mut("block")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "missing recordMap.block".to_string())?;
        let block = blocks
            .get_mut(&mutation.block_id)
            .and_then(record_value_mut)
            .ok_or_else(|| format!("missing block {}", mutation.block_id))?;
        let properties = block
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                format!(
                    "Notion Calendar page {} has no mutable properties",
                    mutation.block_id
                )
            })?;
        properties.insert(
            mutation.date_property_id.clone(),
            json!([["‣", [["d", mutation.date_value.clone()]]]]),
        );
        Ok(())
    }

    fn prepare_undated_count_request(&self) -> Value {
        self.compiled_request.with_only_reducer(
            "no_date_count",
            json!({
                "type": "aggregation",
                "aggregation": {
                    "property": self.date_property_id,
                    "aggregator": "empty",
                    "enforceMaxAggregationLimit": true,
                },
            }),
        )
    }

    fn prepare_calendar_items_request(
        &self,
        request: LoadCalendarItemsRequest,
    ) -> Result<PreparedCalendarItemsQuery, String> {
        let (reducer_name, reducer) = match request.into_query() {
            CalendarItemsQuery::Month {
                grid_start_date,
                grid_end_date,
            } => {
                let calendar_results = self.calendar_results.as_ref().ok_or_else(|| {
                    "Notion Calendar month loading requires an active Calendar view".to_string()
                })?;
                (
                    CALENDAR_RESULTS_REDUCER,
                    calendar_results.month_reducer(
                        &self.date_property_id,
                        &grid_start_date,
                        &grid_end_date,
                    ),
                )
            }
            CalendarItemsQuery::Undated {
                limit,
                search_query,
            } => (
                UNDATED_RESULTS_REDUCER,
                undated_results_reducer(&self.date_property_id, limit, search_query),
            ),
        };
        Ok(PreparedCalendarItemsQuery {
            body: self
                .compiled_request
                .with_only_reducer(reducer_name, reducer),
            reducer_name,
        })
    }

    fn merge_calendar_items_response(
        &mut self,
        mut response: Value,
        reducer_name: &str,
    ) -> Result<LoadCalendarItemsResult, String> {
        let block_ids = response
            .get("result")
            .and_then(|result| result.get("reducerResults"))
            .and_then(|reducers| reducers.get(reducer_name))
            .ok_or_else(|| format!("missing {reducer_name} reducer results"))
            .and_then(|reducer| required_string_array(reducer, "blockIds"))?;
        let incoming_record_map = take_record_map(&mut response)?;
        merge_record_map(&mut self.record_map, incoming_record_map)?;
        let returned_block_count = block_ids.len();
        let items = self.board_items(&block_ids)?;
        Ok(LoadCalendarItemsResult {
            items,
            returned_block_count,
        })
    }

    fn board_items(&self, block_ids: &[String]) -> Result<Vec<crate::model::BoardItem>, String> {
        let blocks = required_record_map_table(&self.record_map, "block")?;
        let collections = required_record_map_table(&self.record_map, "collection")?;
        let collection_schema = collection_entry(collections, &self.collection_id)?
            .get("schema")
            .and_then(Value::as_object);
        let users = optional_record_map_table(&self.record_map, "notion_user");
        let property_lookup = PropertyLookup::preloaded(Some(blocks), users);
        let mut items = Vec::new();
        for block_id in block_ids {
            let block = block_value(blocks, block_id)?;
            items.push(board_item_snapshot(
                block_id,
                block,
                collection_schema,
                property_lookup,
            )?);
        }
        Ok(items)
    }
}
