use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use crate::model::{
    CardPage, CardPageBlock, CardPageEditableBlock, CardPageSimpleTableCell,
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellRoundTrip, CardPageTextAnnotationSpan,
    CardPageWritableSimpleTableCell, PageTextAnnotation, PageTextColor,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum PageTextProjectionTarget {
    Title,
    Block(String),
    SimpleTableCell(CardPageSimpleTableCellAddress),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageProjectedText {
    pub(crate) target: PageTextProjectionTarget,
    pub(crate) text: String,
    pub(crate) annotations: Vec<CardPageTextAnnotationSpan>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PageWriteTextProjection {
    pub(crate) updates: Vec<PageProjectedText>,
    pub(crate) retirements: Vec<PageTextProjectionRetirement>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct PageTextProjectionRetirement {
    block_id: String,
}

impl PageProjectedText {
    pub(crate) fn title(text: impl Into<String>) -> Self {
        Self {
            target: PageTextProjectionTarget::Title,
            text: text.into(),
            annotations: Vec::new(),
        }
    }

    pub(crate) fn block(block: &CardPageBlock) -> Option<Self> {
        Self::editable(block.block_id.clone(), block.editable_content()?)
    }

    pub(crate) fn editable(
        block_id: impl Into<String>,
        editable: &CardPageEditableBlock,
    ) -> Option<Self> {
        Self::new(
            PageTextProjectionTarget::Block(block_id.into()),
            editable.text.clone(),
            editable.annotations.clone(),
        )
    }

    pub(crate) fn plain_block(block_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            target: PageTextProjectionTarget::Block(block_id.into()),
            text: text.into(),
            annotations: Vec::new(),
        }
    }

    pub(crate) fn simple_table_cell(
        target: CardPageSimpleTableCellAddress,
        cell: &CardPageWritableSimpleTableCell,
    ) -> Option<Self> {
        let cell = cell.as_cell();
        Self::new(
            PageTextProjectionTarget::SimpleTableCell(target),
            cell.text().to_string(),
            cell.annotations().to_vec(),
        )
    }

    fn new(
        target: PageTextProjectionTarget,
        text: String,
        annotations: Vec<CardPageTextAnnotationSpan>,
    ) -> Option<Self> {
        let annotations = canonical_annotations(&text, annotations)?;
        Some(Self {
            target,
            text,
            annotations,
        })
    }

    pub(crate) fn matches_editable(&self, editable: &CardPageEditableBlock) -> bool {
        self.text == editable.text
            && canonical_annotations(&editable.text, editable.annotations.clone())
                .is_some_and(|annotations| self.annotations == annotations)
    }

    pub(crate) fn matches_simple_table_cell(&self, cell: &CardPageSimpleTableCell) -> bool {
        matches!(
            cell.round_trip(),
            CardPageSimpleTableCellRoundTrip::Writable
        ) && self.text == cell.text()
            && canonical_annotations(cell.text(), cell.annotations().to_vec())
                .is_some_and(|annotations| self.annotations == annotations)
    }
}

impl PageWriteTextProjection {
    pub(crate) fn update(update: PageProjectedText) -> Self {
        Self {
            updates: vec![update],
            retirements: Vec::new(),
        }
    }

    pub(crate) fn updates(updates: impl IntoIterator<Item = PageProjectedText>) -> Self {
        Self {
            updates: deduplicate_updates(updates),
            retirements: Vec::new(),
        }
    }

    pub(crate) fn with_retired_blocks(
        mut self,
        block_ids: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut seen = self.retirements.iter().cloned().collect::<HashSet<_>>();
        self.retirements.extend(
            block_ids
                .into_iter()
                .map(PageTextProjectionRetirement::block)
                .filter(|retirement| seen.insert(retirement.clone())),
        );
        self
    }

    pub(crate) fn from_page_blocks<'a>(
        blocks: impl IntoIterator<Item = &'a CardPageBlock>,
    ) -> Self {
        Self::updates(blocks.into_iter().map(|block| {
            PageProjectedText::block(block)
                .expect("projected structural write target must be an editable block")
        }))
    }

    pub(crate) fn from_page_targets(page: &CardPage, block_ids: &[String]) -> Self {
        let blocks = page
            .blocks
            .iter()
            .map(|block| (block.block_id.as_str(), block))
            .collect::<HashMap<_, _>>();
        Self::from_page_blocks(block_ids.iter().map(|block_id| {
            blocks
                .get(block_id.as_str())
                .copied()
                .expect("projected structural write target must exist in its prepared page")
        }))
    }
}

impl PageTextProjectionRetirement {
    fn block(block_id: String) -> Self {
        Self { block_id }
    }

    pub(crate) fn into_block_id(self) -> String {
        self.block_id
    }
}

fn deduplicate_updates(
    updates: impl IntoIterator<Item = PageProjectedText>,
) -> Vec<PageProjectedText> {
    let mut unique = Vec::<PageProjectedText>::new();
    let mut indices = HashMap::<PageTextProjectionTarget, usize>::new();
    for update in updates {
        if let Some(index) = indices.get(&update.target).copied() {
            unique[index] = update;
        } else {
            indices.insert(update.target.clone(), unique.len());
            unique.push(update);
        }
    }
    unique
}

fn canonical_annotations(
    text: &str,
    mut spans: Vec<CardPageTextAnnotationSpan>,
) -> Option<Vec<CardPageTextAnnotationSpan>> {
    if spans.iter().any(|span| {
        span.start_utf8 >= span.end_utf8
            || span.end_utf8 > text.len()
            || !text.is_char_boundary(span.start_utf8)
            || !text.is_char_boundary(span.end_utf8)
    }) {
        return None;
    }
    spans.sort_by(annotation_then_range);
    let mut merged: Vec<CardPageTextAnnotationSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(previous) = merged.last_mut().filter(|previous| {
            previous.annotation == span.annotation
                && previous.end_utf8 >= span.start_utf8
                && span.annotation.merges_with_adjacent()
        }) {
            previous.end_utf8 = previous.end_utf8.max(span.end_utf8);
        } else {
            merged.push(span);
        }
    }
    merged.sort_by(range_then_annotation);
    Some(merged)
}

fn annotation_then_range(
    left: &CardPageTextAnnotationSpan,
    right: &CardPageTextAnnotationSpan,
) -> Ordering {
    annotation_key(&left.annotation)
        .cmp(&annotation_key(&right.annotation))
        .then(left.start_utf8.cmp(&right.start_utf8))
        .then(left.end_utf8.cmp(&right.end_utf8))
}

fn range_then_annotation(
    left: &CardPageTextAnnotationSpan,
    right: &CardPageTextAnnotationSpan,
) -> Ordering {
    left.start_utf8
        .cmp(&right.start_utf8)
        .then(left.end_utf8.cmp(&right.end_utf8))
        .then(annotation_key(&left.annotation).cmp(&annotation_key(&right.annotation)))
}

/// Sort key: annotation kind rank, link target, then color or mention kind.
type AnnotationKey<'a> = (u8, Option<&'a str>, Option<u8>);

fn annotation_key(annotation: &PageTextAnnotation) -> AnnotationKey<'_> {
    match annotation {
        PageTextAnnotation::Bold => (0, None, None),
        PageTextAnnotation::Italic => (1, None, None),
        PageTextAnnotation::Underline => (2, None, None),
        PageTextAnnotation::Strike => (3, None, None),
        PageTextAnnotation::Code => (4, None, None),
        PageTextAnnotation::Link(value) => (5, Some(value), None),
        PageTextAnnotation::TextColor(color) => (6, None, Some(color_key(*color))),
        PageTextAnnotation::BackgroundColor(color) => (7, None, Some(color_key(*color))),
        PageTextAnnotation::Mention(mention) => (8, None, Some(mention.kind() as u8)),
    }
}

fn color_key(color: PageTextColor) -> u8 {
    match color {
        PageTextColor::Default => 0,
        PageTextColor::Gray => 1,
        PageTextColor::Brown => 2,
        PageTextColor::Orange => 3,
        PageTextColor::Yellow => 4,
        PageTextColor::Green => 5,
        PageTextColor::Blue => 6,
        PageTextColor::Purple => 7,
        PageTextColor::Pink => 8,
        PageTextColor::Red => 9,
    }
}
