use std::sync::{Arc, OnceLock};

use super::asset::{NotionNamedIconColor, NotionNamedIconSlug};

const NOTION_NAMED_ICON_CATALOG_COUNT: usize = 885;
const NOTION_NAMED_ICON_DEFAULT_COUNT: usize = 143;

struct NotionNamedIconMetadata {
    slug: &'static str,
    tooltip: &'static str,
    search_terms: &'static str,
}

pub(crate) fn notion_named_icon_catalog() -> &'static [NotionNamedIconSlug] {
    static CATALOG: OnceLock<Vec<NotionNamedIconSlug>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            notion_named_icon_metadata()
                .iter()
                .map(|metadata| NotionNamedIconSlug::from_catalog(metadata.slug))
                .collect()
        })
        .as_slice()
}

fn notion_named_icon_metadata() -> &'static [NotionNamedIconMetadata] {
    static METADATA: OnceLock<Vec<NotionNamedIconMetadata>> = OnceLock::new();
    METADATA
        .get_or_init(|| {
            let metadata = include_str!("../assets/named_icon_metadata.txt")
                .lines()
                .map(parse_metadata_row)
                .collect::<Vec<_>>();
            assert_metadata_invariants(&metadata);
            metadata
        })
        .as_slice()
}

fn parse_metadata_row(line: &'static str) -> NotionNamedIconMetadata {
    let (slug, values) = line
        .split_once('|')
        .expect("every Notion named-icon metadata row must contain a slug");
    let (tooltip, search_terms) = values
        .split_once('|')
        .expect("every Notion named-icon metadata row must contain search terms");
    NotionNamedIconMetadata {
        slug,
        tooltip,
        search_terms,
    }
}

fn assert_metadata_invariants(metadata: &[NotionNamedIconMetadata]) {
    assert_eq!(
        metadata.len(),
        NOTION_NAMED_ICON_CATALOG_COUNT,
        "the Notion named-icon metadata must include the complete catalog"
    );
    assert!(
        metadata.windows(2).all(|pair| pair[0].slug < pair[1].slug),
        "the Notion named-icon metadata must be uniquely sorted by slug"
    );
    assert!(
        metadata.iter().all(|metadata| {
            !metadata.slug.is_empty()
                && metadata
                    .slug
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        }),
        "Notion named-icon slugs must be safe URL path components"
    );
}

fn notion_named_icon_metadata_for(slug: NotionNamedIconSlug) -> &'static NotionNamedIconMetadata {
    let metadata = notion_named_icon_metadata();
    let index = metadata
        .binary_search_by(|metadata| metadata.slug.cmp(slug.as_str()))
        .expect("every catalogued Notion named icon must have metadata");
    &metadata[index]
}

pub(super) fn notion_named_icon_label(slug: NotionNamedIconSlug) -> String {
    notion_named_icon_metadata_for(slug)
        .tooltip
        .replace('-', " ")
}

fn notion_named_icon_matches_terms(slug: NotionNamedIconSlug, terms: &[&str]) -> bool {
    let metadata = notion_named_icon_metadata_for(slug);
    terms.iter().all(|term| {
        slug.as_str().contains(term)
            || metadata.tooltip.contains(term)
            || metadata.search_terms.contains(term)
    })
}

fn notion_named_icon_default_catalog() -> &'static [NotionNamedIconSlug] {
    notion_named_icon_catalog()
        .get(..NOTION_NAMED_ICON_DEFAULT_COUNT)
        .expect("the Notion named-icon catalog must include the default icon set")
}

pub(crate) fn notion_named_icon_matches(query: &str) -> Arc<[NotionNamedIconSlug]> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Arc::from(notion_named_icon_default_catalog());
    }
    let terms = query.split_whitespace().collect::<Vec<_>>();
    notion_named_icon_catalog()
        .iter()
        .copied()
        .filter(|slug| notion_named_icon_matches_terms(*slug, &terms))
        .collect::<Vec<_>>()
        .into()
}

pub(crate) fn notion_named_icon_value_has_color(value: &str) -> bool {
    let value = value.strip_prefix("/icons/").unwrap_or(value);
    let value = value.strip_suffix(".svg").unwrap_or(value);
    value
        .rsplit_once('_')
        .is_some_and(|(_, suffix)| NotionNamedIconColor::from_suffix(suffix).is_some())
}
