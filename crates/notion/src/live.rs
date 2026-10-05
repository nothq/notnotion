mod board;
mod bootstrap;
mod code_preferences;
mod credentials;
mod desktop_session;
mod http;
mod page_icon;
mod recent_pages;
pub(crate) mod routing;
mod snapshot_cache;
mod workspace_runtime;

pub use board::*;
pub use bootstrap::{load_notion_board_snapshot, production_notion_bootstrap_api};
pub(crate) use page_icon::{
    create_notion_custom_emoji_page_icon, load_notion_custom_emoji_library,
    notion_block_image_render_url, notion_page_icon_render_url, upload_notion_page_icon,
    NotionIconFile,
};
pub(crate) use page_icon::{
    production_notion_local_icon_file_api, production_notion_remote_image_api,
};
pub use workspace_runtime::NotionWorkspaceRuntime;
pub(crate) use workspace_runtime::NotionWorkspaceRuntimeInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionSessionFailure {
    Missing,
    InvalidStored,
    Rejected { status: u16 },
    ActiveUserMismatch,
}

impl std::fmt::Display for NotionSessionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("no stored Notion Desktop session is available"),
            Self::InvalidStored => {
                formatter.write_str("the stored Notion Desktop session is invalid")
            }
            Self::Rejected { status } => {
                write!(
                    formatter,
                    "Notion Desktop session was rejected with HTTP {status}"
                )
            }
            Self::ActiveUserMismatch => formatter
                .write_str("Notion Desktop session validation returned a different active user"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NotionResourceFailure {
    NotFound { diagnostic: String },
    Restricted { diagnostic: String },
}

impl std::fmt::Display for NotionResourceFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { diagnostic } | Self::Restricted { diagnostic } => {
                formatter.write_str(diagnostic)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NotionLiveError {
    Session(NotionSessionFailure),
    Unavailable(NotionResourceFailure),
    Fatal(String),
}

impl std::fmt::Display for NotionLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session(error) => error.fmt(formatter),
            Self::Unavailable(error) => error.fmt(formatter),
            Self::Fatal(message) => formatter.write_str(message),
        }
    }
}

impl From<String> for NotionLiveError {
    fn from(message: String) -> Self {
        Self::Fatal(message)
    }
}
