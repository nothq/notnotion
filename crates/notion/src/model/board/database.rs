use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{CardSummary, NotionDatabasePropertyId};

use super::PageShellIcon;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardColumn {
    pub title: String,
    pub option_color: Option<String>,
    pub cards: Vec<CardSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardDateValue {
    pub start_date: String,
    pub end_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardItemProperty {
    pub property_id: String,
    pub label: String,
    pub property_type: String,
    pub value: String,
    #[serde(default)]
    pub date: Option<BoardDateValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardItem {
    pub block_id: String,
    pub title: String,
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PageShellIcon>,
    #[serde(default)]
    pub properties: Vec<BoardItemProperty>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableViewColumn {
    pub property_id: String,
    pub label: String,
    pub width: f32,
    #[serde(default)]
    pub wrap: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseViewPropertyVisibility {
    property_id: NotionDatabasePropertyId,
    visible: bool,
}

impl DatabaseViewPropertyVisibility {
    pub fn new(property_id: NotionDatabasePropertyId, visible: bool) -> Self {
        Self {
            property_id,
            visible,
        }
    }

    pub fn property_id(&self) -> &NotionDatabasePropertyId {
        &self.property_id
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "Vec<DatabaseViewPropertyVisibility>",
    into = "Vec<DatabaseViewPropertyVisibility>"
)]
pub struct DatabaseViewPropertyLayout(Vec<DatabaseViewPropertyVisibility>);

impl DatabaseViewPropertyLayout {
    pub fn as_slice(&self) -> &[DatabaseViewPropertyVisibility] {
        &self.0
    }

    pub fn visibility_for(&self, property_id: &str) -> Option<bool> {
        self.0
            .iter()
            .find(|entry| entry.property_id.as_str() == property_id)
            .map(|entry| entry.visible)
    }

    pub(crate) fn setting_visibility(
        &self,
        property_id: &NotionDatabasePropertyId,
        visible: bool,
    ) -> Result<Self, String> {
        let mut entries = self.0.clone();
        let entry = entries
            .iter_mut()
            .find(|entry| entry.property_id() == property_id)
            .ok_or_else(|| {
                format!(
                    "Notion active view layout does not contain property {}",
                    property_id.as_str()
                )
            })?;
        if entry.visible == visible {
            return Err(format!(
                "Notion property {} visibility is already {}",
                property_id.as_str(),
                if visible { "shown" } else { "hidden" }
            ));
        }
        entry.visible = visible;
        Self::try_from(entries)
    }
}

impl TryFrom<Vec<DatabaseViewPropertyVisibility>> for DatabaseViewPropertyLayout {
    type Error = String;

    fn try_from(entries: Vec<DatabaseViewPropertyVisibility>) -> Result<Self, Self::Error> {
        let mut property_ids = HashSet::with_capacity(entries.len());
        for entry in &entries {
            if !property_ids.insert(entry.property_id.as_str()) {
                return Err(format!(
                    "Notion database view property layout contains duplicate property {}",
                    entry.property_id.as_str()
                ));
            }
        }
        Ok(Self(entries))
    }
}

impl From<DatabaseViewPropertyLayout> for Vec<DatabaseViewPropertyVisibility> {
    fn from(layout: DatabaseViewPropertyLayout) -> Self {
        layout.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseProperty {
    pub property_id: String,
    pub label: String,
    pub property_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_type: Option<String>,
    #[serde(default)]
    pub options: Vec<DatabasePropertyOption>,
    #[serde(default)]
    pub status_groups: Vec<DatabaseStatusGroup>,
    #[serde(default)]
    pub relation_collection_id: Option<String>,
}

impl DatabaseProperty {
    pub fn filter_type(&self) -> &str {
        self.filter_type.as_deref().unwrap_or(&self.property_type)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabasePropertyOption {
    pub id: String,
    pub value: String,
    pub color: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseStatusGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub option_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineViewConfig {
    pub date_property_id: String,
    #[serde(default)]
    pub relation_property_id: Option<String>,
    #[serde(default)]
    pub zoom_level: Option<String>,
    #[serde(default)]
    pub center_timestamp_ms: Option<i64>,
    #[serde(default)]
    pub today_marker_timestamp_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarViewConfig {
    pub date_property_id: String,
    #[serde(default)]
    pub has_explicit_sort: bool,
    #[serde(default = "default_true")]
    pub show_page_icon: bool,
}

const fn default_true() -> bool {
    true
}
