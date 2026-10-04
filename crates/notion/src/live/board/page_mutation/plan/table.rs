use super::{metadata_operation, BuiltPageMutation, MutationIdentity, PageMutationEffect};
use crate::live::board::page_mutation::wire::{
    RecordPointer, SaveOperation, SimpleTableCellPropertyPath, SimpleTableCellPropertyValueArgs,
};
use crate::live::board::page_state::PageMutationState;
use crate::model::ReplacePageSimpleTableCellRequest;

pub(super) fn build_replace_simple_table_cell(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageSimpleTableCellRequest,
) -> Result<BuiltPageMutation, String> {
    let resolved = state.simple_table_cell(request.target())?;
    let row_pointer = RecordPointer::block(&resolved.row.id, identity.space_id);
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::set_simple_table_cell(
                row_pointer,
                SimpleTableCellPropertyPath::new(request.target().column_id()),
                SimpleTableCellPropertyValueArgs::new(request.cell()),
                resolved.row.version,
            ),
            metadata_operation(identity, &resolved.row.id),
            metadata_operation(identity, identity.page_block_id),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "Text.handleMutation",
    })
}
