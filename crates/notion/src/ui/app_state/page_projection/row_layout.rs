use std::sync::Arc;

use crate::ui::{CardPage, CardPageBlockKind};

use super::{LoadedCardPageData, LoadedCardPageVisibleRow};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageVisibleRowSpacing {
    top: f32,
    bottom: f32,
}

#[derive(Clone)]
pub(crate) struct PageVisibleRowLayouts {
    spacings: Arc<[Option<PageVisibleRowSpacing>]>,
    nesting_offsets: Arc<[f32]>,
}

impl PageVisibleRowSpacing {
    pub(crate) const fn for_kind(kind: CardPageBlockKind) -> Self {
        match kind {
            CardPageBlockKind::Text => Self::new(6.0, 6.0),
            CardPageBlockKind::SubHeader => Self::new(30.0, 6.0),
            CardPageBlockKind::SubSubHeader => Self::new(26.0, 6.0),
            CardPageBlockKind::Heading3 => Self::new(22.0, 6.0),
            CardPageBlockKind::Heading4 => Self::new(18.0, 6.0),
            CardPageBlockKind::PageLink => Self::new(6.0, 0.0),
            CardPageBlockKind::Callout | CardPageBlockKind::Quote | CardPageBlockKind::Code => {
                Self::new(8.0, 8.0)
            }
            CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList => Self::new(6.0, 6.0),
        }
    }

    pub(crate) const fn new(top: f32, bottom: f32) -> Self {
        Self { top, bottom }
    }

    pub(crate) const fn top(self) -> f32 {
        self.top
    }

    pub(crate) const fn bottom(self) -> f32 {
        self.bottom
    }
}

impl PageVisibleRowLayouts {
    pub(super) fn build(page: &CardPage, rows: &[LoadedCardPageVisibleRow]) -> Self {
        let spacings = rows
            .iter()
            .enumerate()
            .map(|(index, _)| visible_row_spacing(page, rows, index, next_row(rows, index)))
            .collect::<Vec<_>>();
        let mut nesting_offsets = Vec::<f32>::with_capacity(rows.len());
        for row in rows {
            let offset = row.visible_parent_row_index.map_or(0.0, |parent| {
                nesting_offsets[parent]
                    + spacings[parent]
                        .expect("visible Notion parents must retain editable row spacing")
                        .bottom()
            });
            nesting_offsets.push(offset);
        }
        Self {
            spacings: spacings.into(),
            nesting_offsets: nesting_offsets.into(),
        }
    }

    pub(crate) fn spacing(&self, index: usize) -> Option<PageVisibleRowSpacing> {
        self.spacings[index]
    }

    pub(crate) fn nesting_offset(&self, index: usize) -> f32 {
        self.nesting_offsets[index]
    }

    pub(crate) fn nesting_offsets(&self) -> Arc<[f32]> {
        self.nesting_offsets.clone()
    }
}

impl LoadedCardPageData {
    pub(crate) fn visible_row_spacing(&self, index: usize) -> Option<PageVisibleRowSpacing> {
        self.visible_row_layouts.spacing(index)
    }

    pub(crate) fn visible_nesting_offsets(&self) -> Arc<[f32]> {
        self.visible_row_layouts.nesting_offsets()
    }
}

pub(crate) fn visible_row_spacing(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
    index: usize,
    next: Option<usize>,
) -> Option<PageVisibleRowSpacing> {
    let row = rows[index];
    let editable = page.blocks[row.block_index].editable_content()?;
    if !kind_joins_list_run(editable.kind) {
        return Some(PageVisibleRowSpacing::for_kind(editable.kind));
    }
    if row.visual_depth > 0 {
        return Some(PageVisibleRowSpacing::new(2.0, 6.0));
    }
    let next_joins = next.is_some_and(|next| row_joins_list_run(page, &rows[next]));
    Some(PageVisibleRowSpacing::new(
        if row.joins_previous_list_sibling {
            1.0
        } else {
            6.0
        },
        if row.joins_next_list_sibling {
            1.0
        } else if next_joins {
            6.0
        } else {
            12.0
        },
    ))
}

fn next_row(rows: &[LoadedCardPageVisibleRow], index: usize) -> Option<usize> {
    index.checked_add(1).filter(|next| *next < rows.len())
}

fn row_joins_list_run(page: &CardPage, row: &LoadedCardPageVisibleRow) -> bool {
    page.blocks[row.block_index]
        .editable_content()
        .is_some_and(|editable| kind_joins_list_run(editable.kind))
}

const fn kind_joins_list_run(kind: CardPageBlockKind) -> bool {
    matches!(
        kind,
        CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList
    )
}
