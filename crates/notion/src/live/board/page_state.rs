use super::{
    loaded_record_value, record_map_table, required_array, required_string, required_u64,
    CompletePageResponse, HashMap, HashSet, Value,
};
use crate::model::{CardPageBlockColor, CardPageColumnRatio, NotionPageBlockKind};

mod annotation;
mod crdt;
mod simple_table;

use simple_table::{parse_simple_table_record, NotionSimpleTableRecord};

pub(super) use annotation::{
    SerializedAnnotationAnchor, SerializedAnnotationBoundary, SerializedAnnotationOperation,
    SerializedTextItemAnnotations,
};
pub(super) use crdt::{
    CrdtItemId, CrdtOperationId, SerializedTextItem, SerializedTextSlice, SerializedTextSliceNode,
    SerializedTextSliceTree,
};

#[derive(Clone, Debug)]
pub(crate) struct PageMutationState {
    pub(super) page_block_id: String,
    pub(super) space_id: String,
    pub(super) blocks: HashMap<String, NotionBlockRecord>,
    opaque_unavailable_block_ids: HashSet<String>,
}

#[derive(Clone, Debug)]
pub(super) struct NotionBlockRecord {
    pub(super) id: String,
    pub(super) space_id: String,
    pub(super) parent_id: String,
    pub(super) parent_table: String,
    pub(super) alive: bool,
    pub(super) version: u64,
    pub(super) kind: NotionPageBlockKind,
    pub(super) alias_target: Option<NotionBlockPointer>,
    pub(super) block_color: Option<String>,
    pub(super) column_ratio: Option<CardPageColumnRatio>,
    pub(super) content_ids: Vec<String>,
    pub(super) title: Option<CrdtTitleState>,
    simple_table: Option<NotionSimpleTableRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NotionBlockPointer {
    pub(super) id: String,
    pub(super) space_id: String,
}

#[derive(Clone, Debug)]
pub(super) struct CrdtTitleState {
    pub(super) tree: SerializedTextSliceTree,
}

impl PageMutationState {
    pub(super) fn parse(
        page_block_id: &str,
        response: &CompletePageResponse,
    ) -> Result<Self, String> {
        let opaque_unavailable_blocks = response.opaque_unavailable_blocks();
        let response_value = response.value()?;
        let raw_blocks = record_map_table(&response_value, "block")?;
        let root = raw_blocks
            .get(page_block_id)
            .and_then(loaded_record_value)
            .ok_or_else(|| format!("missing loaded page block {page_block_id}"))?;
        let space_id = required_string(root, "space_id")?.to_string();
        let mut blocks = HashMap::new();
        let mut opaque_unavailable_block_ids = HashSet::new();
        let mut pending = vec![(page_block_id.to_string(), None)];

        while let Some((block_id, parent_block_id)) = pending.pop() {
            let loaded_value = raw_blocks.get(&block_id).and_then(loaded_record_value);
            if loaded_value.is_none()
                && consume_opaque_mutation_block(
                    opaque_unavailable_blocks,
                    &block_id,
                    parent_block_id.as_deref(),
                    &space_id,
                    &mut opaque_unavailable_block_ids,
                )?
            {
                continue;
            }
            if blocks.contains_key(&block_id) {
                continue;
            }
            let value =
                loaded_value.ok_or_else(|| format!("missing hydrated page block {block_id}"))?;
            append_loaded_mutation_block(
                value,
                block_id,
                &MutationPageScope {
                    page_block_id,
                    space_id: &space_id,
                },
                &mut pending,
                &mut blocks,
            )?;
        }

        Ok(Self {
            page_block_id: page_block_id.to_string(),
            space_id,
            blocks,
            opaque_unavailable_block_ids,
        })
    }

    pub(super) fn block(&self, block_id: &str) -> Result<&NotionBlockRecord, String> {
        if self.is_opaque_unavailable(block_id) {
            return Err(format!(
                "opaque unavailable block {block_id} cannot be targeted for editing"
            ));
        }
        self.blocks.get(block_id).ok_or_else(|| {
            format!(
                "block {block_id} is not loaded in page {}",
                self.page_block_id
            )
        })
    }

    pub(crate) fn contains_block(&self, block_id: &str) -> bool {
        self.blocks.contains_key(block_id) || self.is_opaque_unavailable(block_id)
    }

    pub(super) fn is_opaque_unavailable(&self, block_id: &str) -> bool {
        self.opaque_unavailable_block_ids.contains(block_id)
    }
}

/// Blocks still to visit, each with the parent that references it.
type PendingMutationBlocks = Vec<(String, Option<String>)>;

struct MutationPageScope<'a> {
    page_block_id: &'a str,
    space_id: &'a str,
}

fn append_loaded_mutation_block(
    value: &Value,
    block_id: String,
    scope: &MutationPageScope<'_>,
    pending: &mut PendingMutationBlocks,
    blocks: &mut HashMap<String, NotionBlockRecord>,
) -> Result<(), String> {
    let MutationPageScope {
        page_block_id,
        space_id,
    } = *scope;
    let mut block = NotionBlockRecord::parse(value)?;
    if block.id != block_id {
        return Err(format!(
            "Notion record key {block_id} contains block {}",
            block.id
        ));
    }
    if block.space_id != space_id {
        return Err(format!(
            "Notion page {page_block_id} contains cross-space block {block_id}"
        ));
    }
    if block.id != page_block_id && block.kind.starts_separate_content_scope() {
        block.content_ids.clear();
    } else {
        pending.extend(
            block
                .content_ids
                .iter()
                .cloned()
                .map(|child_id| (child_id, Some(block.id.clone()))),
        );
    }
    blocks.insert(block_id, block);
    Ok(())
}

fn consume_opaque_mutation_block(
    proofs: &super::ProvenOpaqueUnavailableBlocks,
    block_id: &str,
    parent_block_id: Option<&str>,
    space_id: &str,
    reached: &mut HashSet<String>,
) -> Result<bool, String> {
    let Some(proof) = proofs.get(block_id) else {
        return Ok(false);
    };
    let parent_block_id = parent_block_id
        .ok_or_else(|| format!("opaque Notion block {block_id} cannot be the page root"))?;
    if !proof.validates_content_edge(block_id, parent_block_id, space_id) {
        return Err(format!(
            "opaque Notion block {block_id} proof does not match mutation-state parent {parent_block_id} in space {space_id}"
        ));
    }
    if !reached.insert(block_id.to_string()) {
        return Err(format!(
            "page hierarchy contains opaque block {block_id} more than once"
        ));
    }
    Ok(true)
}

impl NotionPageBlockKind {
    fn starts_separate_content_scope(&self) -> bool {
        matches!(
            self,
            Self::Page | Self::LinkToPage | Self::Alias | Self::Code | Self::Image
        ) || matches!(
            self,
            Self::Other(block_type)
                if matches!(block_type.as_str(), "collection_view" | "collection_view_page")
        )
    }
}

impl NotionBlockRecord {
    fn parse(value: &Value) -> Result<Self, String> {
        let id = required_string(value, "id")?.to_string();
        let title = parse_title(value, &id)?;
        let block_type = required_string(value, "type")
            .map_err(|error| format!("Notion mutation block {id}: {error}"))?;
        let kind = NotionPageBlockKind::from_api_type(block_type);
        let simple_table = parse_simple_table_record(value, &id, block_type)?;
        let alias_target = parse_alias_target(value, &id, &kind)?;
        let block_color = parse_block_color(value, &id)?;
        let column_ratio = parse_column_ratio(value, &id, &kind)?;
        Ok(Self {
            id,
            space_id: required_string(value, "space_id")?.to_string(),
            parent_id: required_string(value, "parent_id")?.to_string(),
            parent_table: required_string(value, "parent_table")?.to_string(),
            alive: value
                .get("alive")
                .and_then(Value::as_bool)
                .ok_or_else(|| "missing boolean field alive".to_string())?,
            version: required_u64(value, "version")?,
            kind,
            alias_target,
            block_color,
            column_ratio,
            content_ids: match value.get("content") {
                Some(_) => required_array(value, "content")?
                    .iter()
                    .map(|id| {
                        id.as_str()
                            .map(str::to_string)
                            .ok_or_else(|| format!("block {id} has a non-string content id"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                None => Vec::new(),
            },
            title,
            simple_table,
        })
    }

    pub(super) fn title(&self) -> Result<&CrdtTitleState, String> {
        self.title
            .as_ref()
            .ok_or_else(|| format!("block {} does not contain CRDT title state", self.id))
    }
}

fn parse_column_ratio(
    value: &Value,
    block_id: &str,
    kind: &NotionPageBlockKind,
) -> Result<Option<CardPageColumnRatio>, String> {
    if !matches!(kind, NotionPageBlockKind::Other(block_type) if block_type == "column") {
        return Ok(None);
    }
    super::column_ratio::parse_live_column_ratio(value, block_id)
}

fn parse_block_color(value: &Value, block_id: &str) -> Result<Option<String>, String> {
    let Some(raw) = value
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("block_color"))
    else {
        return Ok(None);
    };
    let raw = raw
        .as_str()
        .ok_or_else(|| format!("Notion block {block_id} contains a non-string block color"))?;
    CardPageBlockColor::from_api_value(Some(raw))
        .map_err(|error| format!("Notion block {block_id}: {error}"))?;
    Ok(Some(raw.to_string()))
}

fn parse_alias_target(
    value: &Value,
    block_id: &str,
    kind: &NotionPageBlockKind,
) -> Result<Option<NotionBlockPointer>, String> {
    if kind != &NotionPageBlockKind::Alias {
        return Ok(None);
    }
    let pointer = value
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("alias_pointer"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Notion alias block {block_id} is missing its alias pointer"))?;
    if pointer.get("table").and_then(Value::as_str) != Some("block") {
        return Err(format!(
            "Notion alias block {block_id} points to a non-block record"
        ));
    }
    let id = pointer
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty target block ID"))?;
    let space_id = pointer
        .get("spaceId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty target space ID"))?;
    Ok(Some(NotionBlockPointer {
        id: id.to_string(),
        space_id: space_id.to_string(),
    }))
}

fn parse_title(value: &Value, block_id: &str) -> Result<Option<CrdtTitleState>, String> {
    let Some(serialized) = value.get("crdt_data").and_then(|data| data.get("title")) else {
        return Ok(None);
    };
    if value.get("crdt_format_version").and_then(Value::as_u64) != Some(1) {
        return Err(format!("block {block_id} uses an unsupported CRDT format"));
    }
    let tree = serde_json::from_value::<SerializedTextSliceTree>(serialized.clone())
        .map_err(|error| format!("invalid title CRDT for block {block_id}: {error}"))?;
    tree.validate(block_id)?;
    Ok(Some(CrdtTitleState { tree }))
}
