use super::super::page_state::{CrdtItemId, CrdtOperationId, SerializedTextSliceTree};
use serde::Serialize;
use serde_json::Value;

mod code_record;
mod operation;
mod pointer;
mod previous_items;
mod table_cell;
mod transaction;

pub(super) use code_record::NewCodeBlockArgs;
pub(super) use operation::SaveOperation;
pub(super) use pointer::RecordPointer;
pub(super) use previous_items::insert_text_prev_items;
pub(super) use table_cell::{SimpleTableCellPropertyPath, SimpleTableCellPropertyValueArgs};
pub(super) use transaction::submit_page_transaction;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SaveTransactionsRequest {
    pub(super) request_id: String,
    pub(super) transactions: Vec<SaveTransaction>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SaveTransaction {
    pub(super) id: String,
    pub(super) space_id: String,
    pub(super) debug: TransactionDebug,
    pub(super) operations: Vec<SaveOperation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TransactionDebug {
    pub(super) user_action: &'static str,
    pub(super) client_commit_time_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct NewBlockArgs {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) block_type: String,
    pub(super) space_id: String,
    pub(super) created_time: u64,
    pub(super) created_by_table: &'static str,
    pub(super) created_by_id: String,
    pub(super) last_edited_time: u64,
    pub(super) crdt_data: NewBlockCrdtData,
    pub(super) crdt_format_version: u8,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct PastedBlockArgs {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) block_type: &'static str,
    pub(super) properties: EmptyTitleProperties,
    pub(super) parent_id: String,
    pub(super) parent_table: &'static str,
    pub(super) alive: bool,
    pub(super) space_id: String,
    pub(super) created_time: u64,
    pub(super) created_by_table: &'static str,
    pub(super) created_by_id: String,
    pub(super) last_edited_time: u64,
    pub(super) last_edited_by_table: &'static str,
    pub(super) last_edited_by_id: String,
    pub(super) crdt_data: NewBlockCrdtData,
    pub(super) crdt_format_version: u8,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct DuplicatedAliasRecordArgs {
    pub(super) id: String,
    pub(super) version: u64,
    #[serde(rename = "type")]
    pub(super) block_type: &'static str,
    pub(super) last_edited_time: u64,
    pub(super) last_edited_by_table: &'static str,
    pub(super) last_edited_by_id: String,
    pub(super) space_id: String,
    pub(super) copied_from: String,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct EmptyTitleProperties {
    pub(super) title: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct NewBlockCrdtData {
    pub(super) title: SerializedTextSliceTree,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ParentArgs {
    pub(super) parent_id: String,
    pub(super) parent_table: &'static str,
    pub(super) alive: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct BlockTypeArgs {
    #[serde(rename = "type")]
    pub(super) block_type: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BlockPropertyValueArgs {
    pub(super) primitive_op: BlockPropertyPrimitiveOperation,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct BlockPropertyPrimitiveOperation {
    pub(super) command: &'static str,
    pub(super) args: BlockPropertyPrimitiveArgs,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(super) enum BlockPropertyPrimitiveArgs {
    ToDo(ToDoCheckedPropertyArgs),
    CodeLanguage(CodeLanguagePropertyArgs),
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ToDoCheckedPropertyArgs {
    pub(super) checked: [[&'static str; 1]; 1],
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct CodeLanguagePropertyArgs {
    pub(super) language: [[String; 1]; 1],
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct EmptyBlockPropertyValueExpectedVersions {}

#[derive(Clone, Debug, Serialize)]
pub(super) struct BlockColorArgs {
    pub(super) block_color: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct QuoteSizeArgs {
    pub(super) quote_size: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct CodeWrapArgs {
    pub(super) code_wrap: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct AliasCopyFormatArgs {
    pub(super) alias_pointer: RecordPointer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) block_color: Option<String>,
    pub(super) copied_from_pointer: RecordPointer,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct EmptyBlockFormatArgs {}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(super) enum BlockFormatArgs {
    Color(BlockColorArgs),
    QuoteSize(QuoteSizeArgs),
    CodeWrap(CodeWrapArgs),
    ColumnRatio { column_ratio: f64 },
    AliasCopy(AliasCopyFormatArgs),
    Empty(EmptyBlockFormatArgs),
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct EditedMetadataArgs {
    pub(super) last_edited_time: u64,
    pub(super) last_edited_by_table: &'static str,
    pub(super) last_edited_by_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct AliveArgs {
    pub(super) alive: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ListAfterArgs {
    pub(super) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) after: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ListBeforeArgs {
    pub(super) id: String,
    pub(super) before: String,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct ListRemoveArgs {
    pub(super) id: String,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct InsertChildrenAfterArgs {
    pub(super) ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) after: Option<String>,
}

pub(super) type CrdtIdRange = (CrdtOperationId, u64);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InsertTextArgs {
    #[serde(rename = "type")]
    pub(super) operation_type: &'static str,
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) id: CrdtOperationId,
    pub(super) origin_id: CrdtItemId,
    pub(super) content: String,
    pub(super) prev_items: Vec<InsertTextPrevItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(super) enum InsertTextPrevItem {
    Start(InsertTextPrevStartItem),
    Split(InsertTextPrevSplitItem),
    Text(InsertTextPrevTextItem),
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct InsertTextPrevStartItem {
    #[serde(rename = "type")]
    operation_type: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InsertTextPrevSplitItem {
    #[serde(rename = "type")]
    operation_type: &'static str,
    origin_id: CrdtItemId,
    id: CrdtOperationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation_ops_before: Option<Vec<InsertTextPrevAnnotationOperation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation_ops_after: Option<Vec<InsertTextPrevAnnotationOperation>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InsertTextPrevTextItem {
    #[serde(rename = "type")]
    operation_type: &'static str,
    origin_id: CrdtItemId,
    id: CrdtOperationId,
    length: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation_ops_before: Option<Vec<InsertTextPrevAnnotationOperation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation_ops_after: Option<Vec<InsertTextPrevAnnotationOperation>>,
    deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum InsertTextPrevAnnotationOperation {
    Add(InsertTextPrevAddAnnotation),
    Remove(InsertTextPrevRemoveAnnotation),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextPrevAddAnnotation {
    #[serde(rename = "type")]
    operation_type: &'static str,
    text_instance_id: String,
    search_label: String,
    id: CrdtOperationId,
    start: InsertTextPrevAnnotationBoundary,
    end: InsertTextPrevAnnotationBoundary,
    annotation: Vec<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextPrevRemoveAnnotation {
    #[serde(rename = "type")]
    operation_type: &'static str,
    text_instance_id: String,
    search_label: String,
    id: CrdtOperationId,
    start: InsertTextPrevAnnotationBoundary,
    end: InsertTextPrevAnnotationBoundary,
    annotation_key: String,
}

#[derive(Clone, Debug, Serialize)]
struct InsertTextPrevAnnotationBoundary {
    id: CrdtItemId,
    anchor: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteTextArgs {
    #[serde(rename = "type")]
    pub(super) operation_type: &'static str,
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) id_ranges: Vec<CrdtIdRange>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum AnnotationAnchor {
    Before,
    After,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct AnnotationBoundary {
    pub(super) id: CrdtItemId,
    pub(super) anchor: AnnotationAnchor,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AddAnnotationArgs {
    #[serde(rename = "type")]
    pub(super) operation_type: &'static str,
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) id: CrdtOperationId,
    pub(super) start: AnnotationBoundary,
    pub(super) end: AnnotationBoundary,
    /// Notion's annotation tuple: `["b"]`, `["a", url]`, `["h", color]`, or an
    /// object-valued mention such as `["d", {date object}]`.
    pub(super) annotation: Vec<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RemoveAnnotationArgs {
    #[serde(rename = "type")]
    pub(super) operation_type: &'static str,
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) id: CrdtOperationId,
    pub(super) start: AnnotationBoundary,
    pub(super) end: AnnotationBoundary,
    pub(super) annotation_key: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SplitTextArgs {
    #[serde(rename = "type")]
    pub(super) operation_type: &'static str,
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) id: CrdtOperationId,
    pub(super) origin_id: CrdtItemId,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MoveTextSliceArgs {
    pub(super) text_instance_id: String,
    pub(super) text_slice_start_item_id: CrdtItemId,
    pub(super) source_search_label: String,
    pub(super) target_block_id: String,
    pub(super) target_text_instance_id: String,
    pub(super) target_search_label: String,
    pub(super) target_text_slice_end_item_id: CrdtItemId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) is_merge: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MoveTextSliceIntoBlockArgs {
    pub(super) text_instance_id: String,
    pub(super) text_slice_start_item_id: CrdtItemId,
    pub(super) source_search_label: String,
    pub(super) target_block_id: String,
}
