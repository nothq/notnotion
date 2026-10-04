use super::{
    AddAnnotationArgs, AliasCopyFormatArgs, AliveArgs, BlockColorArgs, BlockFormatArgs,
    BlockPropertyValueArgs, BlockTypeArgs, CodeWrapArgs, DeleteTextArgs, DuplicatedAliasRecordArgs,
    EditedMetadataArgs, EmptyBlockFormatArgs, EmptyBlockPropertyValueExpectedVersions,
    InsertChildrenAfterArgs, InsertTextArgs, ListAfterArgs, ListBeforeArgs, ListRemoveArgs,
    MoveTextSliceArgs, MoveTextSliceIntoBlockArgs, NewBlockArgs, NewCodeBlockArgs, ParentArgs,
    PastedBlockArgs, QuoteSizeArgs, RecordPointer, RemoveAnnotationArgs, SaveOperation,
    SimpleTableCellPropertyPath, SimpleTableCellPropertyValueArgs, SplitTextArgs, CONTENT_PATH,
    FORMAT_PATH, PAGE_ICON_PATH, PROPERTIES_PATH, ROOT_PATH,
};
use crate::model::CardPageColumnRatio;

impl SaveOperation {
    pub(in crate::live::board::page_mutation) fn set_block(
        pointer: RecordPointer,
        args: NewBlockArgs,
    ) -> Self {
        Self::SetBlock {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn set_code_block(
        pointer: RecordPointer,
        args: NewCodeBlockArgs,
    ) -> Self {
        Self::SetCodeBlock {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn set_pasted_block(
        pointer: RecordPointer,
        args: PastedBlockArgs,
    ) -> Self {
        Self::SetPastedBlock {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn update_duplicated_alias(
        pointer: RecordPointer,
        args: DuplicatedAliasRecordArgs,
    ) -> Self {
        Self::UpdateDuplicatedAlias {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn update_parent(
        pointer: RecordPointer,
        args: ParentArgs,
    ) -> Self {
        Self::UpdateParent {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn update_type(
        pointer: RecordPointer,
        args: BlockTypeArgs,
    ) -> Self {
        Self::UpdateType {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn update_block_property_value(
        pointer: RecordPointer,
        args: BlockPropertyValueArgs,
        additional_updated_pointers: Vec<RecordPointer>,
    ) -> Self {
        Self::UpdateBlockPropertyValue {
            pointer,
            path: PROPERTIES_PATH,
            args,
            block_property_value_expected_versions: EmptyBlockPropertyValueExpectedVersions {},
            additional_updated_pointers,
        }
    }

    pub(in crate::live::board::page_mutation) fn set_simple_table_cell(
        pointer: RecordPointer,
        path: SimpleTableCellPropertyPath,
        args: SimpleTableCellPropertyValueArgs,
        expected_version: u64,
    ) -> Self {
        Self::SetSimpleTableCell {
            additional_updated_pointers: [pointer.clone()],
            pointer,
            path,
            args,
            expected_version,
            only_log_version_mismatch: (),
            block_property_value_expected_versions: EmptyBlockPropertyValueExpectedVersions {},
        }
    }

    pub(in crate::live::board::page_mutation) fn update_block_color(
        pointer: RecordPointer,
        args: BlockColorArgs,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::Color(args),
        }
    }

    pub(in crate::live::board::page_mutation) fn update_quote_size(
        pointer: RecordPointer,
        args: QuoteSizeArgs,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::QuoteSize(args),
        }
    }

    pub(in crate::live::board::page_mutation) fn update_code_wrap(
        pointer: RecordPointer,
        args: CodeWrapArgs,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::CodeWrap(args),
        }
    }

    pub(in crate::live::board::page_mutation) fn update_column_ratio(
        pointer: RecordPointer,
        ratio: CardPageColumnRatio,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::ColumnRatio {
                column_ratio: ratio.fraction(),
            },
        }
    }

    pub(in crate::live::board::page_mutation) fn update_alias_copy_format(
        pointer: RecordPointer,
        args: AliasCopyFormatArgs,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::AliasCopy(args),
        }
    }

    pub(in crate::live::board::page_mutation) fn update_empty_block_format(
        pointer: RecordPointer,
    ) -> Self {
        Self::UpdateBlockFormat {
            pointer,
            path: FORMAT_PATH,
            args: BlockFormatArgs::Empty(EmptyBlockFormatArgs {}),
        }
    }

    pub(in crate::live::board::page_mutation) fn set_page_icon(
        pointer: RecordPointer,
        icon: Option<String>,
    ) -> Self {
        Self::SetPageIcon {
            pointer,
            path: PAGE_ICON_PATH,
            args: icon,
        }
    }

    pub(in crate::live::board::page_mutation) fn update_metadata(
        pointer: RecordPointer,
        args: EditedMetadataArgs,
    ) -> Self {
        Self::UpdateMetadata {
            pointer,
            path: ROOT_PATH,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn set_alive(
        pointer: RecordPointer,
        alive: bool,
    ) -> Self {
        Self::SetAlive {
            pointer,
            path: ROOT_PATH,
            args: AliveArgs { alive },
        }
    }

    pub(in crate::live::board::page_mutation) fn list_after(
        pointer: RecordPointer,
        id: String,
        after: Option<String>,
    ) -> Self {
        Self::ListAfter {
            pointer,
            path: CONTENT_PATH,
            args: ListAfterArgs { id, after },
        }
    }

    pub(in crate::live::board::page_mutation) fn list_before(
        pointer: RecordPointer,
        id: String,
        before: String,
    ) -> Self {
        Self::ListBefore {
            pointer,
            path: CONTENT_PATH,
            args: ListBeforeArgs { id, before },
        }
    }

    pub(in crate::live::board::page_mutation) fn list_remove(
        pointer: RecordPointer,
        id: String,
    ) -> Self {
        Self::ListRemove {
            pointer,
            path: CONTENT_PATH,
            args: ListRemoveArgs { id },
        }
    }

    pub(in crate::live::board::page_mutation) fn insert_children_after(
        pointer: RecordPointer,
        ids: Vec<String>,
        after: Option<String>,
        additional_updated_pointers: Vec<RecordPointer>,
    ) -> Self {
        Self::InsertChildrenAfter {
            pointer,
            path: CONTENT_PATH,
            args: InsertChildrenAfterArgs { ids, after },
            additional_updated_pointers,
        }
    }

    pub(in crate::live::board::page_mutation) fn insert_text(
        pointer: RecordPointer,
        args: InsertTextArgs,
    ) -> Self {
        Self::InsertText {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn delete_text(
        pointer: RecordPointer,
        args: DeleteTextArgs,
    ) -> Self {
        Self::DeleteText {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn add_annotation(
        pointer: RecordPointer,
        args: AddAnnotationArgs,
    ) -> Self {
        Self::AddAnnotation {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn remove_annotation(
        pointer: RecordPointer,
        args: RemoveAnnotationArgs,
    ) -> Self {
        Self::RemoveAnnotation {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn split_text(
        pointer: RecordPointer,
        args: SplitTextArgs,
    ) -> Self {
        Self::SplitText {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn move_text_slice(
        pointer: RecordPointer,
        args: MoveTextSliceArgs,
    ) -> Self {
        Self::MoveTextSlice {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }

    pub(in crate::live::board::page_mutation) fn move_text_slice_into_block(
        pointer: RecordPointer,
        args: MoveTextSliceIntoBlockArgs,
    ) -> Self {
        Self::MoveTextSliceIntoBlock {
            pointer,
            path: ROOT_PATH,
            op_version: 2,
            args,
        }
    }
}
