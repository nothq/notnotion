mod icon;
mod traversal;

use serde::{Deserialize, Serialize};

use super::super::super::{
    block_value, collection_entry, collection_name, normalize_uuid, required_string,
    title_property, BoardTarget, Map, Value,
};
use crate::model::{
    PageShellIcon, PageShellNodeIdentity, PageShellSidebarItem, PageShellSidebarSection,
    PageShellSidebarSectionIdentity,
};
pub(in crate::live::board) use icon::explicit_page_shell_icon;
pub(super) use icon::named_page_shell_icon;
pub(in crate::live::board::load) use icon::page_shell_icon;
use traversal::parse_children;

pub(super) struct ParsedSidebarSection {
    identity: PageShellSidebarSectionIdentity,
    title: String,
    icon: PageShellIcon,
    nodes: Vec<ParsedSidebarNode>,
}

pub(super) struct ParsedSidebarNode {
    block_id: String,
    kind: ParsedSidebarNodeKind,
    title: ParsedSidebarNodeTitle,
    icon: PageShellIcon,
    children: Vec<ParsedSidebarNode>,
    child_block_ids: Vec<String>,
    unresolved_child_block_ids: Vec<String>,
}

enum ParsedSidebarNodeKind {
    Page,
    Database,
}

enum ParsedSidebarNodeTitle {
    Titled(String),
    Untitled,
    Collections(Vec<CollectionPointer>),
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CollectionPointer {
    id: String,
    table: CollectionTable,
    space_id: String,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum CollectionTable {
    #[serde(rename = "collection")]
    Collection,
}

pub(super) struct SidebarSectionHeader<'a> {
    pub(super) identity: PageShellSidebarSectionIdentity,
    pub(super) title: &'a str,
    pub(super) icon_name: &'a str,
}

pub(super) fn parse_section(
    header: SidebarSectionHeader<'_>,
    block_ids: &[String],
    blocks: &Map<String, Value>,
    space_id: &str,
) -> Result<ParsedSidebarSection, String> {
    let SidebarSectionHeader {
        identity,
        title,
        icon_name,
    } = header;
    let nodes = block_ids
        .iter()
        .map(|block_id| ParsedSidebarNode::parse(block_id, blocks, space_id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedSidebarSection {
        identity,
        title: title.to_string(),
        icon: named_page_shell_icon(icon_name),
        nodes,
    })
}

impl CollectionPointer {
    pub(super) fn id(&self) -> &str {
        &self.id
    }
}

/// The resolved child items and whether every child block was loaded.
type ResolvedSidebarChildren = (Vec<PageShellSidebarItem>, bool);

impl ParsedSidebarNode {
    pub(super) fn parse(
        block_id: &str,
        blocks: &Map<String, Value>,
        space_id: &str,
    ) -> Result<Self, String> {
        Self::parse_tree(block_id, blocks, space_id, &mut Vec::new())
    }

    fn parse_tree(
        block_id: &str,
        blocks: &Map<String, Value>,
        space_id: &str,
        ancestors: &mut Vec<String>,
    ) -> Result<Self, String> {
        if ancestors.iter().any(|ancestor| ancestor == block_id) {
            return Err(format!("cyclic Notion sidebar block tree at {block_id}"));
        }
        let block = block_value(blocks, block_id)?;
        let (kind, title, fallback_icon) = parse_node_header(block, block_id, space_id)?;
        ancestors.push(block_id.to_string());
        let (children, child_block_ids, unresolved_child_block_ids) =
            parse_children(block_id, block, blocks, space_id, ancestors)?;
        ancestors.pop();
        Ok(Self {
            block_id: block_id.to_string(),
            kind,
            title,
            icon: page_shell_icon(block, fallback_icon),
            children,
            child_block_ids,
            unresolved_child_block_ids,
        })
    }

    fn collect_collection_pointers<'a>(&'a self, pointers: &mut Vec<&'a CollectionPointer>) {
        if let ParsedSidebarNodeTitle::Collections(collection_pointers) = &self.title {
            pointers.extend(collection_pointers);
        }
        for child in &self.children {
            child.collect_collection_pointers(pointers);
        }
    }

    pub(super) fn collect_child_collection_pointers<'a>(
        &'a self,
        pointers: &mut Vec<&'a CollectionPointer>,
    ) {
        for child in &self.children {
            child.collect_collection_pointers(pointers);
        }
    }

    pub(super) fn unresolved_child_block_ids(&self) -> &[String] {
        &self.unresolved_child_block_ids
    }

    fn resolve(
        self,
        collections: &Map<String, Value>,
        board_target: &BoardTarget,
    ) -> Result<PageShellSidebarItem, String> {
        self.resolve_with(board_target, &|title| {
            resolve_node_title(title, collections)
        })
    }

    fn resolve_with(
        self,
        board_target: &BoardTarget,
        resolve_title: &impl Fn(ParsedSidebarNodeTitle) -> Result<String, String>,
    ) -> Result<PageShellSidebarItem, String> {
        let sidebar_children_resolved = self.unresolved_child_block_ids.is_empty();
        let title = resolve_title(self.title)?;
        let identity = match self.kind {
            ParsedSidebarNodeKind::Page => PageShellNodeIdentity::Page {
                block_id: self.block_id.clone(),
            },
            ParsedSidebarNodeKind::Database => PageShellNodeIdentity::Database {
                block_id: self.block_id.clone(),
            },
        };
        Ok(PageShellSidebarItem {
            identity: Some(identity),
            title,
            icon: self.icon,
            active: normalize_uuid(&self.block_id) == board_target.collection_view_block_id,
            target_board_url: Some(board_target.child_url(&self.block_id)),
            children: self
                .children
                .into_iter()
                .map(|child| child.resolve_with(board_target, resolve_title))
                .collect::<Result<Vec<_>, _>>()?,
            child_block_ids: self.child_block_ids,
            unresolved_child_block_ids: self.unresolved_child_block_ids,
            sidebar_children_resolved,
        })
    }

    pub(super) fn resolve_children(
        self,
        collections: &Map<String, Value>,
        board_target: &BoardTarget,
    ) -> Result<ResolvedSidebarChildren, String> {
        let sidebar_children_resolved = self.unresolved_child_block_ids.is_empty();
        let children = self
            .children
            .into_iter()
            .map(|child| child.resolve(collections, board_target))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((children, sidebar_children_resolved))
    }
}

impl ParsedSidebarSection {
    pub(super) fn collect_collection_pointers<'a>(
        &'a self,
        pointers: &mut Vec<&'a CollectionPointer>,
    ) {
        for node in &self.nodes {
            node.collect_collection_pointers(pointers);
        }
    }

    pub(super) fn resolve(
        self,
        collections: &Map<String, Value>,
        board_target: &BoardTarget,
    ) -> Result<PageShellSidebarSection, String> {
        self.resolve_with(board_target, &|title| {
            resolve_node_title(title, collections)
        })
    }

    pub(super) fn resolve_initial(
        self,
        board_target: &BoardTarget,
    ) -> Result<PageShellSidebarSection, String> {
        self.resolve_with(board_target, &resolve_initial_node_title)
    }

    fn resolve_with(
        self,
        board_target: &BoardTarget,
        resolve_title: &impl Fn(ParsedSidebarNodeTitle) -> Result<String, String>,
    ) -> Result<PageShellSidebarSection, String> {
        Ok(PageShellSidebarSection {
            identity: Some(self.identity),
            title: self.title,
            icon: self.icon,
            items: self
                .nodes
                .into_iter()
                .map(|node| node.resolve_with(board_target, resolve_title))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

/// A node's kind, its title, and the icon it falls back to.
type ParsedNodeHeader = (ParsedSidebarNodeKind, ParsedSidebarNodeTitle, &'static str);

fn parse_node_header(
    block: &Value,
    block_id: &str,
    space_id: &str,
) -> Result<ParsedNodeHeader, String> {
    match required_string(block, "type")? {
        "page" => Ok((
            ParsedSidebarNodeKind::Page,
            title_property(block)?
                .map(ParsedSidebarNodeTitle::Titled)
                .unwrap_or(ParsedSidebarNodeTitle::Untitled),
            "page",
        )),
        "transcription" => Ok((
            ParsedSidebarNodeKind::Page,
            title_property(block)?
                .map(ParsedSidebarNodeTitle::Titled)
                .unwrap_or(ParsedSidebarNodeTitle::Untitled),
            "meetings",
        )),
        "collection_view" | "collection_view_page" => Ok((
            ParsedSidebarNodeKind::Database,
            match title_property(block)? {
                Some(title) => ParsedSidebarNodeTitle::Titled(title),
                None => ParsedSidebarNodeTitle::Collections(collection_pointers(
                    block, block_id, space_id,
                )?),
            },
            "database",
        )),
        block_type => Err(format!(
            "unsupported Notion sidebar block type {block_type} for {block_id}"
        )),
    }
}

fn resolve_node_title(
    title: ParsedSidebarNodeTitle,
    collections: &Map<String, Value>,
) -> Result<String, String> {
    match title {
        ParsedSidebarNodeTitle::Titled(title) => Ok(title),
        ParsedSidebarNodeTitle::Untitled => Ok("Untitled".to_string()),
        ParsedSidebarNodeTitle::Collections(pointers) => {
            let untitled_label = if pointers.len() == 1 {
                "New database"
            } else {
                "New data source"
            };
            pointers
                .into_iter()
                .map(|pointer| {
                    let collection = collection_entry(collections, &pointer.id)?;
                    Ok(collection_name(collection)?.unwrap_or_else(|| untitled_label.to_string()))
                })
                .collect::<Result<Vec<_>, String>>()
                .map(|names| names.join(" and "))
        }
    }
}

fn resolve_initial_node_title(title: ParsedSidebarNodeTitle) -> Result<String, String> {
    Ok(match title {
        ParsedSidebarNodeTitle::Titled(title) => title,
        ParsedSidebarNodeTitle::Untitled => "Untitled".to_string(),
        ParsedSidebarNodeTitle::Collections(pointers) if pointers.len() == 1 => {
            "New database".to_string()
        }
        ParsedSidebarNodeTitle::Collections(_) => "New data source".to_string(),
    })
}

fn collection_pointers(
    block: &Value,
    block_id: &str,
    space_id: &str,
) -> Result<Vec<CollectionPointer>, String> {
    let format = block
        .get("format")
        .ok_or_else(|| format!("missing format for Notion database {block_id}"))?;
    let pointer_values = match format.get("collection_pointers") {
        Some(Value::Array(pointers)) if !pointers.is_empty() => pointers.clone(),
        Some(Value::Array(_)) | None => vec![format
            .get("collection_pointer")
            .ok_or_else(|| format!("missing collection pointer for Notion database {block_id}"))?
            .clone()],
        Some(_) => {
            return Err(format!(
                "invalid collection pointers for Notion database {block_id}"
            ));
        }
    };
    pointer_values
        .into_iter()
        .map(|pointer| {
            let pointer =
                serde_json::from_value::<CollectionPointer>(pointer).map_err(|error| {
                    format!("invalid collection pointer for Notion database {block_id}: {error}")
                })?;
            match pointer.table {
                CollectionTable::Collection => {}
            }
            if pointer.space_id != space_id {
                return Err(format!(
                    "Notion database {block_id} collection pointer belongs to another space"
                ));
            }
            Ok(pointer)
        })
        .collect()
}
