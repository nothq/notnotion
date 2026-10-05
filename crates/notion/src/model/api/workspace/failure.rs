use std::sync::Arc;

use crate::model::NotionBootstrapApi;

type RebootstrapApi = Arc<dyn NotionBootstrapApi>;

pub struct NotionWorkspaceOperationFailure {
    message: String,
    rebootstrap_api: Option<RebootstrapApi>,
}

impl NotionWorkspaceOperationFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            rebootstrap_api: None,
        }
    }

    pub(crate) fn requiring_rebootstrap(message: String, rebootstrap_api: RebootstrapApi) -> Self {
        Self {
            message,
            rebootstrap_api: Some(rebootstrap_api),
        }
    }

    pub(crate) fn into_parts(self) -> (String, Option<RebootstrapApi>) {
        (self.message, self.rebootstrap_api)
    }

    pub(crate) fn with_context(mut self, context: &str) -> Self {
        self.message = format!("{context}: {}", self.message);
        self
    }
}

impl From<String> for NotionWorkspaceOperationFailure {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl std::fmt::Display for NotionWorkspaceOperationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::fmt::Debug for NotionWorkspaceOperationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NotionWorkspaceOperationFailure")
            .field("message", &self.message)
            .field("requires_rebootstrap", &self.rebootstrap_api.is_some())
            .finish()
    }
}

impl std::error::Error for NotionWorkspaceOperationFailure {}
