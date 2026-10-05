use std::{
    io::Read,
    sync::{Arc, OnceLock},
    time::Duration,
};

use remote_image_model::{RemoteImageApi, RemoteImageData};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, LOCATION},
    redirect::Policy as RedirectPolicy,
    Url,
};
use serde_json::{json, Value};

use super::{notion_image_proxy_url, NOTION_IMAGE_ORIGIN};
use crate::live::{
    credentials::{with_notion_session_recovery, NotionDesktopSession},
    http::{post_private_api_with_session, NotionPrivateApiEndpoint},
    NotionLiveError, NotionSessionFailure,
};
use crate::model::PageShellIcon;

const NOTION_NAMED_ICON_MAX_BYTES: usize = 64 * 1024;
const NOTION_IMAGE_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const NOTION_IMAGE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) fn production_notion_remote_image_api() -> Arc<dyn RemoteImageApi> {
    Arc::new(ProductionNotionRemoteImageApi)
}

struct ProductionNotionRemoteImageApi;

impl RemoteImageApi for ProductionNotionRemoteImageApi {
    fn load_remote_image(&self, value: &str) -> Result<Option<RemoteImageData>, String> {
        if value.starts_with("notion://custom_emoji/") {
            return load_authenticated_notion_image(value).map(Some);
        }
        let url = Url::parse(value).map_err(|_| "invalid Notion image URL".to_string())?;
        if is_notion_named_icon_candidate(&url) {
            return load_notion_named_icon(url).map(Some);
        }
        if is_notion_image_proxy_url(&url) {
            return load_authenticated_notion_image(value).map(Some);
        }
        load_public_notion_image(&url).map(Some)
    }
}

fn load_authenticated_notion_image(value: &str) -> Result<RemoteImageData, String> {
    with_notion_session_recovery(|session| {
        let url = if value.starts_with("notion://custom_emoji/") {
            resolve_notion_custom_emoji_url(session, value)?
        } else {
            Url::parse(value)
                .map_err(|_| NotionLiveError::Fatal("invalid Notion image URL".to_string()))?
        };
        let signed_url = resolve_authenticated_notion_image_url(session, url)?;
        load_public_notion_image(&signed_url).map_err(NotionLiveError::Fatal)
    })
    .map(|recovery| recovery.into_value())
    .map_err(|error| error.to_string())
}

fn load_public_notion_image(url: &Url) -> Result<RemoteImageData, String> {
    remote_image::load_public_remote_image_or_svg_data(url.as_str())
        .map_err(notion_public_image_error)
}

fn notion_public_image_error(error: String) -> String {
    error
        .split_once("unexpected HTTP ")
        .map(|(_, status)| format!("Notion image returned HTTP {status}"))
        .unwrap_or_else(|| "Notion image fetch or validation failed".to_string())
}

fn is_notion_named_icon_candidate(url: &Url) -> bool {
    url.host_str() == Some("app.notion.com") && url.path().starts_with("/icons/")
}

fn load_notion_named_icon(url: Url) -> Result<RemoteImageData, String> {
    validate_notion_named_icon_url(&url)?;
    let response = notion_named_icon_client()?
        .get(url.clone())
        .header(ACCEPT, "image/svg+xml")
        .send()
        .map_err(|error| format!("Notion named-icon request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "{url} returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if content_type != Some("image/svg+xml") {
        return Err(format!(
            "{url} returned unsupported Content-Type {}",
            content_type.unwrap_or("<missing>")
        ));
    }
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > NOTION_NAMED_ICON_MAX_BYTES)
    {
        return Err(format!("{url} exceeded the named-icon size limit"));
    }
    let mut bytes = Vec::new();
    response
        .take((NOTION_NAMED_ICON_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read {url}: {error}"))?;
    if bytes.len() > NOTION_NAMED_ICON_MAX_BYTES {
        return Err(format!("{url} exceeded the named-icon size limit"));
    }
    remote_image::validate_safe_svg(&bytes)
        .map_err(|error| format!("{url} returned an unsafe SVG: {error}"))?;
    Ok(RemoteImageData {
        bytes,
        mimetype: "image/svg+xml".to_string(),
    })
}

fn validate_notion_named_icon_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https"
        || url.host_str() != Some("app.notion.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid Notion named-icon URL".to_string());
    }
    let asset = url
        .path()
        .strip_prefix("/icons/")
        .and_then(|path| path.strip_suffix(".svg"))
        .ok_or_else(|| "invalid Notion named-icon path".to_string())?;
    let (slug, color) = asset
        .rsplit_once('_')
        .ok_or_else(|| "invalid Notion named-icon asset".to_string())?;
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || !matches!(
            color,
            "gray"
                | "lightgray"
                | "brown"
                | "yellow"
                | "orange"
                | "green"
                | "blue"
                | "purple"
                | "pink"
                | "red"
        )
    {
        return Err("invalid Notion named-icon asset".to_string());
    }
    let query = url.query_pairs().collect::<Vec<_>>();
    if query.len() != 1 || query[0].0 != "mode" || !matches!(query[0].1.as_ref(), "light" | "dark")
    {
        return Err("invalid Notion named-icon mode".to_string());
    }
    Ok(())
}

fn notion_named_icon_client() -> Result<&'static Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .connect_timeout(Duration::from_secs(4))
                .timeout(Duration::from_secs(8))
                .redirect(RedirectPolicy::none())
                .build()
                .map_err(|error| format!("failed to build Notion named-icon client: {error}"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn resolve_notion_custom_emoji_url(
    session: &NotionDesktopSession,
    value: &str,
) -> Result<Url, NotionLiveError> {
    let icon = PageShellIcon::custom(value)?;
    let (space_id, custom_emoji_id) = icon.custom_pointer()?;
    let response: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpace,
        &json!({
            "requests": [{
                "pointer": {
                    "table": "custom_emoji",
                    "id": custom_emoji_id,
                    "spaceId": space_id,
                },
                "version": -1,
            }],
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    let entry = response
        .get("recordMap")
        .and_then(|record_map| record_map.get("custom_emoji"))
        .and_then(|custom_emojis| custom_emojis.get(custom_emoji_id))
        .ok_or_else(|| "Notion custom emoji hydration omitted its record".to_string())?;
    let record = entry
        .get("value")
        .and_then(|value| value.get("value").or(Some(value)))
        .unwrap_or(entry);
    let url = record
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "Notion custom emoji record omitted its image URL".to_string())?;
    let url =
        Url::parse(url).map_err(|_| "Notion custom emoji has an invalid image URL".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(NotionLiveError::Fatal(
            "Notion custom emoji has an unsafe image URL".to_string(),
        ));
    }
    let render_url =
        notion_image_proxy_url(url.as_str(), "custom_emoji", custom_emoji_id, space_id, 320)?;
    Url::parse(&render_url).map_err(|_| {
        NotionLiveError::Fatal("Notion custom emoji proxy URL could not be created".to_string())
    })
}

fn is_notion_image_proxy_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("app.notion.com")
        && url.port().is_none()
        && url.path().starts_with("/image/")
        && url.username().is_empty()
        && url.password().is_none()
}

fn resolve_authenticated_notion_image_url(
    session: &NotionDesktopSession,
    mut url: Url,
) -> Result<Url, NotionLiveError> {
    set_notion_image_session_query(&mut url, session.user_id());
    let response = notion_image_redirect_client()?
        .get(url.clone())
        .header(ACCEPT, "image/png,image/jpeg,image/gif,image/webp")
        .header("origin", NOTION_IMAGE_ORIGIN)
        .header("referer", format!("{NOTION_IMAGE_ORIGIN}/"))
        .header("x-notion-active-user-header", session.user_id())
        .header("cookie", session.cookie_header())
        .send()
        .map_err(|_| {
            NotionLiveError::Fatal("failed to resolve authenticated Notion image".to_string())
        })?;
    let status = response.status();
    if matches!(status.as_u16(), 401 | 403) {
        return Err(NotionLiveError::Session(NotionSessionFailure::Rejected {
            status: status.as_u16(),
        }));
    }
    if !status.is_redirection() {
        return Err(NotionLiveError::Fatal(format!(
            "Notion image proxy returned unexpected HTTP {}",
            status.as_u16()
        )));
    }
    let location = response
        .headers()
        .get(LOCATION)
        .ok_or_else(|| "Notion image proxy omitted its redirect target".to_string())?
        .to_str()
        .map_err(|_| {
            NotionLiveError::Fatal(
                "Notion image proxy returned an invalid redirect target".to_string(),
            )
        })?;
    url.join(location).map_err(|_| {
        NotionLiveError::Fatal("Notion image proxy returned an invalid redirect URL".to_string())
    })
}

fn set_notion_image_session_query(url: &mut Url, user_id: &str) {
    let stable_query = url
        .query_pairs()
        .filter(|(name, _)| name != "userId" && name != "imgBuildSrc")
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);
    let mut query = url.query_pairs_mut();
    query.extend_pairs(stable_query);
    query
        .append_pair("userId", user_id)
        .append_pair("imgBuildSrc", "requestProxiedImageUrl");
}

fn notion_image_redirect_client() -> Result<Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .redirect(RedirectPolicy::none())
                .connect_timeout(NOTION_IMAGE_CONNECT_TIMEOUT)
                .timeout(NOTION_IMAGE_REQUEST_TIMEOUT)
                .build()
                .map_err(|error| format!("failed to build Notion image client: {error}"))
        })
        .clone()
}
