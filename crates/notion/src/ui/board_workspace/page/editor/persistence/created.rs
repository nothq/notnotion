use super::PageMutationPlan;
use std::collections::{HashMap, HashSet};

use super::{
    block_shape::page_block_creation,
    property_groups::{page_block_persists_color, PageBlockPropertyGroups},
    snapshot::PageSnapshotIndex,
};
use crate::model::{
    CreatePageBlockRequest, CreatePageCodeBlockRequest, DuplicatePageAliasRequest,
    PageBlockPlacement, PageMutation, SetPageIconRequest, SetPageToDoStateRequest,
};
use crate::ui::{
    CardPage, CardPageBlock, CardPageBlockColor, CardPageBlockKind, CardPageQuoteSize,
    CardPageToDoState,
};

mod projection;

use projection::PreparedPageBlockCreationPlacement;

pub(super) struct PreparedPageBlockCreations {
    blocks: Vec<PreparedPageBlockCreation>,
}

struct PreparedPageBlockCreation {
    mutation: PageMutation,
    target: CardPageBlock,
    projected: CardPageBlock,
    placement: PreparedPageBlockCreationPlacement,
    followup: CreatedBlockFollowup,
}

#[derive(Clone, Copy)]
enum CreatedBlockFollowup {
    None,
    Metadata,
    AnnotationsAndMetadata,
}

impl PreparedPageBlockCreations {
    pub(super) fn new(index: &PageSnapshotIndex<'_>, target: &CardPage) -> Result<Self, String> {
        let mut blocks = Vec::new();
        let mut seen_ids = HashSet::new();
        let mut prepared_by_id = HashMap::new();
        for block in &target.blocks {
            if block.block_id.is_empty() || block.block_id == target.block_id {
                return Err("page history contains an invalid block identity".to_string());
            }
            if !seen_ids.insert(block.block_id.as_str()) {
                return Err(format!(
                    "page history contains duplicate block {}",
                    block.block_id
                ));
            }
            if index.contains_current(&block.block_id) {
                continue;
            }
            let creation =
                prepare_page_block_creation(index, target, &blocks, &prepared_by_id, block)?;
            prepared_by_id.insert(block.block_id.as_str(), blocks.len());
            blocks.push(creation);
        }
        Ok(Self { blocks })
    }
}

impl PageMutationPlan {
    pub(crate) fn persist_appended_page_block(
        &mut self,
        page_block_id: &str,
        block: &CardPageBlock,
    ) {
        let (kind, text) = page_block_creation(block)
            .expect("appended page composer block must have a verified creation shape");
        self.enqueue_page_mutation(
            page_block_id,
            PageMutation::CreateBlock(CreatePageBlockRequest {
                block_id: block.block_id.clone(),
                parent_block_id: block.parent_block_id.clone(),
                kind,
                text: text.to_string(),
                placement: PageBlockPlacement::Append,
            }),
        );
        self.persist_created_page_block_metadata(page_block_id, std::iter::once(block));
    }

    pub(super) fn persist_prepared_page_block_creations(
        &mut self,
        page_id: &str,
        creations: PreparedPageBlockCreations,
    ) {
        let mut metadata_blocks = Vec::new();
        for creation in creations.blocks {
            self.enqueue_page_mutation(page_id, creation.mutation);
            if matches!(
                creation.followup,
                CreatedBlockFollowup::AnnotationsAndMetadata
            ) {
                self.persist_page_block_annotations(page_id, &creation.target);
            }
            if !matches!(creation.followup, CreatedBlockFollowup::None) {
                metadata_blocks.push(creation.target);
            }
        }
        self.persist_created_page_block_metadata(page_id, metadata_blocks.iter());
    }

    pub(in crate::ui::board_workspace::page::editor) fn persist_created_page_block_metadata<'a>(
        &mut self,
        page_id: &str,
        blocks: impl IntoIterator<Item = &'a CardPageBlock>,
    ) {
        let mut groups = PageBlockPropertyGroups::default();
        for block in blocks {
            let Some(editable) = block.editable_content() else {
                continue;
            };
            if matches!(
                editable.kind,
                CardPageBlockKind::PageLink | CardPageBlockKind::Callout
            ) && block.icon.is_some()
            {
                let request = SetPageIconRequest::new(block.block_id.clone(), block.icon.clone())
                    .expect("created page icon must contain a valid icon");
                self.enqueue_page_mutation(page_id, PageMutation::SetPageIcon(request));
            }
            if editable
                .to_do_state()
                .is_some_and(|state| state.is_checked())
            {
                self.enqueue_page_mutation(
                    page_id,
                    PageMutation::SetToDoState(SetPageToDoStateRequest {
                        block_id: block.block_id.clone(),
                        state: CardPageToDoState::Checked,
                    }),
                );
            }
            if block.color != CardPageBlockColor::default() && page_block_persists_color(block) {
                groups.add_color(block.color, block.block_id.clone());
            }
            if editable.quote_size() == Some(CardPageQuoteSize::Large) {
                groups.add_quote_size(CardPageQuoteSize::Large, block.block_id.clone());
            }
        }
        groups.persist(self, page_id);
    }
}

fn prepare_page_block_creation(
    index: &PageSnapshotIndex<'_>,
    target: &CardPage,
    prepared: &[PreparedPageBlockCreation],
    prepared_by_id: &HashMap<&str, usize>,
    block: &CardPageBlock,
) -> Result<PreparedPageBlockCreation, String> {
    if block.alias_content().is_some() {
        return prepare_alias_creation(index, target, prepared, prepared_by_id, block);
    }
    if block
        .editable_content()
        .is_some_and(|editable| editable.kind == CardPageBlockKind::Code)
    {
        return prepare_code_creation(target, block);
    }
    prepare_generic_creation(index, target, prepared, prepared_by_id, block)
}

fn prepare_alias_creation(
    index: &PageSnapshotIndex<'_>,
    target: &CardPage,
    prepared: &[PreparedPageBlockCreation],
    prepared_by_id: &HashMap<&str, usize>,
    block: &CardPageBlock,
) -> Result<PreparedPageBlockCreation, String> {
    let alias = block
        .alias_content()
        .expect("alias creation parser must receive an alias");
    let source_id = alias.copied_from_block_id.as_ref().ok_or_else(|| {
        format!(
            "history alias {} has no copy provenance and cannot be recreated",
            block.block_id
        )
    })?;
    let source = index
        .current_block(source_id)
        .cloned()
        .or_else(|| prepared_block(prepared, prepared_by_id, source_id).cloned())
        .ok_or_else(|| format!("history alias source {source_id} is unavailable"))?;
    validate_alias_source(index, target, &source)?;
    let mut projected = source;
    projected.block_id.clone_from(&block.block_id);
    projected.last_edited = None;
    projected
        .alias_content_mut()
        .expect("validated alias source must remain an alias")
        .copied_from_block_id = Some(source_id.clone());
    if projected.content != block.content
        || projected.color != block.color
        || projected.icon != block.icon
    {
        return Err(format!(
            "history alias {} differs from its reproducible duplicate",
            block.block_id
        ));
    }
    let request = DuplicatePageAliasRequest::new(source_id.clone(), block.block_id.clone())?;
    Ok(PreparedPageBlockCreation {
        mutation: PageMutation::DuplicateAlias(request),
        target: block.clone(),
        projected,
        placement: PreparedPageBlockCreationPlacement::AfterLeaf(source_id.clone()),
        followup: CreatedBlockFollowup::None,
    })
}

fn validate_alias_source(
    index: &PageSnapshotIndex<'_>,
    target: &CardPage,
    source: &CardPageBlock,
) -> Result<(), String> {
    if source.parent_block_id != target.block_id || source.alias_content().is_none() {
        return Err(format!(
            "history alias source {} is not a root-level alias",
            source.block_id
        ));
    }
    if index.current_block(&source.block_id).is_some()
        && !index.current_children(&source.block_id).is_empty()
    {
        return Err(format!(
            "history alias source {} is not a leaf",
            source.block_id
        ));
    }
    Ok(())
}

fn prepare_code_creation(
    target: &CardPage,
    block: &CardPageBlock,
) -> Result<PreparedPageBlockCreation, String> {
    let editable = block
        .editable_content()
        .expect("Code creation parser must receive an editable block");
    if block.parent_block_id != target.block_id
        || block.depth != 0
        || block.color != CardPageBlockColor::default()
        || block.icon.is_some()
        || !editable.text.is_empty()
        || !editable.annotations.is_empty()
    {
        return Err(format!(
            "history Code block {} is nonempty, nested, or decorated",
            block.block_id
        ));
    }
    let settings = editable.code_settings().ok_or_else(|| {
        format!(
            "history Code block {} does not retain Code settings",
            block.block_id
        )
    })?;
    let request = CreatePageCodeBlockRequest::new(
        block.block_id.clone(),
        block.parent_block_id.clone(),
        PageBlockPlacement::Append,
        settings,
    )?;
    let mut projected = block.clone();
    projected.last_edited = None;
    Ok(PreparedPageBlockCreation {
        mutation: PageMutation::CreateCodeBlock(request),
        target: block.clone(),
        projected,
        placement: PreparedPageBlockCreationPlacement::Append(block.parent_block_id.clone()),
        followup: CreatedBlockFollowup::Metadata,
    })
}

fn prepare_generic_creation(
    index: &PageSnapshotIndex<'_>,
    target: &CardPage,
    prepared: &[PreparedPageBlockCreation],
    prepared_by_id: &HashMap<&str, usize>,
    block: &CardPageBlock,
) -> Result<PreparedPageBlockCreation, String> {
    validate_direct_creation_parent(index, target, prepared, prepared_by_id, block)?;
    let (kind, text) = page_block_creation(block).ok_or_else(|| {
        format!(
            "history block {} has a shape that cannot be recreated",
            block.block_id
        )
    })?;
    validate_generic_creation_metadata(block)?;
    let request = CreatePageBlockRequest {
        block_id: block.block_id.clone(),
        parent_block_id: block.parent_block_id.clone(),
        kind,
        text: text.to_string(),
        placement: PageBlockPlacement::Append,
    };
    let mut projected = block.clone();
    projected.last_edited = None;
    Ok(PreparedPageBlockCreation {
        mutation: PageMutation::CreateBlock(request),
        target: block.clone(),
        projected,
        placement: PreparedPageBlockCreationPlacement::Append(block.parent_block_id.clone()),
        followup: CreatedBlockFollowup::AnnotationsAndMetadata,
    })
}

fn validate_direct_creation_parent(
    index: &PageSnapshotIndex<'_>,
    target: &CardPage,
    prepared: &[PreparedPageBlockCreation],
    prepared_by_id: &HashMap<&str, usize>,
    block: &CardPageBlock,
) -> Result<(), String> {
    if block.parent_block_id == target.block_id {
        return Ok(());
    }
    let target_parent = index.target_block(&block.parent_block_id).ok_or_else(|| {
        format!(
            "history block {} targets missing parent {}",
            block.block_id, block.parent_block_id
        )
    })?;
    if !target_parent.accepts_content_children() {
        return Err(format!(
            "history target parent {} cannot accept content children",
            block.parent_block_id
        ));
    }
    let parent = index
        .current_block(&block.parent_block_id)
        .or_else(|| prepared_block(prepared, prepared_by_id, &block.parent_block_id))
        .ok_or_else(|| {
            format!(
                "history block {} is created before its parent {}",
                block.block_id, block.parent_block_id
            )
        })?;
    if !parent.accepts_content_children() {
        return Err(format!(
            "history creation parent {} cannot accept content children",
            block.parent_block_id
        ));
    }
    Ok(())
}

fn prepared_block<'a>(
    prepared: &'a [PreparedPageBlockCreation],
    prepared_by_id: &HashMap<&str, usize>,
    block_id: &str,
) -> Option<&'a CardPageBlock> {
    prepared_by_id
        .get(block_id)
        .map(|index| &prepared[*index].projected)
}

fn validate_generic_creation_metadata(block: &CardPageBlock) -> Result<(), String> {
    let supports_icon = block.editable_content().is_some_and(|editable| {
        matches!(
            editable.kind,
            CardPageBlockKind::PageLink | CardPageBlockKind::Callout
        )
    });
    if block.icon.is_some() && !supports_icon {
        return Err(format!(
            "history block {} has an icon that cannot be recreated",
            block.block_id
        ));
    }
    if block.color != CardPageBlockColor::default() && !page_block_persists_color(block) {
        return Err(format!(
            "history block {} has a color that cannot be recreated",
            block.block_id
        ));
    }
    Ok(())
}
