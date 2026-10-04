use super::super::{
    blocks::{
        create_block_operations, create_code_block_operations, delete_block_operations,
        editable_block, CreateBlockInput, CreateCodeBlockInput, MutationIdentity,
    },
    text::title_plain_text,
    validated_new_block_id, BuiltPageMutation, CrdtClock, PageMutationState,
};
use crate::live::board::page_state::NotionBlockRecord;
use crate::model::{
    CreatePageCodeBlockRequest, NotionPageBlockKind, PageBlockPlacement, PageMutationEffect,
    ReplacePageBlockWithCodeRequest, RestorePageBlockFromCodeRequest,
};

pub(in crate::live::board::page_mutation) fn build_create_code(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &CreatePageCodeBlockRequest,
) -> Result<BuiltPageMutation, String> {
    if request.parent_block_id() != identity.page_block_id {
        return Err("Code append creation requires the page root as parent".to_string());
    }
    let block_id = validated_new_block_id(state, request.block_id())?;
    Ok(BuiltPageMutation {
        operations: create_code_block_operations(
            state,
            identity,
            CreateCodeBlockInput {
                block_id,
                parent_block_id: request.parent_block_id(),
                placement: request.placement(),
                settings: request.settings(),
            },
        )?,
        effect: PageMutationEffect::BlockCreated {
            block_id: block_id.to_string(),
        },
        user_action: "actionRegistry.createInsertAction",
    })
}

pub(in crate::live::board::page_mutation) fn build_replace_block_with_code(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageBlockWithCodeRequest,
) -> Result<BuiltPageMutation, String> {
    let source = code_replacement_source(state, identity, request)?;
    let code_block_id = validated_new_block_id(state, request.code_block_id())?;
    let placement = PageBlockPlacement::After(source.id.clone());
    let mut operations = create_code_block_operations(
        state,
        identity,
        CreateCodeBlockInput {
            block_id: code_block_id,
            parent_block_id: &source.parent_id,
            placement: &placement,
            settings: request.settings(),
        },
    )?;
    operations.extend(delete_block_operations(state, identity, &source.id)?);
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockCreated {
            block_id: code_block_id.to_string(),
        },
        user_action: "actionRegistry.createInsertAction",
    })
}

pub(in crate::live::board::page_mutation) fn build_restore_block_from_code(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &RestorePageBlockFromCodeRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let code = restorable_code_block(state, identity, request)?;
    let source_block_id = validated_new_block_id(state, request.source_block_id())?;
    let placement = PageBlockPlacement::After(code.id.clone());
    let mut operations = create_block_operations(
        state,
        identity,
        CreateBlockInput {
            block_id: source_block_id,
            parent_block_id: &code.parent_id,
            kind: &NotionPageBlockKind::Text,
            text: request.source_text().as_str(),
            placement: &placement,
        },
        clock,
    )?;
    operations.extend(delete_block_operations(state, identity, &code.id)?);
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockCreated {
            block_id: source_block_id.to_string(),
        },
        user_action: "keyboardInputShortcut.undo",
    })
}

fn code_replacement_source<'a>(
    state: &'a PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageBlockWithCodeRequest,
) -> Result<&'a NotionBlockRecord, String> {
    let source = editable_block(state, request.source_block_id())?;
    let source_text = title_plain_text(source.title()?)?;
    if source.id == identity.page_block_id
        || source.parent_table != "block"
        || source.parent_id != identity.page_block_id
        || source.kind != NotionPageBlockKind::Text
        || !source.content_ids.is_empty()
        || source_text != request.source_text().expected_persisted_text()
    {
        return Err(format!(
            "block {} is not the verified leaf Text source for this Code replacement",
            source.id
        ));
    }
    Ok(source)
}

fn restorable_code_block<'a>(
    state: &'a PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &RestorePageBlockFromCodeRequest,
) -> Result<&'a NotionBlockRecord, String> {
    let code = editable_block(state, request.code_block_id())?;
    let valid = code.id != identity.page_block_id
        && code.parent_table == "block"
        && code.parent_id == identity.page_block_id
        && code.kind == NotionPageBlockKind::Code
        && code.content_ids.is_empty()
        && title_plain_text(code.title()?)?.is_empty();
    if !valid {
        return Err(format!(
            "block {} is not a restorable empty leaf Code block",
            code.id
        ));
    }
    Ok(code)
}
