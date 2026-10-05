use crate::ui::CardPageBlock;
use crate::ui::CardPageBlockKind;
use crate::ui::CardPageStructuralBlock;

use super::{LoadedCardPageDocumentUnit, LoadedCardPageSection, LoadedCardPageVisibleRow};

pub(super) const PAGE_SECTION_MAX_UNITS: usize = 11;

pub(crate) fn build_loaded_card_page_sections(
    blocks: &[CardPageBlock],
    visible_rows: &[LoadedCardPageVisibleRow],
    document_units: &[LoadedCardPageDocumentUnit],
) -> Vec<LoadedCardPageSection> {
    let mut sections = Vec::new();
    let mut section_start = 0usize;
    let mut section_units = 0usize;

    for (document_unit_index, document_unit) in document_units.iter().enumerate() {
        let visible_row_index = document_unit.owner_visible_row_index();
        let row = &visible_rows[visible_row_index];
        let block = &blocks[row.block_index];
        if document_unit.is_simple_table_row() || page_row_is_callout_render_unit(row, block) {
            if document_unit_index > section_start {
                sections.push(LoadedCardPageSection {
                    unit_range: section_start..document_unit_index,
                });
            }
            sections.push(LoadedCardPageSection {
                unit_range: document_unit_index..(document_unit_index + 1),
            });
            section_start = document_unit_index + 1;
            section_units = 0;
            continue;
        }
        if document_unit_index > section_start && page_block_starts_new_section(block) {
            sections.push(LoadedCardPageSection {
                unit_range: section_start..document_unit_index,
            });
            section_start = document_unit_index;
            section_units = 0;
        }

        section_units += page_block_section_units(block);
        let is_last_block = document_unit_index + 1 == document_units.len();
        let next_starts_new_section = document_units
            .get(document_unit_index + 1)
            .map(LoadedCardPageDocumentUnit::owner_visible_row_index)
            .map(|visible_row_index| &blocks[visible_rows[visible_row_index].block_index])
            .is_some_and(page_block_starts_new_section);
        let should_break =
            page_block_ends_section(block) || section_units >= PAGE_SECTION_MAX_UNITS;

        if is_last_block || next_starts_new_section || should_break {
            sections.push(LoadedCardPageSection {
                unit_range: section_start..(document_unit_index + 1),
            });
            section_start = document_unit_index + 1;
            section_units = 0;
        }
    }

    sections
}

pub(super) fn page_row_is_callout_render_unit(
    row: &LoadedCardPageVisibleRow,
    block: &CardPageBlock,
) -> bool {
    row.callout_parent_row_index.is_some()
        || block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
}

pub(super) fn page_block_starts_new_section(block: &CardPageBlock) -> bool {
    block.resource_content().is_some()
        || block.editable_content().is_some_and(|editable| {
            matches!(
                editable.kind,
                CardPageBlockKind::ToggleList
                    | CardPageBlockKind::SubHeader
                    | CardPageBlockKind::SubSubHeader
                    | CardPageBlockKind::Heading3
                    | CardPageBlockKind::Heading4
            )
        })
        || matches!(
            block.structural_content(),
            Some(
                CardPageStructuralBlock::CollectionView { .. }
                    | CardPageStructuralBlock::CollectionViewPage { .. }
            )
        )
}

pub(super) fn page_block_ends_section(block: &CardPageBlock) -> bool {
    block.resource_content().is_some()
        || block.alias_content().is_some()
        || block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::PageLink)
        || matches!(
            block.structural_content(),
            Some(
                CardPageStructuralBlock::CollectionView { .. }
                    | CardPageStructuralBlock::CollectionViewPage { .. }
            )
        )
}

pub(super) fn page_block_section_units(block: &CardPageBlock) -> usize {
    let Some(editable) = block.editable_content() else {
        if block.layout_content().is_some() {
            return 0;
        }
        if block.alias_content().is_some() {
            return 3;
        }
        if block.resource_content().is_some() {
            return PAGE_SECTION_MAX_UNITS;
        }
        if block.unsupported_leaf_content().is_some() {
            return 2;
        }
        return match block.structural_content() {
            Some(CardPageStructuralBlock::Divider) => 1,
            Some(
                CardPageStructuralBlock::CollectionView { .. }
                | CardPageStructuralBlock::CollectionViewPage { .. },
            ) => 5,
            None => 0,
        };
    };
    let base_units = match editable.kind {
        CardPageBlockKind::SubHeader => 4,
        CardPageBlockKind::SubSubHeader => 3,
        CardPageBlockKind::Heading3 | CardPageBlockKind::Heading4 => 2,
        CardPageBlockKind::Callout | CardPageBlockKind::PageLink => 3,
        CardPageBlockKind::BulletedList
        | CardPageBlockKind::NumberedList
        | CardPageBlockKind::ToDoList
        | CardPageBlockKind::ToggleList
        | CardPageBlockKind::Quote => 2,
        CardPageBlockKind::Text | CardPageBlockKind::Code => 1,
    };

    base_units + editable.text.len() / 240
}

#[cfg(test)]
mod tests {
    use crate::ui::CardPage;
    use crate::ui::CardPageBlock;
    use crate::ui::CardPageBlockKind;

    use super::super::LoadedCardPage;

    #[gpui::test]
    fn loaded_card_page_groups_body_blocks_into_sections() {
        let page = CardPage {
            block_id: "page".to_string(),
            title: "Test".to_string(),
            status: None,
            properties: Vec::new(),
            discussions: Vec::new(),
            comments_writable: false,
            format: Default::default(),
            blocks: vec![
                CardPageBlock::editable(
                    "heading",
                    "page",
                    0,
                    CardPageBlockKind::SubHeader,
                    "Heading",
                ),
                CardPageBlock::editable(
                    "paragraph-one",
                    "page",
                    0,
                    CardPageBlockKind::Text,
                    "Paragraph one",
                ),
                CardPageBlock::editable(
                    "paragraph-two",
                    "page",
                    0,
                    CardPageBlockKind::Text,
                    "Paragraph two",
                ),
                CardPageBlock::editable(
                    "next-heading",
                    "page",
                    0,
                    CardPageBlockKind::SubSubHeader,
                    "Next",
                ),
                CardPageBlock::editable("item", "page", 0, CardPageBlockKind::BulletedList, "Item"),
            ],
        };

        let loaded = LoadedCardPage::new(page);
        let ranges = loaded
            .data
            .sections
            .iter()
            .map(|section| section.unit_range.clone())
            .collect::<Vec<_>>();

        assert_eq!(ranges, vec![0..3, 3..5]);
    }
}
