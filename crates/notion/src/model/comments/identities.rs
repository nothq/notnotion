use serde::{Deserialize, Serialize};

macro_rules! notion_comment_id {
    ($name:ident, $diagnostic:literal) => {
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
                    return Err(concat!($diagnostic, " must not be empty or whitespace"));
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

notion_comment_id!(NotionCommentTargetId, "Notion comment target ID");
notion_comment_id!(NotionDiscussionId, "Notion discussion ID");
notion_comment_id!(NotionCommentId, "Notion comment ID");
