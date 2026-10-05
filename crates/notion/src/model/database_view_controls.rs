use std::{collections::HashSet, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::model::{NotionCollectionViewId, NotionDatabasePropertyId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseViewSortDirection {
    Ascending,
    Descending,
}

impl DatabaseViewSortDirection {
    pub const fn as_notion_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

impl FromStr for DatabaseViewSortDirection {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ascending" => Ok(Self::Ascending),
            "descending" => Ok(Self::Descending),
            _ => Err("Notion database sort direction must be ascending or descending"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseViewSort {
    property_id: NotionDatabasePropertyId,
    direction: DatabaseViewSortDirection,
}

impl DatabaseViewSort {
    pub fn new(
        property_id: NotionDatabasePropertyId,
        direction: DatabaseViewSortDirection,
    ) -> Self {
        Self {
            property_id,
            direction,
        }
    }

    pub fn property_id(&self) -> &NotionDatabasePropertyId {
        &self.property_id
    }

    pub const fn direction(&self) -> DatabaseViewSortDirection {
        self.direction
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<DatabaseViewSort>", into = "Vec<DatabaseViewSort>")]
pub struct DatabaseViewSortState(Vec<DatabaseViewSort>);

impl DatabaseViewSortState {
    pub fn as_slice(&self) -> &[DatabaseViewSort] {
        &self.0
    }

    pub(crate) fn adding(&self, sort: DatabaseViewSort) -> Result<DatabaseViewSortState, String> {
        if self
            .0
            .iter()
            .any(|existing| existing.property_id == sort.property_id)
        {
            return Err(format!(
                "Notion database view already sorts by property {}",
                sort.property_id.as_str()
            ));
        }
        let mut sorts = self.0.clone();
        sorts.push(sort);
        Self::try_from(sorts)
    }

    pub(crate) fn toggling(
        &self,
        property_id: &NotionDatabasePropertyId,
    ) -> Result<DatabaseViewSortState, String> {
        let mut sorts = self.0.clone();
        let sort = sorts
            .iter_mut()
            .find(|sort| sort.property_id() == property_id)
            .ok_or_else(|| {
                format!(
                    "Notion database view does not sort by property {}",
                    property_id.as_str()
                )
            })?;
        sort.direction = sort.direction.toggled();
        Self::try_from(sorts)
    }

    pub(crate) fn removing(
        &self,
        property_id: &NotionDatabasePropertyId,
    ) -> Result<DatabaseViewSortState, String> {
        let mut sorts = self.0.clone();
        let initial_len = sorts.len();
        sorts.retain(|sort| sort.property_id() != property_id);
        if sorts.len() == initial_len {
            return Err(format!(
                "Notion database view does not sort by property {}",
                property_id.as_str()
            ));
        }
        Self::try_from(sorts)
    }
}

impl TryFrom<Vec<DatabaseViewSort>> for DatabaseViewSortState {
    type Error = String;

    fn try_from(sorts: Vec<DatabaseViewSort>) -> Result<Self, Self::Error> {
        let mut property_ids = HashSet::with_capacity(sorts.len());
        for sort in &sorts {
            if !property_ids.insert(sort.property_id.as_str()) {
                return Err(format!(
                    "Notion database sort contains duplicate property {}",
                    sort.property_id.as_str()
                ));
            }
        }
        Ok(Self(sorts))
    }
}

impl From<DatabaseViewSortState> for Vec<DatabaseViewSort> {
    fn from(state: DatabaseViewSortState) -> Self {
        state.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseViewGroupKind {
    Status,
    Select,
}

impl DatabaseViewGroupKind {
    pub fn parse_schema_type(property_type: &str) -> Result<Self, &'static str> {
        match property_type {
            "status" => Ok(Self::Status),
            "select" => Ok(Self::Select),
            _ => Err("Notion database grouping is only writable for Status and Select properties"),
        }
    }

    pub const fn as_notion_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Select => "select",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseViewGroup {
    property_id: NotionDatabasePropertyId,
    kind: DatabaseViewGroupKind,
}

impl DatabaseViewGroup {
    pub fn new(property_id: NotionDatabasePropertyId, kind: DatabaseViewGroupKind) -> Self {
        Self { property_id, kind }
    }

    pub fn property_id(&self) -> &NotionDatabasePropertyId {
        &self.property_id
    }

    pub const fn kind(&self) -> DatabaseViewGroupKind {
        self.kind
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum DatabaseViewGroupState {
    #[default]
    None,
    Supported {
        group: DatabaseViewGroup,
    },
    Unsupported {
        property_id: Option<String>,
        property_type: Option<String>,
    },
}

impl DatabaseViewGroupState {
    pub fn supported(&self) -> Option<&DatabaseViewGroup> {
        match self {
            Self::Supported { group } => Some(group),
            Self::None | Self::Unsupported { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseViewControlMutationRequest {
    expected_view_id: NotionCollectionViewId,
    mutation: DatabaseViewControlMutation,
}

impl DatabaseViewControlMutationRequest {
    pub fn add_sort(
        expected_view_id: NotionCollectionViewId,
        property_id: impl Into<String>,
        direction: DatabaseViewSortDirection,
    ) -> Result<Self, String> {
        Ok(Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::AddSort(DatabaseViewSort::new(
                property_id.into().parse().map_err(str::to_string)?,
                direction,
            )),
        })
    }

    pub fn toggle_sort_direction(
        expected_view_id: NotionCollectionViewId,
        property_id: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::ToggleSortDirection(
                property_id.into().parse().map_err(str::to_string)?,
            ),
        })
    }

    pub fn remove_sort(
        expected_view_id: NotionCollectionViewId,
        property_id: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::RemoveSort(
                property_id.into().parse().map_err(str::to_string)?,
            ),
        })
    }

    pub fn set_group(
        expected_view_id: NotionCollectionViewId,
        property_id: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::SetGroup(
                property_id.into().parse().map_err(str::to_string)?,
            ),
        })
    }

    pub fn clear_group(expected_view_id: NotionCollectionViewId) -> Self {
        Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::ClearGroup,
        }
    }

    pub fn set_property_visibility(
        expected_view_id: NotionCollectionViewId,
        property_id: impl Into<String>,
        visible: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            expected_view_id,
            mutation: DatabaseViewControlMutation::SetPropertyVisibility {
                property_id: property_id.into().parse().map_err(str::to_string)?,
                visible,
            },
        })
    }

    pub(crate) fn into_parts(self) -> (NotionCollectionViewId, DatabaseViewControlMutation) {
        (self.expected_view_id, self.mutation)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DatabaseViewControlMutation {
    AddSort(DatabaseViewSort),
    ToggleSortDirection(NotionDatabasePropertyId),
    RemoveSort(NotionDatabasePropertyId),
    SetGroup(NotionDatabasePropertyId),
    ClearGroup,
    SetPropertyVisibility {
        property_id: NotionDatabasePropertyId,
        visible: bool,
    },
}
