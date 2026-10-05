#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::{collections::HashMap, fs, io::ErrorKind, path::PathBuf};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use serde::Deserialize;

use crate::live::NotionLiveError;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::live::NotionSessionFailure;

use super::workspace::NonEmptyNotionId;

#[cfg(any(target_os = "macos", target_os = "windows"))]
const UNGROUPED_TAB_SECTION_ID: &str = "ungrouped";

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn restored_notion_space_id(
    active_user_id: &str,
) -> Result<RestoredNotionSpace, String> {
    let Some(state_path) = notion_state_path() else {
        return Ok(RestoredNotionSpace::Unavailable);
    };
    let raw = match fs::read_to_string(&state_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(RestoredNotionSpace::Unavailable);
        }
        Err(error) => {
            return Err(format!(
                "failed to read Notion Desktop restoration state at {}: {error}",
                state_path.display()
            ));
        }
    };
    let state = serde_json::from_str::<NotionDesktopState>(&raw).map_err(|error| {
        format!(
            "failed to decode Notion Desktop restoration state at {}: {error}",
            state_path.display()
        )
    })?;
    match state
        .history
        .and_then(|history| history.app_restoration_state)
    {
        Some(restoration) => restoration.active_space_id(active_user_id),
        None => Ok(RestoredNotionSpace::Unavailable),
    }
}

/// Notion Desktop's window and tab state, in its Electron user data directory.
#[cfg(target_os = "macos")]
fn notion_state_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/Notion/state.json"))
}

#[cfg(target_os = "windows")]
fn notion_state_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Notion").join("state.json"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn restored_notion_space_id(
    _active_user_id: &str,
) -> Result<RestoredNotionSpace, String> {
    Ok(RestoredNotionSpace::Unavailable)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn notion_desktop_restoration_has_different_user(
    active_user_id: &str,
) -> Result<bool, String> {
    restored_notion_space_id(active_user_id).map(|space| space.is_different_user())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn notion_desktop_restoration_has_different_user(
    _active_user_id: &str,
) -> Result<bool, String> {
    Ok(false)
}

pub(super) enum RestoredNotionSpace {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Current(NonEmptyNotionId),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    DifferentUser,
    Unavailable,
}

impl RestoredNotionSpace {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub(super) fn is_different_user(&self) -> bool {
        matches!(self, Self::DifferentUser)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub(super) const fn is_different_user(&self) -> bool {
        false
    }

    pub(super) fn space_id(&self) -> Result<Option<&NonEmptyNotionId>, NotionLiveError> {
        match self {
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Self::Current(space_id) => Ok(Some(space_id)),
            Self::Unavailable => Ok(None),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            Self::DifferentUser => Err(NotionLiveError::Session(
                NotionSessionFailure::ActiveUserMismatch,
            )),
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionDesktopState {
    #[serde(default)]
    history: Option<NotionDesktopHistory>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionDesktopHistory {
    #[serde(default, rename = "appRestorationState")]
    app_restoration_state: Option<NotionAppRestorationState>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionAppRestorationState {
    windows: Vec<NotionRestoredWindow>,
    #[serde(rename = "tabSpaces")]
    tab_spaces: Vec<NotionRestoredTabSpace>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl NotionAppRestorationState {
    fn focused_window(&self) -> Result<Option<&NotionRestoredWindow>, String> {
        let Some(max_focus_order) = self.windows.iter().map(|window| window.focus_order).max()
        else {
            return Ok(None);
        };
        let mut focused_windows = self
            .windows
            .iter()
            .filter(|window| window.focus_order == max_focus_order);
        let focused = focused_windows
            .next()
            .expect("maximum Notion window focus order must have an owner");
        if focused_windows.next().is_some() {
            return Err(format!(
                "Notion Desktop restoration state has multiple focused windows at order {max_focus_order}"
            ));
        }
        Ok(Some(focused))
    }

    fn active_space_id(&self, active_user_id: &str) -> Result<RestoredNotionSpace, String> {
        let Some(focused) = self.focused_window()? else {
            return Ok(RestoredNotionSpace::Unavailable);
        };
        let active_tab_id = match focused
            .active_tab_id_map
            .get(&focused.active_tab_section_id)
        {
            Some(Some(active_tab_id)) => active_tab_id,
            Some(None) | None => return Ok(RestoredNotionSpace::Unavailable),
        };
        let tabs = if focused.active_tab_section_id == UNGROUPED_TAB_SECTION_ID {
            &focused.tabs
        } else {
            &self
                .tab_spaces
                .iter()
                .find(|space| space.tab_space_id == focused.active_tab_section_id)
                .ok_or_else(|| {
                    format!(
                        "Notion Desktop active tab section {} has no matching tab space",
                        focused.active_tab_section_id
                    )
                })?
                .tabs
        };
        let mut matching_tabs = tabs.iter().filter(|tab| tab.tab_id == *active_tab_id);
        let active_tab = matching_tabs.next().ok_or_else(|| {
            format!(
                "Notion Desktop active tab {active_tab_id} is absent from section {}",
                focused.active_tab_section_id
            )
        })?;
        if matching_tabs.next().is_some() {
            return Err(format!(
                "Notion Desktop section {} contains duplicate active tab {active_tab_id}",
                focused.active_tab_section_id
            ));
        }
        let Some(app_state) = active_tab.app_store_state.as_ref() else {
            return Ok(RestoredNotionSpace::Unavailable);
        };
        let (Some(current_user_id), Some(current_space_id)) =
            (&app_state.current_user_id, &app_state.current_space_id)
        else {
            return Ok(RestoredNotionSpace::Unavailable);
        };
        if current_user_id.0 != active_user_id {
            return Ok(RestoredNotionSpace::DifferentUser);
        }
        Ok(RestoredNotionSpace::Current(current_space_id.clone()))
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionRestoredWindow {
    #[serde(rename = "focusOrder")]
    focus_order: u64,
    #[serde(rename = "activeTabIdMap")]
    active_tab_id_map: HashMap<String, Option<String>>,
    #[serde(rename = "activeTabSectionId")]
    active_tab_section_id: String,
    tabs: Vec<NotionRestoredTab>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionRestoredTabSpace {
    #[serde(rename = "tabSpaceId")]
    tab_space_id: String,
    tabs: Vec<NotionRestoredTab>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionRestoredTab {
    #[serde(rename = "tabId")]
    tab_id: String,
    #[serde(default, rename = "appStoreState")]
    app_store_state: Option<NotionRestoredAppStoreState>,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Deserialize)]
struct NotionRestoredAppStoreState {
    #[serde(default, rename = "currentUserId")]
    current_user_id: Option<NonEmptyNotionId>,
    #[serde(default, rename = "currentSpaceId")]
    current_space_id: Option<NonEmptyNotionId>,
}
