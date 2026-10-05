use super::super::super::super::{
    normalize_uuid, optional_string_array, required_string, unwrap_record_value, Map, Value,
};
use super::ParsedSidebarNode;

pub(super) type ParsedChildren = (Vec<ParsedSidebarNode>, Vec<String>, Vec<String>);

/// Walks a sidebar node's content, collecting child nodes and the child blocks
/// that still need to be loaded.
struct ContentTraversal<'a> {
    blocks: &'a Map<String, Value>,
    space_id: &'a str,
    ancestors: &'a mut Vec<String>,
    children: Vec<ParsedSidebarNode>,
    unresolved: Vec<String>,
}

pub(super) fn parse_children(
    parent_block_id: &str,
    block: &Value,
    blocks: &Map<String, Value>,
    space_id: &str,
    ancestors: &mut Vec<String>,
) -> Result<ParsedChildren, String> {
    let mut traversal = ContentTraversal {
        blocks,
        space_id,
        ancestors,
        children: Vec::new(),
        unresolved: Vec::new(),
    };
    let child_block_ids = optional_string_array(block, "content")?;
    for child_block_id in &child_block_ids {
        traversal.parse_content_descendant(child_block_id, parent_block_id)?;
    }
    Ok((traversal.children, child_block_ids, traversal.unresolved))
}

impl ContentTraversal<'_> {
    fn parse_content_descendant(
        &mut self,
        block_id: &str,
        expected_parent_block_id: &str,
    ) -> Result<(), String> {
        let Some(block) = self.blocks.get(block_id).and_then(unwrap_record_value) else {
            self.unresolved.push(block_id.to_string());
            return Ok(());
        };
        if block.get("id").is_none() && block.get("version").is_none() {
            self.unresolved.push(block_id.to_string());
            return Ok(());
        }
        if required_string(block, "parent_table")? != "block"
            || normalize_uuid(required_string(block, "parent_id")?)
                != normalize_uuid(expected_parent_block_id)
        {
            return Ok(());
        }
        let block_type = required_string(block, "type")?;
        if block.get("alive").and_then(Value::as_bool) != Some(true) {
            return Ok(());
        }
        if self.ancestors.iter().any(|ancestor| ancestor == block_id) {
            return Err(format!("cyclic Notion sidebar block tree at {block_id}"));
        }
        if block_type == "transcription" {
            return Ok(());
        }
        if is_sidebar_node_type(block_type) {
            self.children.push(ParsedSidebarNode::parse_tree(
                block_id,
                self.blocks,
                self.space_id,
                self.ancestors,
            )?);
            return Ok(());
        }
        self.ancestors.push(block_id.to_string());
        for descendant_id in optional_string_array(block, "content")? {
            self.parse_content_descendant(&descendant_id, block_id)?;
        }
        self.ancestors.pop();
        Ok(())
    }
}

fn is_sidebar_node_type(block_type: &str) -> bool {
    matches!(
        block_type,
        "page" | "collection_view" | "collection_view_page"
    )
}
