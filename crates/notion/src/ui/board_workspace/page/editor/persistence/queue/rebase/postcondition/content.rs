use crate::model::{
    CardPageAliasBlock, CardPageBlockContent, CardPageBlockKind, CardPageCodeLanguage,
    CardPageCodeWrap, CardPageLayoutBlock, CardPageQuoteSize, CardPageResourceBlock,
    CardPageSimpleTableBlock, CardPageSimpleTableRowBlock, CardPageStructuralBlock,
    CardPageTextAnnotationSpan, CardPageToDoState, CardPageUnsupportedLeafBlock, PageShellIcon,
};
use crate::ui::board_workspace::PageProjectedText;

use super::effect::{changed, EffectSide, FieldEffect, IconEffect};

type AnnotationSpans = Vec<CardPageTextAnnotationSpan>;

pub(super) struct ContentEffect {
    editable_kind: Option<FieldEffect<Option<CardPageBlockKind>>>,
    text: Option<FieldEffect<Option<String>>>,
    annotations: Option<FieldEffect<Option<AnnotationSpans>>>,
    to_do_state: Option<FieldEffect<Option<CardPageToDoState>>>,
    quote_size: Option<FieldEffect<Option<CardPageQuoteSize>>>,
    code_language: Option<FieldEffect<Option<CardPageCodeLanguage>>>,
    code_wrap: Option<FieldEffect<Option<CardPageCodeWrap>>>,
    alias_target_block_id: Option<FieldEffect<Option<String>>>,
    alias_target_space_id: Option<FieldEffect<Option<String>>>,
    alias_copied_from_block_id: Option<FieldEffect<Option<String>>>,
    alias_title: Option<FieldEffect<Option<String>>>,
    alias_icon: Option<IconEffect>,
    structural: Option<FieldEffect<Option<CardPageStructuralBlock>>>,
    resource: Option<FieldEffect<Option<CardPageResourceBlock>>>,
    unsupported_leaf: Option<FieldEffect<Option<CardPageUnsupportedLeafBlock>>>,
    layout: Option<FieldEffect<Option<CardPageLayoutBlock>>>,
    simple_table: Option<FieldEffect<Option<CardPageSimpleTableBlock>>>,
    simple_table_row: Option<FieldEffect<Option<CardPageSimpleTableRowBlock>>>,
}

struct ContentFields {
    editable_kind: Option<CardPageBlockKind>,
    text: Option<String>,
    annotations: Option<Vec<CardPageTextAnnotationSpan>>,
    to_do_state: Option<CardPageToDoState>,
    quote_size: Option<CardPageQuoteSize>,
    code_language: Option<CardPageCodeLanguage>,
    code_wrap: Option<CardPageCodeWrap>,
    alias_target_block_id: Option<String>,
    alias_target_space_id: Option<String>,
    alias_copied_from_block_id: Option<String>,
    alias_title: Option<String>,
    alias_icon: Option<PageShellIcon>,
    structural: Option<CardPageStructuralBlock>,
    resource: Option<CardPageResourceBlock>,
    unsupported_leaf: Option<CardPageUnsupportedLeafBlock>,
    layout: Option<CardPageLayoutBlock>,
    simple_table: Option<CardPageSimpleTableBlock>,
    simple_table_row: Option<CardPageSimpleTableRowBlock>,
}

impl ContentEffect {
    pub(super) fn between(
        before: &CardPageBlockContent,
        after: &CardPageBlockContent,
    ) -> Option<Self> {
        let before = ContentFields::from_content(before);
        let after = ContentFields::from_content(after);
        let effect = Self {
            editable_kind: changed(&before.editable_kind, &after.editable_kind),
            text: changed(&before.text, &after.text),
            annotations: changed(&before.annotations, &after.annotations),
            to_do_state: changed(&before.to_do_state, &after.to_do_state),
            quote_size: changed(&before.quote_size, &after.quote_size),
            code_language: changed(&before.code_language, &after.code_language),
            code_wrap: changed(&before.code_wrap, &after.code_wrap),
            alias_target_block_id: changed(
                &before.alias_target_block_id,
                &after.alias_target_block_id,
            ),
            alias_target_space_id: changed(
                &before.alias_target_space_id,
                &after.alias_target_space_id,
            ),
            alias_copied_from_block_id: changed(
                &before.alias_copied_from_block_id,
                &after.alias_copied_from_block_id,
            ),
            alias_title: changed(&before.alias_title, &after.alias_title),
            alias_icon: IconEffect::changed(&before.alias_icon, &after.alias_icon),
            structural: changed(&before.structural, &after.structural),
            resource: changed(&before.resource, &after.resource),
            unsupported_leaf: changed(&before.unsupported_leaf, &after.unsupported_leaf),
            layout: changed(&before.layout, &after.layout),
            simple_table: changed(&before.simple_table, &after.simple_table),
            simple_table_row: changed(&before.simple_table_row, &after.simple_table_row),
        };
        effect.has_changes().then_some(effect)
    }

    pub(super) fn matches(&self, actual: &CardPageBlockContent, side: EffectSide) -> bool {
        let actual = ContentFields::from_content(actual);
        matches_field(&self.editable_kind, &actual.editable_kind, side)
            && matches_field(&self.text, &actual.text, side)
            && matches_field(&self.annotations, &actual.annotations, side)
            && matches_field(&self.to_do_state, &actual.to_do_state, side)
            && matches_field(&self.quote_size, &actual.quote_size, side)
            && matches_field(&self.code_language, &actual.code_language, side)
            && matches_field(&self.code_wrap, &actual.code_wrap, side)
            && matches_field(
                &self.alias_target_block_id,
                &actual.alias_target_block_id,
                side,
            )
            && matches_field(
                &self.alias_target_space_id,
                &actual.alias_target_space_id,
                side,
            )
            && matches_field(
                &self.alias_copied_from_block_id,
                &actual.alias_copied_from_block_id,
                side,
            )
            && matches_field(&self.alias_title, &actual.alias_title, side)
            && self
                .alias_icon
                .as_ref()
                .is_none_or(|effect| effect.matches(&actual.alias_icon, side))
            && matches_field(&self.structural, &actual.structural, side)
            && matches_field(&self.resource, &actual.resource, side)
            && matches_field(&self.unsupported_leaf, &actual.unsupported_leaf, side)
            && matches_field(&self.layout, &actual.layout, side)
            && matches_field(&self.simple_table, &actual.simple_table, side)
            && matches_field(&self.simple_table_row, &actual.simple_table_row, side)
    }

    fn has_changes(&self) -> bool {
        self.editable_kind.is_some()
            || self.text.is_some()
            || self.annotations.is_some()
            || self.to_do_state.is_some()
            || self.quote_size.is_some()
            || self.code_language.is_some()
            || self.code_wrap.is_some()
            || self.alias_target_block_id.is_some()
            || self.alias_target_space_id.is_some()
            || self.alias_copied_from_block_id.is_some()
            || self.alias_title.is_some()
            || self.alias_icon.is_some()
            || self.structural.is_some()
            || self.resource.is_some()
            || self.unsupported_leaf.is_some()
            || self.layout.is_some()
            || self.simple_table.is_some()
            || self.simple_table_row.is_some()
    }
}

impl ContentFields {
    fn from_content(content: &CardPageBlockContent) -> Self {
        match content {
            CardPageBlockContent::Editable(editable) => Self::editable(editable),
            CardPageBlockContent::Alias(alias) => Self::alias(alias),
            CardPageBlockContent::Structural(structural) => {
                Self::empty_with_structural(structural.clone())
            }
            CardPageBlockContent::Resource(resource) => Self {
                resource: Some(resource.clone()),
                ..Self::empty()
            },
            CardPageBlockContent::UnsupportedLeaf(unsupported_leaf) => Self {
                unsupported_leaf: Some(unsupported_leaf.clone()),
                ..Self::empty()
            },
            CardPageBlockContent::OpaqueUnavailable { .. } => Self::empty(),
            CardPageBlockContent::Layout(layout) => Self::empty_with_layout(layout.clone()),
            CardPageBlockContent::Table { table } => Self {
                simple_table: Some(table.clone()),
                ..Self::empty()
            },
            CardPageBlockContent::TableRow { table_row } => Self {
                simple_table_row: Some(table_row.clone()),
                ..Self::empty()
            },
        }
    }

    fn editable(editable: &crate::model::CardPageEditableBlock) -> Self {
        let annotations = PageProjectedText::editable("", editable)
            .expect("authoritative page annotations must remain canonicalizable")
            .annotations;
        Self {
            editable_kind: Some(editable.kind),
            text: Some(editable.text.clone()),
            annotations: Some(annotations),
            to_do_state: editable.to_do_state(),
            quote_size: editable.quote_size(),
            code_language: editable.code_language().cloned(),
            code_wrap: editable.code_wrap(),
            ..Self::empty()
        }
    }

    fn alias(alias: &CardPageAliasBlock) -> Self {
        Self {
            alias_target_block_id: Some(alias.target_block_id.clone()),
            alias_target_space_id: Some(alias.target_space_id.clone()),
            alias_copied_from_block_id: alias.copied_from_block_id.clone(),
            alias_title: Some(alias.title.clone()),
            alias_icon: Some(alias.icon.clone()),
            ..Self::empty()
        }
    }

    fn empty_with_structural(structural: CardPageStructuralBlock) -> Self {
        Self {
            structural: Some(structural),
            ..Self::empty()
        }
    }

    fn empty_with_layout(layout: CardPageLayoutBlock) -> Self {
        Self {
            layout: Some(layout),
            ..Self::empty()
        }
    }

    fn empty() -> Self {
        Self {
            editable_kind: None,
            text: None,
            annotations: None,
            to_do_state: None,
            quote_size: None,
            code_language: None,
            code_wrap: None,
            alias_target_block_id: None,
            alias_target_space_id: None,
            alias_copied_from_block_id: None,
            alias_title: None,
            alias_icon: None,
            structural: None,
            resource: None,
            unsupported_leaf: None,
            layout: None,
            simple_table: None,
            simple_table_row: None,
        }
    }
}

fn matches_field<T: Eq>(effect: &Option<FieldEffect<T>>, actual: &T, side: EffectSide) -> bool {
    effect
        .as_ref()
        .is_none_or(|effect| effect.matches(actual, side))
}
