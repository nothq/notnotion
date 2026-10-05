use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

use super::view::NotionCollectionViewId;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NotionBoardUrl(Url);

impl NotionBoardUrl {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Build the canonical board URL for a block opened from this board.
    ///
    /// Notion page-link anchors use the current workspace path as their base,
    /// replace the final block id, and discard view/query state. Keeping that
    /// URL construction on the typed route avoids each renderer inventing its
    /// own path handling.
    pub fn child_block_url(&self, child_block_id: &str) -> String {
        let mut child_url = self.0.clone();
        let parent_path = child_url
            .path()
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or_default()
            .to_string();
        child_url.set_path(&format!(
            "{parent_path}/{}",
            child_block_id.replace('-', "")
        ));
        child_url.set_query(None);
        child_url.set_fragment(None);
        child_url.to_string()
    }

    pub fn with_collection_view_id(&self, collection_view_id: &NotionCollectionViewId) -> Self {
        let mut replaced_view_id = false;
        let mut query_pairs = self
            .0
            .query_pairs()
            .filter_map(|(name, value)| {
                if name == "v" {
                    if replaced_view_id {
                        return None;
                    }
                    replaced_view_id = true;
                    return Some((name.into_owned(), collection_view_id.as_str().to_string()));
                }
                Some((name.into_owned(), value.into_owned()))
            })
            .collect::<Vec<_>>();
        if !replaced_view_id {
            query_pairs.push(("v".to_string(), collection_view_id.as_str().to_string()));
        }

        let mut board_url = self.0.clone();
        board_url.set_query(None);
        board_url.query_pairs_mut().extend_pairs(query_pairs);
        Self(board_url)
    }
}

impl TryFrom<String> for NotionBoardUrl {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = Url::parse(&value)
            .map_err(|error| format!("invalid Notion board URL `{value}`: {error}"))?;
        if !parsed.has_host() {
            return Err(format!("Notion board URL must be absolute, got `{value}`"));
        }
        Ok(Self(parsed))
    }
}

impl FromStr for NotionBoardUrl {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<NotionBoardUrl> for String {
    fn from(value: NotionBoardUrl) -> Self {
        value.0.to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NotionLaunchRoute {
    Board { board_url: NotionBoardUrl },
}

impl NotionLaunchRoute {
    pub const fn board(board_url: NotionBoardUrl) -> Self {
        Self::Board { board_url }
    }

    pub const fn board_url(&self) -> &NotionBoardUrl {
        match self {
            Self::Board { board_url } => board_url,
        }
    }

    pub fn board_url_state(&self) -> NotionBoardUrlState {
        NotionBoardUrlState::parse(self.board_url())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotionBoardUrlState {
    pub selected_page_block_id: Option<String>,
    pub slash_command_open: bool,
}

impl NotionBoardUrlState {
    pub fn parse(board_url: &NotionBoardUrl) -> Self {
        let mut state = Self::default();

        for (name, value) in board_url.0.query_pairs() {
            match name.as_ref() {
                "p" => state.selected_page_block_id = Some(normalize_notion_block_id(&value)),
                "pm" => state.slash_command_open = value == "s",
                _ => {}
            }
        }

        if state.selected_page_block_id.is_none() {
            state.slash_command_open = false;
        }
        state
    }
}

fn normalize_notion_block_id(value: &str) -> String {
    if value.len() == 32 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        format!(
            "{}-{}-{}-{}-{}",
            &value[0..8],
            &value[8..12],
            &value[12..16],
            &value[16..20],
            &value[20..32]
        )
    } else {
        value.to_string()
    }
}
