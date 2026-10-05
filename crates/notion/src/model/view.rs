use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::model::DatabaseViewFilterState;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NotionCollectionViewId(String);

impl NotionCollectionViewId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NotionCollectionViewId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("Notion collection view ID must not be empty or whitespace");
        }
        Ok(Self(value))
    }
}

impl FromStr for NotionCollectionViewId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<NotionCollectionViewId> for String {
    fn from(value: NotionCollectionViewId) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewTabKind {
    Table,
    Board,
    List,
    Gallery,
    Timeline,
    Calendar,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewTab {
    pub provider_view_id: NotionCollectionViewId,
    pub label: String,
    pub kind: ViewTabKind,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<DatabaseViewFilterState>,
}
