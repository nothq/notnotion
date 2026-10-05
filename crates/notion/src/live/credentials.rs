use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};

use secret_store::SecretStore;
use serde::{Deserialize, Serialize};

use super::desktop_session::capture_notion_desktop_session;
use super::{NotionLiveError, NotionSessionFailure};
use crate::model::NotionRecoveryDisposition;

const NOTION_DESKTOP_SESSION_CACHE_KEY: &str = "notion|desktop-session";

#[derive(Clone, Serialize)]
pub(crate) struct NotionDesktopSession {
    user_id: String,
    cookie_header: String,
    #[serde(skip)]
    instance_id: u64,
}

impl PartialEq for NotionDesktopSession {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id && self.cookie_header == other.cookie_header
    }
}

impl Eq for NotionDesktopSession {}

impl NotionDesktopSession {
    pub(crate) fn new(user_id: String, cookie_header: String) -> Result<Self, String> {
        let user_id = user_id.trim().to_string();
        if user_id.is_empty() {
            return Err("Notion Desktop session is missing its active user id".to_string());
        }
        let cookie_header = cookie_header.trim().to_string();
        let cookies = cookie_pairs(&cookie_header);
        if !cookies
            .iter()
            .any(|(name, value)| *name == "token_v2" && !value.is_empty())
        {
            return Err("Notion Desktop session is missing token_v2".to_string());
        }
        if !cookies
            .iter()
            .any(|(name, value)| *name == "notion_user_id" && *value == user_id)
        {
            return Err("Notion Desktop session cookies do not match the active user".to_string());
        }
        Ok(Self {
            user_id,
            cookie_header,
            instance_id: next_session_instance_id(),
        })
    }

    pub(crate) fn user_id(&self) -> &str {
        &self.user_id
    }

    pub(crate) fn cookie_header(&self) -> &str {
        &self.cookie_header
    }

    fn instance_id(&self) -> u64 {
        self.instance_id
    }
}

#[derive(Deserialize)]
struct StoredNotionDesktopSession {
    user_id: String,
    cookie_header: String,
}

pub(crate) enum StoredNotionSessionError {
    Invalid(String),
    Storage(String),
}

impl std::fmt::Display for StoredNotionSessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) | Self::Storage(message) => formatter.write_str(message),
        }
    }
}

pub(crate) fn current_notion_desktop_session() -> Result<NotionDesktopSession, NotionLiveError> {
    let mut session_slot = session_slot()
        .lock()
        .map_err(|_| NotionLiveError::Fatal("Notion Desktop session lock is poisoned".into()))?;
    if let Some(session) = session_slot.as_ref() {
        return Ok(session.clone());
    }
    let session = match load_stored_notion_desktop_session() {
        Ok(Some(session)) => session,
        Ok(None) => return Err(NotionLiveError::Session(NotionSessionFailure::Missing)),
        Err(StoredNotionSessionError::Invalid(error)) => {
            println!("notnotion stored session is invalid: {error}");
            return Err(NotionLiveError::Session(
                NotionSessionFailure::InvalidStored,
            ));
        }
        Err(StoredNotionSessionError::Storage(error)) => {
            return Err(NotionLiveError::Fatal(error));
        }
    };
    *session_slot = Some(session.clone());
    Ok(session)
}

pub(crate) fn stored_notion_desktop_session(
) -> Result<Option<NotionDesktopSession>, StoredNotionSessionError> {
    let mut session_slot = session_slot().lock().map_err(|_| {
        StoredNotionSessionError::Storage("Notion Desktop session lock is poisoned".to_string())
    })?;
    if let Some(session) = session_slot.as_ref() {
        return Ok(Some(session.clone()));
    }
    let session = load_stored_notion_desktop_session()?;
    *session_slot = session.clone();
    Ok(session)
}

pub(crate) fn recover_notion_desktop_session() -> Result<NotionDesktopSession, String> {
    let session = capture_and_store_notion_desktop_session()?;
    let mut session_slot = session_slot()
        .lock()
        .map_err(|_| "Notion Desktop session lock is poisoned".to_string())?;
    *session_slot = Some(session.clone());
    Ok(session)
}

pub(crate) struct NotionSessionRecoveryError {
    message: String,
    recovery_disposition: NotionRecoveryDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NotionSessionIdentityChange {
    Unchanged,
    Established {
        user_id: String,
    },
    Changed {
        previous_user_id: String,
        current_user_id: String,
    },
}

impl NotionSessionIdentityChange {
    pub(crate) const fn recovery_disposition(&self) -> NotionRecoveryDisposition {
        match self {
            Self::Unchanged => NotionRecoveryDisposition::Preserve,
            Self::Established { user_id } => {
                let _ = user_id;
                NotionRecoveryDisposition::ReloadCache
            }
            Self::Changed {
                previous_user_id,
                current_user_id,
            } => {
                let _ = (previous_user_id, current_user_id);
                NotionRecoveryDisposition::DiscardStateAndReloadCache
            }
        }
    }
}

pub(crate) struct NotionSessionRecovery<T> {
    value: T,
    identity_change: NotionSessionIdentityChange,
}

impl<T> NotionSessionRecovery<T> {
    pub(crate) fn into_parts(self) -> (T, NotionSessionIdentityChange) {
        (self.value, self.identity_change)
    }

    pub(crate) fn into_value(self) -> T {
        self.value
    }
}

impl NotionSessionRecoveryError {
    pub(crate) const fn recovery_disposition(&self) -> NotionRecoveryDisposition {
        self.recovery_disposition
    }
}

impl std::fmt::Display for NotionSessionRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) fn with_notion_session_recovery<T>(
    mut operation: impl FnMut(&NotionDesktopSession) -> Result<T, NotionLiveError>,
) -> Result<NotionSessionRecovery<T>, NotionSessionRecoveryError> {
    let initial_session = current_notion_desktop_session();
    let initial_user_id = initial_session
        .as_ref()
        .ok()
        .map(|session| session.user_id().to_string());
    let first_result = initial_session
        .as_ref()
        .map_err(clone_live_error)
        .and_then(&mut operation);
    let first_error = match first_result {
        Ok(value) => {
            return Ok(NotionSessionRecovery {
                value,
                identity_change: NotionSessionIdentityChange::Unchanged,
            });
        }
        Err(error) => error,
    };
    let NotionLiveError::Session(first_failure) = first_error else {
        return Err(recovery_error(
            first_error,
            NotionRecoveryDisposition::Preserve,
        ));
    };
    let initial_disposition = if first_failure == NotionSessionFailure::ActiveUserMismatch {
        NotionRecoveryDisposition::DiscardStateAndReloadCache
    } else {
        NotionRecoveryDisposition::Preserve
    };
    let recovered = recover_session_after_failure(initial_session.as_ref().ok())
        .map_err(|error| recovery_error_message(error, initial_disposition))?;
    let identity_change = session_identity_change(initial_user_id, recovered.user_id().into());
    let recovery_disposition = initial_disposition.merge(identity_change.recovery_disposition());
    operation(&recovered)
        .map(|value| NotionSessionRecovery {
            value,
            identity_change,
        })
        .map_err(|error| {
            let disposition = if matches!(
                error,
                NotionLiveError::Session(NotionSessionFailure::ActiveUserMismatch)
            ) {
                recovery_disposition.merge(NotionRecoveryDisposition::DiscardStateAndReloadCache)
            } else {
                recovery_disposition
            };
            recovery_error(error, disposition)
        })
}

fn recover_session_after_failure(
    failed_session: Option<&NotionDesktopSession>,
) -> Result<NotionDesktopSession, String> {
    let _guard = session_recovery_lock()
        .lock()
        .map_err(|_| "Notion Desktop session recovery lock is poisoned".to_string())?;
    match current_notion_desktop_session() {
        Ok(current)
            if failed_session
                .is_none_or(|failed| current.instance_id() != failed.instance_id()) =>
        {
            Ok(current)
        }
        Err(NotionLiveError::Fatal(error)) => Err(error),
        Ok(_) | Err(NotionLiveError::Session(_)) => recover_notion_desktop_session(),
        Err(NotionLiveError::Unavailable(error)) => Err(error.to_string()),
    }
}

fn clone_live_error(error: &NotionLiveError) -> NotionLiveError {
    match error {
        NotionLiveError::Session(error) => NotionLiveError::Session(*error),
        NotionLiveError::Unavailable(error) => NotionLiveError::Unavailable(error.clone()),
        NotionLiveError::Fatal(message) => NotionLiveError::Fatal(message.clone()),
    }
}

fn recovery_error(
    error: NotionLiveError,
    recovery_disposition: NotionRecoveryDisposition,
) -> NotionSessionRecoveryError {
    NotionSessionRecoveryError {
        message: error.to_string(),
        recovery_disposition,
    }
}

fn recovery_error_message(
    error: String,
    recovery_disposition: NotionRecoveryDisposition,
) -> NotionSessionRecoveryError {
    NotionSessionRecoveryError {
        message: format!("automatic Notion session recovery failed: {error}"),
        recovery_disposition,
    }
}

fn session_identity_change(
    initial_user_id: Option<String>,
    current_user_id: String,
) -> NotionSessionIdentityChange {
    match initial_user_id {
        None => NotionSessionIdentityChange::Established {
            user_id: current_user_id,
        },
        Some(previous_user_id) if previous_user_id == current_user_id => {
            NotionSessionIdentityChange::Unchanged
        }
        Some(previous_user_id) => NotionSessionIdentityChange::Changed {
            previous_user_id,
            current_user_id,
        },
    }
}

fn load_stored_notion_desktop_session(
) -> Result<Option<NotionDesktopSession>, StoredNotionSessionError> {
    let store = SecretStore::notnotion()
        .map_err(|error| StoredNotionSessionError::Storage(error.to_string()))?;
    let Some(raw) = store
        .read_secret(NOTION_DESKTOP_SESSION_CACHE_KEY)
        .map_err(|error| {
            StoredNotionSessionError::Storage(format!(
                "failed to read stored Notion Desktop session: {error}"
            ))
        })?
    else {
        return Ok(None);
    };
    let stored = serde_json::from_str::<StoredNotionDesktopSession>(&raw).map_err(|error| {
        StoredNotionSessionError::Invalid(format!(
            "failed to decode stored Notion Desktop session: {error}"
        ))
    })?;
    NotionDesktopSession::new(stored.user_id, stored.cookie_header)
        .map(Some)
        .map_err(StoredNotionSessionError::Invalid)
}

fn capture_and_store_notion_desktop_session() -> Result<NotionDesktopSession, String> {
    let session = capture_notion_desktop_session()?;
    let store = SecretStore::notnotion().map_err(|error| error.to_string())?;
    let raw = serde_json::to_string(&session)
        .map_err(|error| format!("failed to encode stored Notion Desktop session: {error}"))?;
    store
        .upsert_secret(NOTION_DESKTOP_SESSION_CACHE_KEY, &raw)
        .map_err(|error| format!("failed to store Notion Desktop session: {error}"))?;
    Ok(session)
}

fn session_slot() -> &'static Mutex<Option<NotionDesktopSession>> {
    static CURRENT_SESSION: OnceLock<Mutex<Option<NotionDesktopSession>>> = OnceLock::new();
    CURRENT_SESSION.get_or_init(|| Mutex::new(None))
}

fn session_recovery_lock() -> &'static Mutex<()> {
    static SESSION_RECOVERY: OnceLock<Mutex<()>> = OnceLock::new();
    SESSION_RECOVERY.get_or_init(|| Mutex::new(()))
}

fn next_session_instance_id() -> u64 {
    static NEXT_SESSION_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_SESSION_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
}

fn cookie_pairs(cookie_header: &str) -> Vec<(&str, &str)> {
    cookie_header
        .split(';')
        .filter_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (!name.is_empty()).then_some((name, value))
        })
        .collect()
}
