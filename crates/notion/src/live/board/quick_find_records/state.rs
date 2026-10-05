use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};

use serde_json::{Map, Value};

use super::super::normalize_uuid;
use super::merge::{merge_quick_find_record_map, preview_root_hydration, touch_record};
use super::preview::record_by_id;
use super::{
    InitialSyncRecordPointer, QuickFindRecordScope, QuickFindRecordState, QuickFindRecords,
    RecordKey, SharedRecordMap,
};

type QuickFindRecordRegistry = Mutex<HashMap<QuickFindRecordScope, Weak<QuickFindRecords>>>;

impl QuickFindRecords {
    pub(in super::super) fn for_scope(active_user_id: &str, space_id: &str) -> Arc<Self> {
        static REGISTRY: OnceLock<QuickFindRecordRegistry> = OnceLock::new();
        let scope = QuickFindRecordScope {
            active_user_id: active_user_id.to_string(),
            space_id: space_id.to_string(),
        };
        let mut registry = REGISTRY
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("Notion Quick Find record registry must not be poisoned");
        registry.retain(|_, records| records.strong_count() > 0);
        if let Some(records) = registry.get(&scope).and_then(Weak::upgrade) {
            return records;
        }
        let records = Arc::new(Self {
            space_id: space_id.to_string(),
            state: Mutex::new(QuickFindRecordState::default()),
        });
        registry.insert(scope, Arc::downgrade(&records));
        records
    }

    pub(in super::super) fn merge_response(&self, response: &Value) -> Result<(), String> {
        let record_map = response
            .get("recordMap")
            .and_then(Value::as_object)
            .ok_or_else(|| "Notion response is missing recordMap".to_string())?;
        self.merge(record_map.clone())
    }

    pub(in super::super) fn merge(&self, record_map: Map<String, Value>) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        merge_quick_find_record_map(&mut state, record_map)
    }

    pub(in super::super) fn merge_tables(
        &self,
        tables: impl IntoIterator<Item = (&'static str, Map<String, Value>)>,
    ) -> Result<(), String> {
        self.merge(
            tables
                .into_iter()
                .map(|(table, records)| (table.to_string(), Value::Object(records)))
                .collect(),
        )
    }

    pub(in super::super) fn merge_hydrated_root(
        &self,
        record_map: Map<String, Value>,
        block_id: &str,
    ) -> Result<(), String> {
        let generation = self.preview_root_generation(block_id)?;
        self.merge_hydrated_root_at_generation(record_map, block_id, generation)
    }

    pub(in super::super) fn merge_hydrated_root_at_generation(
        &self,
        record_map: Map<String, Value>,
        block_id: &str,
        generation: u64,
    ) -> Result<(), String> {
        self.merge_preview_response(record_map, block_id, generation, true)
            .map(|_| ())
    }

    pub(in super::super) fn record_map_snapshot(&self) -> Result<SharedRecordMap, String> {
        self.state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())
            .map(|state| state.record_map.clone())
    }

    pub(in super::super) fn invalidate_root(&self, block_id: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        let removed_id = Arc::make_mut(&mut state.record_map)
            .get_mut("block")
            .and_then(Value::as_object_mut)
            .and_then(|blocks| {
                if blocks.remove(block_id).is_some() {
                    return Some(block_id.to_string());
                }
                let normalized_id = normalize_uuid(block_id);
                let alias = blocks
                    .keys()
                    .find(|id| normalize_uuid(id) == normalized_id)
                    .cloned()?;
                blocks.remove(&alias);
                Some(alias)
            });
        if let Some(removed_id) = removed_id {
            state
                .recency
                .retain(|key| key.table != "block" || key.id != removed_id);
        } else {
            let previous_len = state.recency.len();
            state
                .recency
                .retain(|key| key.table != "block" || key.id != block_id);
            if state.recency.len() == previous_len {
                let normalized_id = normalize_uuid(block_id);
                state
                    .recency
                    .retain(|key| key.table != "block" || normalize_uuid(&key.id) != normalized_id);
            }
        }
        state
            .hydrated_preview_roots
            .remove(&normalize_uuid(block_id));
        let generation = state
            .preview_root_generations
            .entry(normalize_uuid(block_id))
            .or_default();
        *generation = generation.wrapping_add(1);
        Ok(())
    }

    pub(super) fn touch_preview_records(
        &self,
        block_id: &str,
        preview_records: &[InitialSyncRecordPointer],
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        let root = InitialSyncRecordPointer {
            table: "block".to_string(),
            id: block_id.to_string(),
            space_id: self.space_id.clone(),
        };
        for pointer in std::iter::once(&root).chain(preview_records) {
            if let Some(id) = record_by_id(&state.record_map, &pointer.table, &pointer.id)
                .map(|(id, _)| id.to_string())
            {
                touch_record(
                    &mut state.recency,
                    RecordKey {
                        table: pointer.table.clone(),
                        id,
                    },
                );
            }
        }
        Ok(())
    }

    pub(super) fn preview_root_is_hydrated(&self, block_id: &str) -> Result<bool, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        let Some(marked) = state.hydrated_preview_roots.get(&normalize_uuid(block_id)) else {
            return Ok(false);
        };
        Ok(preview_root_hydration(&state.record_map, block_id)?.as_ref() == Some(marked))
    }

    pub(in super::super) fn preview_root_generation(&self, block_id: &str) -> Result<u64, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        Ok(state
            .preview_root_generations
            .get(&normalize_uuid(block_id))
            .copied()
            .unwrap_or_default())
    }

    pub(super) fn merge_preview_response(
        &self,
        record_map: Map<String, Value>,
        block_id: &str,
        generation: u64,
        hydrates_root: bool,
    ) -> Result<bool, String> {
        let response_hydration = if hydrates_root {
            Some(
                preview_root_hydration(&record_map, block_id)?.ok_or_else(|| {
                    format!("Notion Quick Find preview root {block_id} was not hydrated")
                })?,
            )
        } else {
            None
        };
        let normalized_id = normalize_uuid(block_id);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        let current_generation = state
            .preview_root_generations
            .get(&normalized_id)
            .copied()
            .unwrap_or_default();
        if current_generation != generation {
            return Err(format!(
                "Notion Quick Find preview root {block_id} was invalidated while loading"
            ));
        }
        merge_quick_find_record_map(&mut state, record_map)?;
        if let Some(response_hydration) = response_hydration {
            let current_hydration = preview_root_hydration(&state.record_map, block_id)?;
            if current_hydration != Some(response_hydration) {
                return Ok(false);
            }
            state
                .hydrated_preview_roots
                .insert(normalized_id, response_hydration);
        }
        Ok(true)
    }

    #[cfg(test)]
    pub(super) fn mark_preview_root_hydrated(&self, block_id: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Notion Quick Find record cache lock is poisoned".to_string())?;
        let hydration = preview_root_hydration(&state.record_map, block_id)?
            .ok_or_else(|| format!("Notion Quick Find preview root {block_id} is unavailable"))?;
        state
            .hydrated_preview_roots
            .insert(normalize_uuid(block_id), hydration);
        Ok(())
    }
}
