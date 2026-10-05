//! Signing in by borrowing the Notion web session. With Notion Desktop
//! installed, notnotion relaunches it with its inspector open and reads the
//! session cookies. Without it (Notion ships no Linux app), notnotion opens
//! Notion's sign-in page in a Chromium-based browser and waits for the sign-in.

mod browser;
mod cdp;
mod desktop_app;
mod inspector;

use super::credentials::NotionDesktopSession;

pub(crate) fn capture_notion_desktop_session() -> Result<NotionDesktopSession, String> {
    match desktop_app::installed_notion_app_path()? {
        Some(app_path) => desktop_app::capture_session(&app_path),
        None => browser::sign_in(),
    }
}
