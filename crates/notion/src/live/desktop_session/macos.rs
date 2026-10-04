use std::{
    net::TcpListener,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::live::credentials::NotionDesktopSession;

use super::cdp;

const NOTION_BUNDLE_PATH: &str = "/Applications/Notion.app";
const NOTION_BROWSER_WAIT: Duration = Duration::from_secs(60);
const NOTION_QUIT_WAIT: Duration = Duration::from_secs(5);

pub(crate) fn capture_notion_desktop_session() -> Result<NotionDesktopSession, String> {
    NotionDesktopSessionExtractor::new()?.extract()
}

struct NotionDesktopSessionExtractor {
    client: Client,
    port: u16,
}

#[derive(Clone, Debug, Deserialize)]
struct CdpTarget {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "webSocketDebuggerUrl")]
    web_socket_url: Option<String>,
}

impl NotionDesktopSessionExtractor {
    fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build Notion Desktop inspector client: {error}"))?;
        Ok(Self {
            client,
            port: available_local_port()?,
        })
    }

    fn extract(&self) -> Result<NotionDesktopSession, String> {
        let capture_result =
            relaunch_notion_with_debugging(self.port).and_then(|()| self.extract_from_debugger());
        let restore_result = relaunch_notion_without_debugging();
        match (capture_result, restore_result) {
            (Ok(session), Ok(())) => Ok(session),
            (Err(capture_error), Ok(())) => Err(capture_error),
            (Ok(_), Err(restore_error)) => Err(restore_error),
            (Err(capture_error), Err(restore_error)) => Err(format!(
                "{capture_error}; additionally failed to restore Notion Desktop normally: {restore_error}"
            )),
        }
    }

    fn extract_from_debugger(&self) -> Result<NotionDesktopSession, String> {
        let target = self.wait_for_notion_target()?;
        let web_socket_url = target
            .web_socket_url
            .ok_or_else(|| "Notion Desktop did not expose a debugger websocket".to_string())?;
        tokio::runtime::Runtime::new()
            .map_err(|error| format!("failed to start Notion Desktop inspector runtime: {error}"))?
            .block_on(cdp::capture_notion_web_session(web_socket_url))
    }

    fn wait_for_notion_target(&self) -> Result<CdpTarget, String> {
        let deadline = Instant::now() + NOTION_BROWSER_WAIT;
        loop {
            if let Some(target) = self.cdp_targets()?.into_iter().find(is_notion_page_target) {
                return Ok(target);
            }
            if Instant::now() >= deadline {
                return Err(notion_target_timeout_message(self.port));
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    fn cdp_targets(&self) -> Result<Vec<CdpTarget>, String> {
        let url = format!("http://127.0.0.1:{}/json/list", self.port);
        match self.client.get(url).send() {
            Ok(response) if response.status().is_success() => response
                .json::<Vec<CdpTarget>>()
                .map_err(|error| format!("failed to decode Notion Desktop targets: {error}")),
            Ok(_) | Err(_) => Ok(Vec::new()),
        }
    }
}

fn available_local_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to allocate Notion Desktop inspector port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("failed to read Notion Desktop inspector port: {error}"))
}

fn relaunch_notion_with_debugging(port: u16) -> Result<(), String> {
    let app_path = notion_app_path()?;
    quit_notion_desktop()?;
    thread::sleep(Duration::from_millis(1200));
    let status = Command::new("open")
        .arg("-na")
        .arg(&app_path)
        .arg("--args")
        .arg(format!("--remote-debugging-port={port}"))
        .arg("--remote-debugging-address=127.0.0.1")
        .status()
        .map_err(|error| format!("failed to launch Notion Desktop: {error}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "failed to launch Notion Desktop with status {status}"
    ))
}

fn relaunch_notion_without_debugging() -> Result<(), String> {
    let app_path = notion_app_path()?;
    quit_notion_desktop()?;
    thread::sleep(Duration::from_millis(1200));
    let status = Command::new("open")
        .arg("-na")
        .arg(app_path)
        .status()
        .map_err(|error| format!("failed to reopen Notion Desktop normally: {error}"))?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "failed to reopen Notion Desktop normally with status {status}"
    ))
}

fn notion_app_path() -> Result<PathBuf, String> {
    let home =
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Applications/Notion.app"));
    std::iter::once(PathBuf::from(NOTION_BUNDLE_PATH))
        .chain(home)
        .find(|path| path.join("Contents/Info.plist").is_file())
        .ok_or_else(notion_app_missing_message)
}

fn quit_notion_desktop() -> Result<(), String> {
    let _ = Command::new("killall")
        .arg("Notion")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
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

fn notion_process_running() -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg("Notion")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn is_notion_page_target(target: &CdpTarget) -> bool {
    target.kind == "page"
        && target.web_socket_url.is_some()
        && url::Url::parse(&target.url)
            .is_ok_and(|url| url.host_str() == Some("app.notion.com") && url.path() != "/blank")
}

fn notion_app_missing_message() -> String {
    "Notion Desktop is required to import the signed-in Notion session automatically".to_string()
}

fn notion_target_timeout_message(port: u16) -> String {
    format!(
        "Notion Desktop did not expose a signed-in app.notion.com page on 127.0.0.1:{port}; \
open Notion Desktop signed in and try again"
    )
}
