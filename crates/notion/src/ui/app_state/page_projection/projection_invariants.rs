use super::{
    identity::card_page_has_column_structure, LoadedCardPageAuthority, LoadedCardPageSection,
    PageFlowNode, PageFlowProjection,
};

pub(super) fn validate_flow_projection(
    page: &crate::ui::CardPage,
    authority: &LoadedCardPageAuthority,
    flow: &PageFlowProjection,
    sections: &[LoadedCardPageSection],
) -> bool {
    let has_column_structure = card_page_has_column_structure(page)
        || matches!(authority, LoadedCardPageAuthority::Diverged(authority)
            if card_page_has_column_structure(authority));
    if !has_column_structure {
        assert_flat_sections_match_root_flow(flow, sections);
    }
    has_column_structure
}

fn assert_flat_sections_match_root_flow(
    flow: &PageFlowProjection,
    sections: &[LoadedCardPageSection],
) {
    assert_eq!(flow.root.nodes.len(), sections.len());
    for (node, section) in flow.root.nodes.iter().zip(sections) {
        let PageFlowNode::Section(flow_section) = node else {
            panic!("a Notion page without column structure must contain only root Sections");
        };
        assert_eq!(flow_section.document_unit_range, section.unit_range);
    }
}
