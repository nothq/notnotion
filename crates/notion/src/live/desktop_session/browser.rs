//! Signing in through a Chromium-based browser where Notion Desktop is not
//! installed. notnotion opens Notion's sign-in page in a browser window with a
//! profile of its own, waits until the sign-in finishes, keeps the session
//! cookies and closes the window.

use std::{
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::{Duration, Instant},
};

use super::{
    cdp,
    desktop_app::{first_existing, quiet_command},
    inspector::{block_on, Inspector},
};
use crate::live::credentials::NotionDesktopSession;

const NOTION_SIGN_IN_URL: &str = "https://app.notion.com/login";
const SIGN_IN_WAIT: Duration = Duration::from_secs(10 * 60);
const SIGN_IN_POLL: Duration = Duration::from_secs(1);
const PROFILE_DIR_NAME: &str = "notnotion-sign-in";

pub(super) fn sign_in() -> Result<NotionDesktopSession, String> {
    let browser = installed_browser_path()?.ok_or_else(no_browser_message)?;
    let profile = profile_dir(&browser)?;
    std::fs::create_dir_all(&profile).map_err(|error| {
        format!(
            "failed to create the sign-in browser profile {}: {error}",
            profile.display()
        )
    })?;
    let inspector = Inspector::new()?;
    let mut child = quiet_command(&browser)
        .args(inspector.debugging_args())
        .arg(format!("--user-data-dir={}", profile.display()))
        .args(["--no-first-run", "--no-default-browser-check", "--new-window"])
        .arg(NOTION_SIGN_IN_URL)
        .spawn()
        .map_err(|error| format!("failed to open {} for Notion sign-in: {error}", browser.display()))?;
    let result = wait_for_sign_in(&inspector, &mut child);
    if let Some(browser_web_socket_url) = inspector.browser_web_socket_url() {
        let _ = block_on(cdp::close_browser(browser_web_socket_url));
    }
    if !matches!(child.try_wait(), Ok(Some(_))) {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

fn wait_for_sign_in(
    inspector: &Inspector,
    child: &mut Child,
) -> Result<NotionDesktopSession, String> {
    let deadline = Instant::now() + SIGN_IN_WAIT;
    loop {
        let targets = inspector.targets()?;
        for web_socket_url in targets
            .iter()
            .filter(|target| target.kind == "page")
            .filter_map(|target| target.web_socket_url.clone())
        {
            // Errors here mean the page is mid-navigation or the cookies are
            // still partial; the next poll tries again.
            if let Ok(Ok(Some(session))) = block_on(cdp::signed_in_session(web_socket_url)) {
                return Ok(session);
            }
        }
        // Some launchers hand off to another process and exit at once, so a
        // finished child only counts once its inspector is gone too.
        if targets.is_empty() && matches!(child.try_wait(), Ok(Some(_))) {
            return Err("the sign-in browser closed before Notion sign-in finished".to_string());
        }
        if Instant::now() >= deadline {
            return Err("Notion sign-in did not finish in the browser window".to_string());
        }
        thread::sleep(SIGN_IN_POLL);
    }
}

/// A profile notnotion owns, so the browser accepts the inspector flag and the
/// person's own browser profile stays untouched. Snap and Flatpak browsers can
/// only write inside their own sandbox directories.
fn profile_dir(browser: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    if let (Some(home), Some(name)) = (std::env::var_os("HOME"), browser.file_name()) {
        let home = PathBuf::from(home);
        if browser.starts_with("/snap/bin") {
            return Ok(home.join("snap").join(name).join("common").join(PROFILE_DIR_NAME));
        }
        if browser.parent().is_some_and(|dir| dir.ends_with("flatpak/exports/bin")) {
            return Ok(home.join(".var/app").join(name).join("data").join(PROFILE_DIR_NAME));
        }
    }
    let _ = browser;
    app_model::app_data_dir().map(|dir| dir.join(PROFILE_DIR_NAME))
}

fn no_browser_message() -> String {
    "Notion sign-in needs Notion Desktop, Google Chrome, Microsoft Edge, Brave or Chromium"
        .to_string()
}

#[cfg(target_os = "macos")]
fn installed_browser_path() -> Result<Option<PathBuf>, String> {
    let apps = [
        "Google Chrome.app/Contents/MacOS/Google Chrome",
        "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "Brave Browser.app/Contents/MacOS/Brave Browser",
        "Chromium.app/Contents/MacOS/Chromium",
    ];
    let home_apps = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Applications"));
    let roots = std::iter::once(PathBuf::from("/Applications")).chain(home_apps);
    let candidates = roots
        .flat_map(|root| apps.iter().map(move |app| root.join(app)))
        .collect::<Vec<_>>();
    first_existing(candidates, Path::is_file)
}

/// Edge ships with Windows, so this finds a browser on every Windows machine.
#[cfg(target_os = "windows")]
fn installed_browser_path() -> Result<Option<PathBuf>, String> {
    let installs = [
        ["Google", "Chrome", "Application", "chrome.exe"],
        ["Microsoft", "Edge", "Application", "msedge.exe"],
        ["BraveSoftware", "Brave-Browser", "Application", "brave.exe"],
    ];
    let roots = ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let candidates = installs
        .iter()
        .flat_map(|install| {
            roots
                .iter()
                .map(move |root| install.iter().fold(root.clone(), |path, part| path.join(part)))
        })
        .collect::<Vec<_>>();
    first_existing(candidates, Path::is_file)
}

/// Distribution packages first, then Snap and Flatpak launchers, which pass
/// their arguments on to the browser.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn installed_browser_path() -> Result<Option<PathBuf>, String> {
    let names = [
        "google-chrome",
        "google-chrome-stable",
        "microsoft-edge",
        "microsoft-edge-stable",
        "brave-browser",
        "chromium",
        "chromium-browser",
    ];
    let mut dirs = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    dirs.extend(["/usr/bin", "/snap/bin"].map(PathBuf::from));
    let mut candidates = names
        .iter()
        .flat_map(|name| dirs.iter().map(move |dir| dir.join(name)))
        .filter(|path| !path.starts_with("/snap/bin"))
        .collect::<Vec<_>>();
    candidates.extend(names.iter().map(|name| PathBuf::from("/snap/bin").join(name)));
    let flatpak_apps = [
        "com.google.Chrome",
        "com.microsoft.Edge",
        "com.brave.Browser",
        "org.chromium.Chromium",
    ];
    let flatpak_roots = std::iter::once(PathBuf::from("/var/lib/flatpak/exports/bin")).chain(
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".local/share/flatpak/exports/bin")),
    );
    candidates.extend(
        flatpak_roots.flat_map(|root| flatpak_apps.iter().map(move |app| root.join(app))),
    );
    first_existing(candidates, |path| path.is_file() && !is_snap_wrapper(path))
}

/// Ubuntu's `chromium-browser` is a script that starts the Chromium snap; the
/// snap's own launcher comes later in the search, with the right profile path.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_snap_wrapper(path: &Path) -> bool {
    !path.starts_with("/snap/bin")
        && std::fs::metadata(path).is_ok_and(|metadata| metadata.len() < 64 * 1024)
        && std::fs::read(path).is_ok_and(|bytes| {
            bytes
                .windows(b"/snap/bin/".len())
                .any(|window| window == b"/snap/bin/")
        })
}
