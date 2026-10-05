use std::collections::{HashMap, HashSet};

use crate::model::{
    CardPage, CardPageBlock, CardPageBlockColor, CardPageBlockContent, PageShellIcon,
};

use super::content::ContentEffect;
use super::order::PageOrderEffect;

pub(super) struct PageEffect {
    title: Option<FieldEffect<String>>,
    blocks: Vec<BlockEffect>,
    orders: Vec<PageOrderEffect>,
}

pub(super) struct FieldEffect<T> {
    before: T,
    after: T,
}

struct BlockEffect {
    block_id: String,
    presence: Option<PresenceEffect>,
    parent: Option<FieldEffect<String>>,
    depth: Option<FieldEffect<usize>>,
    color: Option<FieldEffect<CardPageBlockColor>>,
    icon: Option<IconEffect>,
    content: Option<ContentEffect>,
}

struct PresenceEffect {
    before: PresenceExpectation,
    after: PresenceExpectation,
}

enum PresenceExpectation {
    Absent,
    Present,
    Compatible(Box<BlockSnapshot>),
}

#[derive(Clone)]
struct BlockSnapshot {
    parent: String,
    depth: usize,
    color: CardPageBlockColor,
    icon: Option<PageShellIcon>,
    content: CardPageBlockContent,
}

pub(super) struct IconEffect {
    before: Option<PageShellIcon>,
    after: Option<PageShellIcon>,
}

#[derive(Clone, Copy)]
pub(super) enum EffectSide {
    Before,
    After,
}

impl PageEffect {
    pub(super) fn between(
        before: &CardPage,
        after: &CardPage,
        orders: Vec<PageOrderEffect>,
    ) -> Self {
        Self {
            title: changed(&before.title, &after.title),
            blocks: block_effects(before, after),
            orders,
        }
    }

    pub(super) fn matches(&self, authority: &CardPage, side: EffectSide) -> bool {
        self.title
            .as_ref()
            .is_none_or(|effect| effect.matches(&authority.title, side))
            && self
                .blocks
                .iter()
                .all(|effect| effect.matches(authority, side))
            && self
                .orders
                .iter()
                .all(|effect| effect.matches(authority, matches!(side, EffectSide::After)))
    }
}

impl<T: Eq> FieldEffect<T> {
    pub(super) fn matches(&self, actual: &T, side: EffectSide) -> bool {
        actual == self.value(side)
    }

    fn value(&self, side: EffectSide) -> &T {
        match side {
            EffectSide::Before => &self.before,
            EffectSide::After => &self.after,
        }
    }
}

impl IconEffect {
    pub(super) fn changed(
        before: &Option<PageShellIcon>,
        after: &Option<PageShellIcon>,
    ) -> Option<Self> {
        (!PageShellIcon::persisted_value_eq(before.as_ref(), after.as_ref())).then(|| Self {
            before: before.clone(),
            after: after.clone(),
        })
    }

    pub(super) fn matches(&self, actual: &Option<PageShellIcon>, side: EffectSide) -> bool {
        let expected = match side {
            EffectSide::Before => &self.before,
            EffectSide::After => &self.after,
        };
        PageShellIcon::persisted_value_eq(actual.as_ref(), expected.as_ref())
    }
}

impl BlockEffect {
    fn matches(&self, authority: &CardPage, side: EffectSide) -> bool {
        let block = authority
            .blocks
            .iter()
            .find(|block| block.block_id == self.block_id);
        if let Some(presence) = &self.presence {
            return presence.matches(block, side);
        }
        let Some(block) = block else {
            return false;
        };
        self.matches_present(block, side)
    }

    fn matches_present(&self, block: &CardPageBlock, side: EffectSide) -> bool {
        self.parent
            .as_ref()
            .is_none_or(|effect| effect.matches(&block.parent_block_id, side))
            && self
                .depth
                .as_ref()
                .is_none_or(|effect| effect.matches(&block.depth, side))
            && self
                .color
                .as_ref()
                .is_none_or(|effect| effect.matches(&block.color, side))
            && self
                .icon
                .as_ref()
                .is_none_or(|effect| effect.matches(&block.icon, side))
            && self
                .content
                .as_ref()
                .is_none_or(|effect| effect.matches(&block.content, side))
    }
}

impl PresenceEffect {
    fn matches(&self, actual: Option<&CardPageBlock>, side: EffectSide) -> bool {
        match self.value(side) {
            PresenceExpectation::Absent => actual.is_none(),
            PresenceExpectation::Present => actual.is_some(),
            PresenceExpectation::Compatible(expected) => actual
                .map(BlockSnapshot::from)
                .is_some_and(|actual| actual.eq(expected.as_ref())),
        }
    }

    fn value(&self, side: EffectSide) -> &PresenceExpectation {
        match side {
            EffectSide::Before => &self.before,
            EffectSide::After => &self.after,
        }
    }
}

impl From<&CardPageBlock> for BlockSnapshot {
    fn from(block: &CardPageBlock) -> Self {
        Self {
            parent: block.parent_block_id.clone(),
            depth: block.depth,
            color: block.color,
            icon: block.icon.clone(),
            content: block.content.clone(),
        }
    }
}

impl PartialEq for BlockSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent
            && self.depth == other.depth
            && self.color == other.color
            && PageShellIcon::persisted_value_eq(self.icon.as_ref(), other.icon.as_ref())
            && content_eq(&self.content, &other.content)
    }
}

impl Eq for BlockSnapshot {}

fn block_effects(before: &CardPage, after: &CardPage) -> Vec<BlockEffect> {
    let before_by_id = block_map(before);
    let after_by_id = block_map(after);
    ordered_block_ids(before, after)
        .into_iter()
        .filter_map(|block_id| {
            block_effect(
                block_id,
                before_by_id.get(block_id).copied(),
                after_by_id.get(block_id).copied(),
            )
        })
        .collect()
}

fn block_effect(
    block_id: &str,
    before: Option<&CardPageBlock>,
    after: Option<&CardPageBlock>,
) -> Option<BlockEffect> {
    match (before, after) {
        (Some(before), Some(after)) => existing_block_effect(block_id, before, after),
        (Some(_), None) => Some(presence_block_effect(
            block_id,
            PresenceExpectation::Present,
            PresenceExpectation::Absent,
        )),
        (None, Some(after)) => Some(presence_block_effect(
            block_id,
            PresenceExpectation::Absent,
            PresenceExpectation::Compatible(Box::new(BlockSnapshot::from(after))),
        )),
        (None, None) => None,
    }
}

fn presence_block_effect(
    block_id: &str,
    before: PresenceExpectation,
    after: PresenceExpectation,
) -> BlockEffect {
    BlockEffect {
        block_id: block_id.to_string(),
        presence: Some(PresenceEffect { before, after }),
        parent: None,
        depth: None,
        color: None,
        icon: None,
        content: None,
    }
}

fn existing_block_effect(
    block_id: &str,
    before: &CardPageBlock,
    after: &CardPageBlock,
) -> Option<BlockEffect> {
    let effect = BlockEffect {
        block_id: block_id.to_string(),
        presence: None,
        parent: changed(&before.parent_block_id, &after.parent_block_id),
        depth: changed(&before.depth, &after.depth),
        color: changed(&before.color, &after.color),
        icon: IconEffect::changed(&before.icon, &after.icon),
        content: ContentEffect::between(&before.content, &after.content),
    };
    (effect.parent.is_some()
        || effect.depth.is_some()
        || effect.color.is_some()
        || effect.icon.is_some()
        || effect.content.is_some())
    .then_some(effect)
}

fn block_map(page: &CardPage) -> HashMap<&str, &CardPageBlock> {
    page.blocks
        .iter()
        .map(|block| (block.block_id.as_str(), block))
        .collect()
}

fn ordered_block_ids<'a>(before: &'a CardPage, after: &'a CardPage) -> Vec<&'a str> {
    let mut ids = Vec::with_capacity(before.blocks.len().max(after.blocks.len()));
    let mut seen = HashSet::new();
    for block in before.blocks.iter().chain(&after.blocks) {
        if seen.insert(block.block_id.as_str()) {
            ids.push(block.block_id.as_str());
        }
    }
    ids
}

pub(super) fn changed<T: Clone + Eq>(before: &T, after: &T) -> Option<FieldEffect<T>> {
    (before != after).then(|| FieldEffect {
        before: before.clone(),
        after: after.clone(),
    })
}

fn content_eq(left: &CardPageBlockContent, right: &CardPageBlockContent) -> bool {
    match (left, right) {
        (CardPageBlockContent::Alias(left), CardPageBlockContent::Alias(right)) => {
            left.target_block_id == right.target_block_id
                && left.target_space_id == right.target_space_id
                && left.copied_from_block_id == right.copied_from_block_id
                && left.title == right.title
                && PageShellIcon::persisted_value_eq(Some(&left.icon), Some(&right.icon))
        }
        _ => left == right,
    }
}
