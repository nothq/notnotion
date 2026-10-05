//! Finding, quitting and relaunching the installed Notion Desktop app.

use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::{
    cdp,
    inspector::{block_on, CdpTarget, Inspector},
};
use crate::live::credentials::NotionDesktopSession;

const NOTION_PAGE_WAIT: Duration = Duration::from_secs(60);
const NOTION_QUIT_WAIT: Duration = Duration::from_secs(5);

/// Relaunches Notion Desktop with its inspector open, reads the session, then
/// relaunches it normally.
pub(super) fn capture_session(app_path: &Path) -> Result<NotionDesktopSession, String> {
    let inspector = Inspector::new()?;
    let capture_result = relaunch_notion_with_debugging(app_path, &inspector)
        .and_then(|()| capture_from_inspector(&inspector));
    let restore_result = relaunch_notion_normally(app_path);
    match (capture_result, restore_result) {
        (Ok(session), Ok(())) => Ok(session),
        (Err(capture_error), Ok(())) => Err(capture_error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(capture_error), Err(restore_error)) => Err(format!(
            "{capture_error}; additionally failed to restore Notion Desktop normally: {restore_error}"
        )),
    }
}

fn capture_from_inspector(inspector: &Inspector) -> Result<NotionDesktopSession, String> {
    let target = inspector
        .wait_for_target(NOTION_PAGE_WAIT, is_notion_page_target)?
        .ok_or_else(|| notion_target_timeout_message(inspector.port()))?;
    let web_socket_url = target
        .web_socket_url
        .ok_or_else(|| "Notion Desktop did not expose a debugger websocket".to_string())?;
    block_on(cdp::capture_notion_web_session(web_socket_url))?
}

fn relaunch_notion_with_debugging(app_path: &Path, inspector: &Inspector) -> Result<(), String> {
    quit_notion_desktop()?;
    thread::sleep(Duration::from_millis(1200));
    launch_notion(app_path, &inspector.debugging_args())
        .map_err(|error| format!("failed to launch Notion Desktop: {error}"))
}

fn relaunch_notion_normally(app_path: &Path) -> Result<(), String> {
    quit_notion_desktop()?;
    thread::sleep(Duration::from_millis(1200));
    launch_notion(app_path, &[])
        .map_err(|error| format!("failed to reopen Notion Desktop normally: {error}"))
}

fn quit_notion_desktop() -> Result<(), String> {
    force_quit_notion_desktop();
    if wait_for_notion_exit(NOTION_QUIT_WAIT) {
        return Ok(());
    }
    Err("Notion Desktop did not exit for session relaunch".to_string())
}

fn wait_for_notion_exit(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !notion_process_running() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn is_notion_page_target(target: &CdpTarget) -> bool {
    target.kind == "page"
        && target.web_socket_url.is_some()
        && url::Url::parse(&target.url)
            .is_ok_and(|url| url.host_str() == Some("app.notion.com") && url.path() != "/blank")
}

fn notion_target_timeout_message(port: u16) -> String {
    format!(
        "Notion Desktop did not expose a signed-in app.notion.com page on 127.0.0.1:{port}; \
open Notion Desktop signed in and try again"
    )
}

pub(super) fn quiet_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Returns the first candidate that exists and passes `is_valid`, so the
/// search order decides which install wins when several are present.
pub(super) fn first_existing(
    candidates: impl IntoIterator<Item = PathBuf>,
    is_valid: impl Fn(&Path) -> bool,
) -> Result<Option<PathBuf>, String> {
    for path in candidates {
        match std::fs::metadata(&path) {
            Ok(_) if is_valid(&path) => return Ok(Some(path)),
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "failed to inspect application {}: {error}",
                    path.display()
                ))
            }
        }
    }
    Ok(None)
}

#[cfg(target_os = "macos")]
pub(super) fn installed_notion_app_path() -> Result<Option<PathBuf>, String> {
    let home = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Applications/Notion.app"));
    let candidates = std::iter::once(PathBuf::from("/Applications/Notion.app")).chain(home);
    first_existing(candidates, |path| path.join("Contents/Info.plist").is_file())
}

#[cfg(target_os = "macos")]
fn launch_notion(app_path: &Path, args: &[String]) -> Result<(), String> {
    let mut command = Command::new("open");
    command.arg("-na").arg(app_path);
    if !args.is_empty() {
        command.arg("--args").args(args);
    }
    let status = command.status().map_err(|error| error.to_string())?;
    if status.success() {
        return Ok(());
    }
    Err(format!("open exited with status {status}"))
}

#[cfg(target_os = "macos")]
fn notion_process_running() -> bool {
    quiet_command("pgrep")
        .args(["-x", "Notion"])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "macos")]
fn force_quit_notion_desktop() {
    let _ = quiet_command("killall").arg("Notion").status();
}

/// Notion's per-user installer, then a machine-wide install.
#[cfg(target_os = "windows")]
pub(super) fn installed_notion_app_path() -> Result<Option<PathBuf>, String> {
    let mut candidates = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Programs").join("Notion").join("Notion.exe"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("Notion").join("Notion.exe"));
    }
    first_existing(candidates, Path::is_file)
}

#[cfg(target_os = "windows")]
fn launch_notion(app_path: &Path, args: &[String]) -> Result<(), String> {
    quiet_command(app_path)
        .args(args)
        .spawn()
        .map(drop)
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn notion_process_running() -> bool {
    quiet_command("tasklist")
        .args(["/FI", "IMAGENAME eq Notion.exe", "/NH"])
        .stdout(Stdio::piped())
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .to_ascii_lowercase()
                .contains("notion.exe")
        })
}

#[cfg(target_os = "windows")]
fn force_quit_notion_desktop() {
    let _ = quiet_command("taskkill")
        .args(["/IM", "Notion.exe", "/T", "/F"])
        .status();
}

/// Notion ships no Linux app, so sign-in goes through a browser there.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn installed_notion_app_path() -> Result<Option<PathBuf>, String> {
    Ok(None)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_notion(_: &Path, _: &[String]) -> Result<(), String> {
    Err("Notion Desktop is not available on this platform".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn notion_process_running() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn force_quit_notion_desktop() {}
