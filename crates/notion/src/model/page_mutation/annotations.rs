use serde::{Deserialize, Serialize};

use super::mention::{PageMention, PageMentionKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageTextColor {
    Default,
    Gray,
    Brown,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Red,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PageTextAnnotation {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Link(String),
    TextColor(PageTextColor),
    BackgroundColor(PageTextColor),
    /// An inline mention token. The annotated text is exactly one
    /// [`PAGE_MENTION_TOKEN`](super::PAGE_MENTION_TOKEN) character.
    Mention(PageMention),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageTextAnnotationKind {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Link,
    TextColor,
    BackgroundColor,
    Mention(PageMentionKind),
}

impl PageTextAnnotationKind {
    pub const fn is_mention(self) -> bool {
        matches!(self, Self::Mention(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageTextEditTarget {
    Selection {
        block_id: String,
        start_utf8: usize,
        end_utf8: usize,
    },
    Typing {
        block_id: String,
        offset_utf8: usize,
        text: String,
    },
}

#[derive(Clone, Debug)]
pub struct EditPageBlockTextRequest {
    pub page_block_id: String,
    targets: Vec<PageTextEditTarget>,
    annotation_removals: Vec<PageTextAnnotationKind>,
    annotation_additions: Vec<PageTextAnnotation>,
}

impl PageTextAnnotation {
    pub fn kind(&self) -> PageTextAnnotationKind {
        match self {
            Self::Bold => PageTextAnnotationKind::Bold,
            Self::Italic => PageTextAnnotationKind::Italic,
            Self::Underline => PageTextAnnotationKind::Underline,
            Self::Strike => PageTextAnnotationKind::Strike,
            Self::Code => PageTextAnnotationKind::Code,
            Self::Link(_) => PageTextAnnotationKind::Link,
            Self::TextColor(_) => PageTextAnnotationKind::TextColor,
            Self::BackgroundColor(_) => PageTextAnnotationKind::BackgroundColor,
            Self::Mention(mention) => PageTextAnnotationKind::Mention(mention.kind()),
        }
    }

    pub fn mention(&self) -> Option<&PageMention> {
        match self {
            Self::Mention(mention) => Some(mention),
            _ => None,
        }
    }

    /// Whether adjacent spans with equal annotations may merge into one span.
    /// Mention tokens are individual atoms and never merge, even when two
    /// identical mentions sit next to each other.
    pub const fn merges_with_adjacent(&self) -> bool {
        !matches!(self, Self::Mention(_))
    }
}

impl EditPageBlockTextRequest {
    pub fn new(
        page_block_id: String,
        targets: Vec<PageTextEditTarget>,
        annotation_removals: Vec<PageTextAnnotationKind>,
        annotation_additions: Vec<PageTextAnnotation>,
    ) -> Result<Self, String> {
        validate_targets(&targets, &annotation_removals, &annotation_additions)?;
        validate_unique_removals(&annotation_removals)?;
        validate_unique_additions(&annotation_additions)?;
        if annotation_additions
            .iter()
            .any(|annotation| matches!(annotation, PageTextAnnotation::Link(url) if url.trim().is_empty()))
        {
            return Err("a page-text link annotation requires a URL".to_string());
        }
        if annotation_additions
            .iter()
            .any(|annotation| annotation.kind().is_mention())
            || annotation_removals.iter().any(|kind| kind.is_mention())
        {
            return Err(
                "mention annotations are inserted and updated through their own page mutations"
                    .to_string(),
            );
        }
        Ok(Self {
            page_block_id,
            targets,
            annotation_removals,
            annotation_additions,
        })
    }

    pub fn targets(&self) -> &[PageTextEditTarget] {
        &self.targets
    }

    pub fn annotation_removals(&self) -> &[PageTextAnnotationKind] {
        &self.annotation_removals
    }

    pub fn annotation_additions(&self) -> &[PageTextAnnotation] {
        &self.annotation_additions
    }
}

impl PageTextEditTarget {
    pub fn block_id(&self) -> &str {
        match self {
            Self::Selection { block_id, .. } | Self::Typing { block_id, .. } => block_id,
        }
    }
}

fn validate_targets(
    targets: &[PageTextEditTarget],
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) -> Result<(), String> {
    if targets.is_empty() {
        return Err("a page-text edit requires at least one block target".to_string());
    }
    for (index, target) in targets.iter().enumerate() {
        if target.block_id().trim().is_empty() {
            return Err("a page-text edit target requires a block id".to_string());
        }
        if targets[..index]
            .iter()
            .any(|existing| existing.block_id() == target.block_id())
        {
            return Err(format!(
                "duplicate page-text edit target {}",
                target.block_id()
            ));
        }
    }
    let selections = targets
        .iter()
        .filter(|target| matches!(target, PageTextEditTarget::Selection { .. }))
        .count();
    if selections == targets.len() {
        if removals.len() + additions.len() != 1 && !is_clear_format_action(removals, additions) {
            return Err(
                "a selected page-text edit requires one annotation action or one clear-format action"
                    .to_string(),
            );
        }
    } else if targets.len() != 1 || selections != 0 {
        return Err("page-text typing cannot be batched or mixed with selections".to_string());
    }
    for target in targets {
        validate_target(target)?;
    }
    Ok(())
}

fn is_clear_format_action(
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) -> bool {
    additions == [PageTextAnnotation::TextColor(PageTextColor::Default)]
        && removals.contains(&PageTextAnnotationKind::TextColor)
        && removals.contains(&PageTextAnnotationKind::BackgroundColor)
}

fn validate_target(target: &PageTextEditTarget) -> Result<(), String> {
    match target {
        PageTextEditTarget::Selection {
            start_utf8,
            end_utf8,
            ..
        } => {
            if start_utf8 >= end_utf8 {
                return Err("a page-text annotation selection must be non-empty".to_string());
            }
        }
        PageTextEditTarget::Typing { text, .. } if text.is_empty() => {
            return Err("a page-text typing mutation requires inserted text".to_string());
        }
        PageTextEditTarget::Typing { .. } => {}
    }
    Ok(())
}

fn validate_unique_removals(removals: &[PageTextAnnotationKind]) -> Result<(), String> {
    for (index, kind) in removals.iter().enumerate() {
        if removals[..index].contains(kind) {
            return Err(format!("duplicate page-text annotation removal {kind:?}"));
        }
    }
    Ok(())
}

fn validate_unique_additions(additions: &[PageTextAnnotation]) -> Result<(), String> {
    for (index, annotation) in additions.iter().enumerate() {
        let kind = annotation.kind();
        if additions[..index]
            .iter()
            .any(|existing| existing.kind() == kind)
        {
            return Err(format!("duplicate page-text annotation addition {kind:?}"));
        }
    }
    Ok(())
}
