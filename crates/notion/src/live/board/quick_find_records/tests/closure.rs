use serde_json::{json, Map, Value};

use super::super::preview::preview_record_closure;
use super::super::test_support::{block_entry, block_table, entry};

fn alias_entry() -> Value {
    block_entry(
        "alias",
        2,
        "alias",
        None,
        Map::from_iter([(
            "format".to_string(),
            json!({
                "alias_pointer": {
                    "table": "block",
                    "id": "alias-target",
                    "spaceId": "space-1",
                }
            }),
        )]),
    )
}

fn link_entry() -> Value {
    block_entry(
        "link",
        3,
        "link_to_page",
        None,
        Map::from_iter([(
            "format".to_string(),
            json!({
                "page_pointer": {
                    "table": "block",
                    "id": "link-target",
                    "spaceId": "space-1",
                }
            }),
        )]),
    )
}

fn text_entry() -> Value {
    block_entry(
        "text",
        4,
        "text",
        None,
        Map::from_iter([(
            "properties".to_string(),
            json!({ "title": [["See "], ["‣", [["p", "page-target"]]]] }),
        )]),
    )
}

#[test]
fn preview_closure_includes_alias_and_rich_text_page_targets() {
    let record_map = block_table([
        (
            "root",
            entry("root", 1, Some(vec!["alias", "link", "text"])),
        ),
        ("alias", alias_entry()),
        ("link", link_entry()),
        ("text", text_entry()),
    ]);

    let pointers = preview_record_closure(&record_map, "space-1", "root").expect("preview closure");
    assert_eq!(
        pointers
            .iter()
            .map(|pointer| (pointer.table.as_str(), pointer.id.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("block", "alias"),
            ("block", "link"),
            ("block", "text"),
            ("block", "alias-target"),
            ("block", "link-target"),
            ("block", "page-target"),
        ]
    );
}

#[test]
fn preview_closure_does_not_descend_into_nested_pages() {
    let record_map = block_table([
        (
            "root",
            entry("root", 1, Some(vec!["nested-page", "sibling"])),
        ),
        (
            "nested-page",
            entry("nested-page", 2, Some(vec!["nested-content"])),
        ),
        ("nested-content", entry("nested-content", 3, None)),
        (
            "sibling",
            block_entry("sibling", 4, "text", None, Map::new()),
        ),
    ]);

    let pointers = preview_record_closure(&record_map, "space-1", "root").expect("preview closure");
    assert_eq!(
        pointers
            .iter()
            .map(|pointer| pointer.id.as_str())
            .collect::<Vec<_>>(),
        vec!["nested-page", "sibling"]
    );
}
