use serde::{Deserialize, Serialize};

use super::condition::DatabasePropertyFilter;
use super::request::{
    DatabaseAdvancedFilterMutation, DatabaseFilterGroupOperator, DatabaseSimpleFilterMutation,
    DatabaseSimpleFilterPlacement, DatabaseViewFilterMutation,
};
use super::NotionDatabaseFilterId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabaseFilterNode {
    Group(DatabaseFilterGroup),
    Property(DatabasePropertyFilter),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseFilterGroup {
    operator: DatabaseFilterGroupOperator,
    filters: Vec<DatabaseFilterNode>,
}

impl DatabaseFilterGroup {
    pub fn new(operator: DatabaseFilterGroupOperator, filters: Vec<DatabaseFilterNode>) -> Self {
        Self { operator, filters }
    }

    pub fn operator(&self) -> DatabaseFilterGroupOperator {
        self.operator
    }

    pub fn filters(&self) -> &[DatabaseFilterNode] {
        &self.filters
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseSimpleFilter {
    filter_id: NotionDatabaseFilterId,
    filter: DatabasePropertyFilter,
}

impl DatabaseSimpleFilter {
    pub fn new(filter_id: NotionDatabaseFilterId, filter: DatabasePropertyFilter) -> Self {
        Self { filter_id, filter }
    }

    pub fn filter_id(&self) -> &NotionDatabaseFilterId {
        &self.filter_id
    }

    pub fn filter(&self) -> &DatabasePropertyFilter {
        &self.filter
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "filters")]
pub enum DatabaseSimpleFiltersState {
    Entries(Vec<DatabaseSimpleFilterState>),
    Unsupported,
}

impl Default for DatabaseSimpleFiltersState {
    fn default() -> Self {
        Self::Entries(Vec::new())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "filter")]
pub enum DatabaseSimpleFilterState {
    Editable(DatabaseSimpleFilter),
    Unsupported(NotionDatabaseFilterId),
}

impl DatabaseSimpleFilterState {
    pub fn filter_id(&self) -> &NotionDatabaseFilterId {
        match self {
            Self::Editable(filter) => filter.filter_id(),
            Self::Unsupported(filter_id) => filter_id,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "filter")]
pub enum DatabaseAdvancedFilterState {
    #[default]
    None,
    Editable(DatabaseFilterGroup),
    Unsupported,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseViewFilterState {
    simple: DatabaseSimpleFiltersState,
    advanced: DatabaseAdvancedFilterState,
}

impl DatabaseViewFilterState {
    pub fn new(simple: DatabaseSimpleFiltersState, advanced: DatabaseAdvancedFilterState) -> Self {
        Self { simple, advanced }
    }

    pub fn simple(&self) -> &DatabaseSimpleFiltersState {
        &self.simple
    }

    pub fn advanced(&self) -> &DatabaseAdvancedFilterState {
        &self.advanced
    }

    pub(crate) fn applying(&self, mutation: &DatabaseViewFilterMutation) -> Result<Self, String> {
        let mut next = self.clone();
        match mutation {
            DatabaseViewFilterMutation::Simple(mutation) => next.apply_simple_mutation(mutation)?,
            DatabaseViewFilterMutation::Advanced(mutation) => {
                next.apply_advanced_mutation(mutation)?
            }
        }
        Ok(next)
    }

    fn apply_simple_mutation(
        &mut self,
        mutation: &DatabaseSimpleFilterMutation,
    ) -> Result<(), String> {
        let DatabaseSimpleFiltersState::Entries(filters) = &mut self.simple else {
            return Err("unsupported Notion simple filters are not editable".to_string());
        };
        match mutation {
            DatabaseSimpleFilterMutation::Add { filter, placement } => {
                add_simple_filter(filters, filter, placement)
            }
            DatabaseSimpleFilterMutation::Update(filter) => update_simple_filter(filters, filter),
            DatabaseSimpleFilterMutation::Remove(filter_id) => {
                remove_simple_filter(filters, filter_id)
            }
        }
    }

    fn apply_advanced_mutation(
        &mut self,
        mutation: &DatabaseAdvancedFilterMutation,
    ) -> Result<(), String> {
        if self.advanced == DatabaseAdvancedFilterState::Unsupported {
            return Err("unsupported Notion advanced filters are not editable".to_string());
        }
        self.advanced = match mutation {
            DatabaseAdvancedFilterMutation::Set(filter) => {
                DatabaseAdvancedFilterState::Editable(filter.clone())
            }
            DatabaseAdvancedFilterMutation::Clear => DatabaseAdvancedFilterState::None,
        };
        Ok(())
    }
}

fn add_simple_filter(
    filters: &mut Vec<DatabaseSimpleFilterState>,
    filter: &DatabaseSimpleFilter,
    placement: &DatabaseSimpleFilterPlacement,
) -> Result<(), String> {
    if filters
        .iter()
        .any(|existing| existing.filter_id() == filter.filter_id())
    {
        return Err(format!(
            "Notion simple filter {} already exists",
            filter.filter_id.as_str()
        ));
    }
    let index = insertion_index(filters, placement)?;
    filters.insert(index, DatabaseSimpleFilterState::Editable(filter.clone()));
    Ok(())
}

fn insertion_index(
    filters: &[DatabaseSimpleFilterState],
    placement: &DatabaseSimpleFilterPlacement,
) -> Result<usize, String> {
    match placement {
        DatabaseSimpleFilterPlacement::End => Ok(filters.len()),
        DatabaseSimpleFilterPlacement::Before(filter_id) => filters
            .iter()
            .position(|filter| filter.filter_id() == filter_id)
            .ok_or_else(|| {
                format!(
                    "Notion simple filter insertion anchor {} does not exist",
                    filter_id.as_str()
                )
            }),
    }
}

fn update_simple_filter(
    filters: &mut [DatabaseSimpleFilterState],
    filter: &DatabaseSimpleFilter,
) -> Result<(), String> {
    let existing = filters
        .iter_mut()
        .find(|existing| existing.filter_id() == filter.filter_id())
        .ok_or_else(|| {
            format!(
                "Notion simple filter {} does not exist",
                filter.filter_id.as_str()
            )
        })?;
    *existing = DatabaseSimpleFilterState::Editable(filter.clone());
    Ok(())
}

fn remove_simple_filter(
    filters: &mut Vec<DatabaseSimpleFilterState>,
    filter_id: &NotionDatabaseFilterId,
) -> Result<(), String> {
    let index = filters
        .iter()
        .position(|filter| filter.filter_id() == filter_id)
        .ok_or_else(|| format!("Notion simple filter {} does not exist", filter_id.as_str()))?;
    filters.remove(index);
    Ok(())
}
