#[cfg(not(target_os = "macos"))]
use super::credentials::NotionDesktopSession;

#[cfg(target_os = "macos")]
mod cdp;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub(crate) use macos::capture_notion_desktop_session;

#[cfg(not(target_os = "macos"))]
pub(crate) fn capture_notion_desktop_session() -> Result<NotionDesktopSession, String> {
    Err("automatic Notion Desktop session import is supported only on macOS".to_string())
}
