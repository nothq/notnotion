use super::{
    AddAnnotationArgs, AliasCopyFormatArgs, AliveArgs, BlockColorArgs, BlockFormatArgs,
    BlockPropertyValueArgs, BlockTypeArgs, CodeWrapArgs, DeleteTextArgs, DuplicatedAliasRecordArgs,
    EditedMetadataArgs, EmptyBlockFormatArgs, EmptyBlockPropertyValueExpectedVersions,
    InsertChildrenAfterArgs, InsertTextArgs, ListAfterArgs, ListBeforeArgs, ListRemoveArgs,
    MoveTextSliceArgs, MoveTextSliceIntoBlockArgs, NewBlockArgs, NewCodeBlockArgs, ParentArgs,
    PastedBlockArgs, QuoteSizeArgs, RecordPointer, RemoveAnnotationArgs,
    SimpleTableCellPropertyPath, SimpleTableCellPropertyValueArgs, SplitTextArgs,
};
use serde::{Serialize, Serializer};

mod builders;

const ROOT_PATH: &[&str] = &[];
const CONTENT_PATH: &[&str] = &["content"];
const PROPERTIES_PATH: &[&str] = &["properties"];
const FORMAT_PATH: &[&str] = &["format"];
const PAGE_ICON_PATH: &[&str] = &["format", "page_icon"];

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "command")]
pub(in crate::live::board::page_mutation) enum SaveOperation {
    #[serde(rename = "set")]
    SetBlock {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: NewBlockArgs,
    },
    #[serde(rename = "set")]
    SetCodeBlock {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: NewCodeBlockArgs,
    },
    #[serde(rename = "set")]
    SetPastedBlock {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: PastedBlockArgs,
    },
    #[serde(rename = "update")]
    UpdateDuplicatedAlias {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: DuplicatedAliasRecordArgs,
    },
    #[serde(rename = "update")]
    UpdateParent {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: ParentArgs,
    },
    #[serde(rename = "update")]
    UpdateType {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: BlockTypeArgs,
    },
    #[serde(rename = "updateBlockPropertyValue")]
    UpdateBlockPropertyValue {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: BlockPropertyValueArgs,
        #[serde(rename = "blockPropertyValueExpectedVersions")]
        block_property_value_expected_versions: EmptyBlockPropertyValueExpectedVersions,
        #[serde(rename = "additionalUpdatedPointers")]
        additional_updated_pointers: Vec<RecordPointer>,
    },
    #[serde(rename = "updateBlockPropertyValue")]
    SetSimpleTableCell {
        pointer: RecordPointer,
        path: SimpleTableCellPropertyPath,
        args: SimpleTableCellPropertyValueArgs,
        #[serde(rename = "expectedVersion")]
        expected_version: u64,
        #[serde(rename = "onlyLogVersionMismatch")]
        #[serde(serialize_with = "serialize_true")]
        only_log_version_mismatch: (),
        #[serde(rename = "blockPropertyValueExpectedVersions")]
        block_property_value_expected_versions: EmptyBlockPropertyValueExpectedVersions,
        #[serde(rename = "additionalUpdatedPointers")]
        additional_updated_pointers: [RecordPointer; 1],
    },
    #[serde(rename = "update")]
    UpdateBlockFormat {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: BlockFormatArgs,
    },
    #[serde(rename = "set")]
    SetPageIcon {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: Option<String>,
    },
    #[serde(rename = "update")]
    UpdateMetadata {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: EditedMetadataArgs,
    },
    #[serde(rename = "update")]
    SetAlive {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: AliveArgs,
    },
    #[serde(rename = "listAfter")]
    ListAfter {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: ListAfterArgs,
    },
    #[serde(rename = "listBefore")]
    ListBefore {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: ListBeforeArgs,
    },
    #[serde(rename = "listRemove")]
    ListRemove {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: ListRemoveArgs,
    },
    #[serde(rename = "insertChildrenAfter")]
    InsertChildrenAfter {
        pointer: RecordPointer,
        path: &'static [&'static str],
        args: InsertChildrenAfterArgs,
        #[serde(rename = "additionalUpdatedPointers")]
        additional_updated_pointers: Vec<RecordPointer>,
    },
    #[serde(rename = "insertText")]
    InsertText {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: InsertTextArgs,
    },
    #[serde(rename = "deleteText")]
    DeleteText {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: DeleteTextArgs,
    },
    #[serde(rename = "addAnnotation")]
    AddAnnotation {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: AddAnnotationArgs,
    },
    #[serde(rename = "removeAnnotation")]
    RemoveAnnotation {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: RemoveAnnotationArgs,
    },
    #[serde(rename = "splitText")]
    SplitText {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: SplitTextArgs,
    },
    #[serde(rename = "moveTextSlice")]
    MoveTextSlice {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: MoveTextSliceArgs,
    },
    #[serde(rename = "moveTextSliceIntoBlock")]
    MoveTextSliceIntoBlock {
        pointer: RecordPointer,
        path: &'static [&'static str],
        #[serde(rename = "opVersion")]
        op_version: u8,
        args: MoveTextSliceIntoBlockArgs,
    },
}

impl SaveOperation {
    pub(in crate::live::board::page_mutation) fn append_commit_block_ids(
        &self,
        block_ids: &mut Vec<String>,
    ) {
        self.append_block_ids(block_ids, true);
    }

    pub(in crate::live::board::page_mutation) fn append_record_overlay_block_ids(
        &self,
        block_ids: &mut Vec<String>,
    ) {
        self.append_block_ids(block_ids, false);
    }

    fn append_block_ids(&self, block_ids: &mut Vec<String>, include_metadata: bool) {
        match self {
            Self::UpdateBlockPropertyValue {
                pointer,
                additional_updated_pointers,
                ..
            }
            | Self::InsertChildrenAfter {
                pointer,
                additional_updated_pointers,
                ..
            } => {
                append_block_pointer_id(pointer, block_ids);
                for pointer in additional_updated_pointers {
                    append_block_pointer_id(pointer, block_ids);
                }
            }
            Self::SetSimpleTableCell {
                pointer,
                additional_updated_pointers,
                ..
            } => {
                append_block_pointer_id(pointer, block_ids);
                for pointer in additional_updated_pointers {
                    append_block_pointer_id(pointer, block_ids);
                }
            }
            Self::SetPageIcon { pointer, .. } if !include_metadata => {
                append_block_pointer_id(pointer, block_ids);
            }
            Self::SetBlock { pointer, .. }
            | Self::SetCodeBlock { pointer, .. }
            | Self::SetPastedBlock { pointer, .. }
            | Self::UpdateDuplicatedAlias { pointer, .. }
            | Self::UpdateParent { pointer, .. }
            | Self::UpdateType { pointer, .. }
            | Self::UpdateBlockFormat { pointer, .. }
            | Self::SetPageIcon { pointer, .. }
            | Self::SetAlive { pointer, .. }
            | Self::ListAfter { pointer, .. }
            | Self::ListBefore { pointer, .. }
            | Self::ListRemove { pointer, .. }
            | Self::InsertText { pointer, .. }
            | Self::DeleteText { pointer, .. }
            | Self::AddAnnotation { pointer, .. }
            | Self::RemoveAnnotation { pointer, .. }
            | Self::SplitText { pointer, .. }
            | Self::MoveTextSlice { pointer, .. }
            | Self::MoveTextSliceIntoBlock { pointer, .. } => {
                append_block_pointer_id(pointer, block_ids);
            }
            Self::UpdateMetadata { pointer, .. } if include_metadata => {
                append_block_pointer_id(pointer, block_ids);
            }
            Self::UpdateMetadata { .. } => {}
        }
    }
}

fn append_block_pointer_id(pointer: &RecordPointer, block_ids: &mut Vec<String>) {
    if pointer.table == "block" {
        block_ids.push(pointer.id.clone());
    }
}

fn serialize_true<S>(_: &(), serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(true)
}
