use super::{
    editable_block, metadata_operation, BlockColorArgs, BlockPropertyPrimitiveArgs,
    BlockPropertyPrimitiveOperation, BlockPropertyValueArgs, BuiltPageMutation,
    CodeLanguagePropertyArgs, CodeWrapArgs, MutationIdentity, NotionPageBlockKind,
    PageMutationEffect, PageMutationState, QuoteSizeArgs, RecordPointer, SaveOperation,
    SetPageBlockColorRequest, SetPageCodeLanguageRequest, SetPageCodeWrapRequest,
    SetPageIconRequest, SetPageQuoteSizeRequest, SetPageToDoStateRequest, ToDoCheckedPropertyArgs,
};

pub(super) fn build_set_to_do_state(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageToDoStateRequest,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, &request.block_id)?;
    if block.id == identity.page_block_id || block.kind != NotionPageBlockKind::ToDo {
        return Err(format!(
            "block {} is not a mutable Notion to-do block",
            request.block_id
        ));
    }
    let pointer = RecordPointer::block(&block.id, identity.space_id);
    let checked = if request.state.is_checked() {
        "Yes"
    } else {
        "No"
    };
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::update_block_property_value(
                pointer.clone(),
                BlockPropertyValueArgs {
                    primitive_op: BlockPropertyPrimitiveOperation {
                        command: "update",
                        args: BlockPropertyPrimitiveArgs::ToDo(ToDoCheckedPropertyArgs {
                            checked: [[checked]],
                        }),
                    },
                },
                vec![pointer],
            ),
            metadata_operation(identity, &block.id),
            metadata_operation(identity, identity.page_block_id),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "TodoBlock.toggleChecked",
    })
}

pub(super) fn build_set_code_language(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageCodeLanguageRequest,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, request.block_id())?;
    if block.id == identity.page_block_id || block.kind != NotionPageBlockKind::Code {
        return Err(format!(
            "block {} is not a mutable Notion code block",
            request.block_id()
        ));
    }
    let pointer = RecordPointer::block(&block.id, identity.space_id);
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::update_block_property_value(
                pointer.clone(),
                BlockPropertyValueArgs {
                    primitive_op: BlockPropertyPrimitiveOperation {
                        command: "update",
                        args: BlockPropertyPrimitiveArgs::CodeLanguage(CodeLanguagePropertyArgs {
                            language: [[request.language().as_str().to_string()]],
                        }),
                    },
                },
                vec![pointer],
            ),
            metadata_operation(identity, &block.id),
            metadata_operation(identity, identity.page_block_id),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "CodeBlock.setPersistedLanguage",
    })
}

pub(super) fn build_set_code_wrap(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageCodeWrapRequest,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, request.block_id())?;
    if block.id == identity.page_block_id || block.kind != NotionPageBlockKind::Code {
        return Err(format!(
            "block {} is not a mutable Notion code block",
            request.block_id()
        ));
    }
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::update_code_wrap(
                RecordPointer::block(&block.id, identity.space_id),
                CodeWrapArgs {
                    code_wrap: request.wrap().is_enabled(),
                },
            ),
            metadata_operation(identity, &block.id),
            metadata_operation(identity, identity.page_block_id),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "actionRegistry.createToggleAction",
    })
}

pub(super) fn build_set_block_color(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageBlockColorRequest,
) -> Result<BuiltPageMutation, String> {
    let mut operations = Vec::with_capacity(request.block_ids().len() * 2 + 1);
    for block_id in request.block_ids() {
        let block = editable_block(state, block_id)?;
        if block.id == identity.page_block_id || !block_kind_supports_color(&block.kind) {
            return Err(format!(
                "block {block_id} does not support a verified Notion block color mutation"
            ));
        }
        operations.push(SaveOperation::update_block_color(
            RecordPointer::block(&block.id, identity.space_id),
            BlockColorArgs {
                block_color: request.color().api_value(),
            },
        ));
        operations.push(metadata_operation(identity, &block.id));
    }
    operations.push(metadata_operation(identity, identity.page_block_id));
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "actionRegistry.createBlockColorAction",
    })
}

pub(super) fn build_set_page_icon(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageIconRequest,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, request.block_id())?;
    if block.id == identity.page_block_id
        || !matches!(
            &block.kind,
            NotionPageBlockKind::Page
                | NotionPageBlockKind::LinkToPage
                | NotionPageBlockKind::Callout
        )
    {
        return Err(format!(
            "block {} is not a mutable Notion icon block",
            request.block_id()
        ));
    }
    let icon = request.icon().map(page_icon_wire_value).transpose()?;
    let pointer = RecordPointer::block(&block.id, identity.space_id);
    let user_action = if block.kind == NotionPageBlockKind::Callout {
        "CalloutBlock.handleEmojiChange"
    } else {
        "PageIcon.handleChange"
    };
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::set_page_icon(pointer, icon),
            metadata_operation(identity, &block.id),
            metadata_operation(identity, identity.page_block_id),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action,
    })
}

fn page_icon_wire_value(icon: &crate::model::PageShellIcon) -> Result<String, String> {
    if icon.value.trim().is_empty() {
        return Err("a page-icon mutation requires a non-empty icon value".to_string());
    }
    match icon.kind.as_str() {
        "emoji" => Ok(icon.value.clone()),
        "named" => named_icon_wire_value(icon),
        "external" => {
            icon.validate_external()?;
            Ok(icon.value.clone())
        }
        "custom" => {
            icon.validate_custom()?;
            Ok(icon.value.clone())
        }
        kind => Err(format!("unsupported Notion page icon kind {kind}")),
    }
}

fn named_icon_wire_value(icon: &crate::model::PageShellIcon) -> Result<String, String> {
    if let Some(path) = icon.value.strip_prefix("/icons/") {
        let name = path
            .strip_suffix(".svg")
            .ok_or_else(|| format!("invalid named Notion page icon path {}", icon.value))?;
        if named_icon_name_is_valid(name) && named_icon_has_color_suffix(name) {
            return Ok(icon.value.clone());
        }
        return Err(format!(
            "invalid named Notion page icon path {}",
            icon.value
        ));
    }
    if !named_icon_name_is_valid(&icon.value) {
        return Err(format!("invalid named Notion page icon {}", icon.value));
    }
    if named_icon_has_color_suffix(&icon.value) {
        Ok(format!("/icons/{}.svg", icon.value))
    } else {
        Ok(format!("/icons/{}_gray.svg", icon.value))
    }
}

fn named_icon_name_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn named_icon_has_color_suffix(value: &str) -> bool {
    let Some((name, color)) = value.rsplit_once('_') else {
        return false;
    };
    !name.is_empty()
        && matches!(
            color,
            "gray"
                | "lightgray"
                | "brown"
                | "orange"
                | "yellow"
                | "green"
                | "blue"
                | "purple"
                | "pink"
                | "red"
        )
}

pub(super) fn build_set_quote_size(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SetPageQuoteSizeRequest,
) -> Result<BuiltPageMutation, String> {
    let mut operations = Vec::with_capacity(request.block_ids().len() * 2 + 1);
    for block_id in request.block_ids() {
        let block = editable_block(state, block_id)?;
        if block.id == identity.page_block_id || block.kind != NotionPageBlockKind::Quote {
            return Err(format!(
                "block {block_id} is not a mutable Notion quote block"
            ));
        }
        operations.push(SaveOperation::update_quote_size(
            RecordPointer::block(&block.id, identity.space_id),
            QuoteSizeArgs {
                quote_size: request.size().api_value(),
            },
        ));
        operations.push(metadata_operation(identity, &block.id));
    }
    operations.push(metadata_operation(identity, identity.page_block_id));
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "actionRegistry.createQuoteSizeAction",
    })
}

fn block_kind_supports_color(kind: &NotionPageBlockKind) -> bool {
    matches!(
        kind,
        NotionPageBlockKind::Text
            | NotionPageBlockKind::Header
            | NotionPageBlockKind::SubHeader
            | NotionPageBlockKind::SubSubHeader
            | NotionPageBlockKind::Header4
            | NotionPageBlockKind::BulletedList
            | NotionPageBlockKind::NumberedList
            | NotionPageBlockKind::ToDo
            | NotionPageBlockKind::Toggle
            | NotionPageBlockKind::Callout
            | NotionPageBlockKind::Quote
            | NotionPageBlockKind::Alias
    )
}
