use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::live::credentials::NotionDesktopSession;

const NOTION_APP_URL: &str = "https://app.notion.com/";
const CDP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

type CdpStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct CdpSocket {
    stream: CdpStream,
    next_id: u64,
}

#[derive(Deserialize)]
struct CookieResult {
    cookies: Vec<CdpCookie>,
}

#[derive(Deserialize)]
struct CdpCookie {
    name: String,
    value: String,
}

pub(super) async fn capture_notion_web_session(
    web_socket_url: String,
) -> Result<NotionDesktopSession, String> {
    let (stream, _) = timeout(CDP_CONNECT_TIMEOUT, connect_async(web_socket_url))
        .await
        .map_err(|_| "Notion Desktop CDP connection timed out".to_string())?
        .map_err(|error| format!("failed to connect to Notion Desktop CDP: {error}"))?;
    let mut socket = CdpSocket { stream, next_id: 1 };
    let session = socket.desktop_session().await?;
    socket.close().await?;
    Ok(session)
}

impl CdpSocket {
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        timeout(
            CDP_COMMAND_TIMEOUT,
            self.stream.send(Message::Text(
                json!({ "id": id, "method": method, "params": params }).to_string(),
            )),
        )
        .await
        .map_err(|_| format!("Notion Desktop CDP {method} send timed out"))?
        .map_err(|error| format!("Notion Desktop CDP send failed: {error}"))?;
        timeout(CDP_COMMAND_TIMEOUT, self.wait_for_result(id, method))
            .await
            .map_err(|_| format!("Notion Desktop CDP {method} response timed out"))?
    }

    async fn wait_for_result(&mut self, id: u64, method: &str) -> Result<Value, String> {
        loop {
            let payload = self.next_json().await?;
            if payload.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = payload.get("error") {
                return Err(format!("Notion Desktop CDP {method} failed: {error}"));
            }
            return Ok(payload.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn next_json(&mut self) -> Result<Value, String> {
        loop {
            let Some(message) = timeout(CDP_COMMAND_TIMEOUT, self.stream.next())
                .await
                .map_err(|_| "Notion Desktop CDP read timed out".to_string())?
            else {
                return Err("Notion Desktop CDP connection closed".to_string());
            };
            let text = match message
                .map_err(|error| format!("Notion Desktop CDP read failed: {error}"))?
            {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8(bytes)
                    .map_err(|error| format!("Notion Desktop CDP sent invalid UTF-8: {error}"))?,
                Message::Close(_) => return Err("Notion Desktop CDP connection closed".to_string()),
                _ => continue,
            };
            return serde_json::from_str(&text)
                .map_err(|error| format!("failed to decode Notion Desktop CDP JSON: {error}"));
        }
    }

    async fn desktop_session(&mut self) -> Result<NotionDesktopSession, String> {
        let result = self
            .call("Network.getCookies", json!({ "urls": [NOTION_APP_URL] }))
            .await?;
        let result = serde_json::from_value::<CookieResult>(result)
            .map_err(|error| format!("failed to decode Notion Desktop cookies: {error}"))?;
        let user_id = result
            .cookies
            .iter()
            .find(|cookie| cookie.name == "notion_user_id" && !cookie.value.trim().is_empty())
            .map(|cookie| cookie.value.clone())
            .ok_or_else(|| "Notion Desktop did not expose an active signed-in user".to_string())?;
        let cookie_header = result
            .cookies
            .into_iter()
            .filter(|cookie| !cookie.name.trim().is_empty())
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; ");
        if cookie_header.is_empty() {
            return Err("Notion Desktop did not expose Notion cookies".to_string());
        }
        NotionDesktopSession::new(user_id, cookie_header)
    }

    async fn close(&mut self) -> Result<(), String> {
        timeout(CDP_CLOSE_TIMEOUT, self.stream.close(None))
            .await
            .map_err(|_| "Notion Desktop CDP close timed out".to_string())?
            .map_err(|error| format!("failed to close Notion Desktop CDP connection: {error}"))
    }
}
