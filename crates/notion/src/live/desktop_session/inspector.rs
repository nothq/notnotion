//! The Chrome DevTools HTTP endpoint of a Notion Desktop or browser process
//! that notnotion launched with `--remote-debugging-port`.

use std::{
    future::Future,
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::Client;
use serde::Deserialize;

pub(super) struct Inspector {
    client: Client,
    port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct CdpTarget {
    #[serde(default, rename = "type")]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) url: String,
    #[serde(default, rename = "webSocketDebuggerUrl")]
    pub(super) web_socket_url: Option<String>,
}

#[derive(Deserialize)]
struct BrowserVersion {
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_url: String,
}

impl Inspector {
    pub(super) fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build Notion session inspector client: {error}"))?;
        Ok(Self {
            client,
            port: available_local_port()?,
        })
    }

    pub(super) const fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn debugging_args(&self) -> [String; 2] {
        [
            format!("--remote-debugging-port={}", self.port),
            "--remote-debugging-address=127.0.0.1".to_string(),
        ]
    }

    /// Polls until a target matches or `wait` runs out.
    pub(super) fn wait_for_target(
        &self,
        wait: Duration,
        matches: impl Fn(&CdpTarget) -> bool,
    ) -> Result<Option<CdpTarget>, String> {
        let deadline = Instant::now() + wait;
        loop {
            if let Some(target) = self.targets()?.into_iter().find(&matches) {
                return Ok(Some(target));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    /// The open targets, or none while the process is still starting.
    pub(super) fn targets(&self) -> Result<Vec<CdpTarget>, String> {
        let url = format!("http://127.0.0.1:{}/json/list", self.port);
        match self.client.get(url).send() {
            Ok(response) if response.status().is_success() => response
                .json::<Vec<CdpTarget>>()
                .map_err(|error| format!("failed to decode Notion session targets: {error}")),
            Ok(_) | Err(_) => Ok(Vec::new()),
        }
    }

    /// The browser-level websocket, for closing the whole process.
    pub(super) fn browser_web_socket_url(&self) -> Option<String> {
        let url = format!("http://127.0.0.1:{}/json/version", self.port);
        let response = self.client.get(url).send().ok()?;
        response
            .json::<BrowserVersion>()
            .ok()
            .map(|version| version.web_socket_url)
    }
}

pub(super) fn block_on<T>(future: impl Future<Output = T>) -> Result<T, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(|runtime| runtime.block_on(future))
        .map_err(|error| format!("failed to start Notion session inspector runtime: {error}"))
}

fn available_local_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to allocate Notion session inspector port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("failed to read Notion session inspector port: {error}"))
}
