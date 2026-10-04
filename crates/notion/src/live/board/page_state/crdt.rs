use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::SerializedTextItemAnnotations;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::live::board) struct SerializedTextSliceTree {
    #[serde(rename = "r")]
    pub(in crate::live::board) root_key: String,
    #[serde(rename = "n")]
    pub(in crate::live::board) nodes: HashMap<String, SerializedTextSliceNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::live::board) struct SerializedTextSliceNode {
    #[serde(rename = "s")]
    pub(in crate::live::board) text_slice: SerializedTextSlice,
    #[serde(rename = "c")]
    pub(in crate::live::board) child_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::live::board) struct SerializedTextSlice {
    #[serde(rename = "x")]
    pub(in crate::live::board) text_instance_id: String,
    #[serde(rename = "i")]
    pub(in crate::live::board) items: Vec<SerializedTextItem>,
    #[serde(rename = "l")]
    pub(in crate::live::board) search_label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t")]
pub(in crate::live::board) enum SerializedTextItem {
    #[serde(rename = "s")]
    Start {
        #[serde(flatten)]
        annotations: SerializedTextItemAnnotations,
    },
    #[serde(rename = "e")]
    End {
        #[serde(flatten)]
        annotations: SerializedTextItemAnnotations,
    },
    #[serde(rename = "p")]
    Split {
        #[serde(rename = "i")]
        id: CrdtOperationId,
        #[serde(rename = "o")]
        origin_id: CrdtItemId,
        #[serde(flatten)]
        annotations: SerializedTextItemAnnotations,
    },
    #[serde(rename = "t")]
    Text {
        #[serde(rename = "i")]
        id: CrdtOperationId,
        #[serde(rename = "o")]
        origin_id: CrdtItemId,
        #[serde(rename = "l")]
        length: i64,
        #[serde(rename = "c", default)]
        content: Option<String>,
        #[serde(flatten)]
        annotations: SerializedTextItemAnnotations,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(in crate::live::board) struct CrdtOperationId(
    pub(in crate::live::board) String,
    pub(in crate::live::board) u64,
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub(in crate::live::board) enum CrdtItemId {
    Boundary(String),
    Operation(CrdtOperationId),
}

impl SerializedTextSliceTree {
    pub(super) fn validate(&self, block_id: &str) -> Result<(), String> {
        let mut visited = HashSet::new();
        self.visit(&self.root_key, &mut visited)?;
        if visited.len() != self.nodes.len() {
            return Err(format!(
                "block {block_id} has disconnected title CRDT nodes"
            ));
        }
        for node in self.nodes.values() {
            node.text_slice.validate(block_id)?;
        }
        Ok(())
    }

    fn visit<'a>(&'a self, key: &'a str, visited: &mut HashSet<&'a str>) -> Result<(), String> {
        if !visited.insert(key) {
            return Err(format!("title CRDT contains a cycle at {key}"));
        }
        let node = self
            .nodes
            .get(key)
            .ok_or_else(|| format!("title CRDT references missing node {key}"))?;
        for child_key in &node.child_keys {
            self.visit(child_key, visited)?;
        }
        Ok(())
    }

    pub(in crate::live::board) fn ordered_keys(&self) -> Vec<&str> {
        let mut keys = Vec::with_capacity(self.nodes.len());
        self.append_ordered_keys(&self.root_key, &mut keys);
        keys
    }

    fn append_ordered_keys<'a>(&'a self, key: &'a str, keys: &mut Vec<&'a str>) {
        keys.push(key);
        for child_key in &self.nodes[key].child_keys {
            self.append_ordered_keys(child_key, keys);
        }
    }

    pub(in crate::live::board) fn node(&self, key: &str) -> &SerializedTextSliceNode {
        &self.nodes[key]
    }

    pub(in crate::live::board) fn following_top_level_keys(
        &self,
        selected_key: &str,
        include_selected_children: bool,
    ) -> Result<Vec<&str>, String> {
        let path = self.path_to(selected_key)?;
        let mut following = Vec::new();
        if include_selected_children {
            following.extend(
                self.nodes[selected_key]
                    .child_keys
                    .iter()
                    .map(String::as_str),
            );
        }
        for pair in path.windows(2).rev() {
            let parent = self.node(pair[0]);
            let child_index = parent
                .child_keys
                .iter()
                .position(|key| key == pair[1])
                .ok_or_else(|| "invalid title CRDT parent path".to_string())?;
            following.extend(
                parent.child_keys[child_index + 1..]
                    .iter()
                    .map(String::as_str),
            );
        }
        Ok(following)
    }

    fn path_to<'a>(&'a self, selected_key: &str) -> Result<Vec<&'a str>, String> {
        let mut path = Vec::new();
        if self.find_path(&self.root_key, selected_key, &mut path) {
            return Ok(path);
        }
        Err(format!("title CRDT does not contain node {selected_key}"))
    }

    fn find_path<'a>(&'a self, key: &'a str, selected_key: &str, path: &mut Vec<&'a str>) -> bool {
        path.push(key);
        if key == selected_key {
            return true;
        }
        for child_key in &self.nodes[key].child_keys {
            if self.find_path(child_key, selected_key, path) {
                return true;
            }
        }
        path.pop();
        false
    }
}

impl SerializedTextSlice {
    fn validate(&self, block_id: &str) -> Result<(), String> {
        if self.items.len() < 2 {
            return Err(format!(
                "block {block_id} has an incomplete title CRDT slice"
            ));
        }
        for item in &self.items {
            if let SerializedTextItem::Text {
                length, content, ..
            } = item
            {
                if *length > 0 {
                    let content = content.as_ref().ok_or_else(|| {
                        format!("block {block_id} has visible CRDT text without content")
                    })?;
                    if content.encode_utf16().count() != *length as usize {
                        return Err(format!(
                            "block {block_id} has inconsistent CRDT text length"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub(in crate::live::board) fn start_item_id(&self) -> Result<CrdtItemId, String> {
        boundary_item_id(
            self.items
                .first()
                .ok_or_else(|| "missing title CRDT start item".to_string())?,
            "start",
        )
    }

    pub(in crate::live::board) fn end_item_id(&self) -> Result<CrdtItemId, String> {
        boundary_item_id(
            self.items
                .last()
                .ok_or_else(|| "missing title CRDT end item".to_string())?,
            "end",
        )
    }
}

fn boundary_item_id(item: &SerializedTextItem, boundary: &str) -> Result<CrdtItemId, String> {
    match item {
        SerializedTextItem::Start { .. } if boundary == "start" => {
            Ok(CrdtItemId::Boundary("start".to_string()))
        }
        SerializedTextItem::End { .. } if boundary == "end" => {
            Ok(CrdtItemId::Boundary("end".to_string()))
        }
        SerializedTextItem::Split { id, .. } => Ok(CrdtItemId::Operation(id.clone())),
        _ => Err(format!("invalid title CRDT {boundary} boundary")),
    }
}
