use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

mod condition;
mod request;
mod state;

pub use condition::*;
pub use request::*;
pub use state::*;

macro_rules! notion_non_empty_id {
    ($name:ident, $error:literal) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.trim().is_empty() {
                    return Err($error);
                }
                Ok(Self(value))
            }
        }

        impl std::str::FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(value.to_string())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

notion_non_empty_id!(
    NotionDatabasePropertyId,
    "Notion database property ID must not be empty or whitespace"
);
notion_non_empty_id!(
    NotionDatabaseFilterId,
    "Notion database filter ID must not be empty or whitespace"
);
pub type NotionFilterUserId = crate::model::NotionUserId;
notion_non_empty_id!(
    NotionFilterPageId,
    "Notion filter page ID must not be empty or whitespace"
);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonEmptyDatabaseFilterValues<T>(Vec<T>);

impl<T> NonEmptyDatabaseFilterValues<T> {
    pub fn one(value: T) -> Self {
        Self(vec![value])
    }

    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptyDatabaseFilterValues<T> {
    type Error = &'static str;

    fn try_from(values: Vec<T>) -> Result<Self, Self::Error> {
        if values.is_empty() {
            return Err("Notion database filter values must not be empty");
        }
        Ok(Self(values))
    }
}

impl<T> Serialize for NonEmptyDatabaseFilterValues<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for NonEmptyDatabaseFilterValues<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(Vec::<T>::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}
