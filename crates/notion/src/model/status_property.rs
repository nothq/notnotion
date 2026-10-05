use std::{collections::HashSet, sync::Arc};

use super::{
    CardPageProperty, DatabaseProperty, DatabasePropertyOption, NotionCollectionViewId,
    NotionDatabasePropertyId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseStatusProperty {
    property_id: NotionDatabasePropertyId,
    options: Arc<[DatabasePropertyOption]>,
}

impl DatabaseStatusProperty {
    pub fn parse(
        property_id: &str,
        property_type: &str,
        options: &[DatabasePropertyOption],
    ) -> Result<Self, String> {
        if property_type != "status" {
            return Err(format!(
                "Notion property {property_id} has type {property_type}, expected status"
            ));
        }
        let property_id = property_id
            .parse::<NotionDatabasePropertyId>()
            .map_err(str::to_string)?;
        if options.is_empty() {
            return Err(format!(
                "Notion status property {} has no schema options",
                property_id.as_str()
            ));
        }
        let mut option_ids = HashSet::with_capacity(options.len());
        for option in options {
            if option.id.trim().is_empty() {
                return Err(format!(
                    "Notion status property {} contains an empty option ID",
                    property_id.as_str()
                ));
            }
            if option.value.trim().is_empty() {
                return Err(format!(
                    "Notion status property {} contains an empty option label",
                    property_id.as_str()
                ));
            }
            if !option_ids.insert(option.id.as_str()) {
                return Err(format!(
                    "Notion status property {} contains duplicate option ID {}",
                    property_id.as_str(),
                    option.id
                ));
            }
        }
        Ok(Self {
            property_id,
            options: options.to_vec().into(),
        })
    }

    pub fn property_id(&self) -> &NotionDatabasePropertyId {
        &self.property_id
    }

    pub fn options(&self) -> &[DatabasePropertyOption] {
        &self.options
    }

    pub fn option_id_for_value(&self, value: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|option| option.value == value)
            .map(|option| option.id.as_str())
    }

    fn option(&self, option_id: &str) -> Result<DatabasePropertyOption, String> {
        self.options
            .iter()
            .find(|option| option.id == option_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Notion status option {option_id} does not belong to property {}",
                    self.property_id.as_str()
                )
            })
    }
}

impl TryFrom<&DatabaseProperty> for DatabaseStatusProperty {
    type Error = String;

    fn try_from(property: &DatabaseProperty) -> Result<Self, Self::Error> {
        Self::parse(
            &property.property_id,
            &property.property_type,
            &property.options,
        )
    }
}

impl TryFrom<&CardPageProperty> for DatabaseStatusProperty {
    type Error = String;

    fn try_from(property: &CardPageProperty) -> Result<Self, Self::Error> {
        Self::parse(
            &property.property_id,
            &property.property_type,
            &property.status_options,
        )
    }
}

#[derive(Clone, Debug)]
pub struct SetDatabaseStatusPropertyRequest {
    page_id: NotionDatabasePageId,
    scope: DatabaseStatusPropertyMutationScope,
    property_id: NotionDatabasePropertyId,
    option: DatabasePropertyOption,
}

impl SetDatabaseStatusPropertyRequest {
    pub fn for_database_view(
        page_id: impl Into<String>,
        expected_view_id: NotionCollectionViewId,
        property: &DatabaseStatusProperty,
        option_id: &str,
    ) -> Result<Self, String> {
        Self::new(
            page_id,
            DatabaseStatusPropertyMutationScope::DatabaseView(expected_view_id),
            property,
            option_id,
        )
    }

    pub fn for_loaded_page(
        page_id: impl Into<String>,
        property: &DatabaseStatusProperty,
        option_id: &str,
    ) -> Result<Self, String> {
        Self::new(
            page_id,
            DatabaseStatusPropertyMutationScope::LoadedPage,
            property,
            option_id,
        )
    }

    fn new(
        page_id: impl Into<String>,
        scope: DatabaseStatusPropertyMutationScope,
        property: &DatabaseStatusProperty,
        option_id: &str,
    ) -> Result<Self, String> {
        let page_id = NotionDatabasePageId::try_from(page_id.into()).map_err(str::to_string)?;
        let option = property.option(option_id)?;
        Ok(Self {
            page_id,
            scope,
            property_id: property.property_id.clone(),
            option,
        })
    }

    pub(crate) fn page_id(&self) -> &str {
        &self.page_id.0
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        String,
        DatabaseStatusPropertyMutationScope,
        NotionDatabasePropertyId,
        DatabasePropertyOption,
    ) {
        (self.page_id.0, self.scope, self.property_id, self.option)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DatabaseStatusPropertyMutationScope {
    DatabaseView(NotionCollectionViewId),
    LoadedPage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NotionDatabasePageId(String);

impl TryFrom<String> for NotionDatabasePageId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("Notion database page ID must not be empty or whitespace");
        }
        Ok(Self(value))
    }
}
