use reqwest::Url;
use serde_json::{json, Value};

mod local_file;
mod remote;
mod upload;

pub(crate) use local_file::production_notion_local_icon_file_api;
pub(crate) use remote::production_notion_remote_image_api;
use upload::transfer_authorized_notion_icon;

use super::{
    credentials::NotionDesktopSession,
    http::{post_private_api_in_space_with_session, NotionPrivateApiEndpoint},
    NotionLiveError,
};
use crate::model::{NotionCustomEmoji, NotionCustomEmojiLibrary, PageShellIcon};

const NOTION_IMAGE_ORIGIN: &str = "https://app.notion.com";
const NOTION_CUSTOM_EMOJI_PAGE_SIZE: usize = 200;

pub(crate) fn notion_page_icon_render_url(
    source: &str,
    block_id: &str,
    space_id: &str,
) -> Result<String, String> {
    notion_image_proxy_url(source, "block", block_id, space_id, 320)
}

pub(crate) fn notion_block_image_render_url(
    source: &str,
    block_id: &str,
    space_id: &str,
    width: u16,
) -> Result<String, String> {
    notion_image_proxy_url(source, "block", block_id, space_id, width)
}

fn notion_image_proxy_url(
    source: &str,
    table: &str,
    record_id: &str,
    space_id: &str,
    width: u16,
) -> Result<String, String> {
    if source.is_empty()
        || table.is_empty()
        || record_id.is_empty()
        || space_id.is_empty()
        || width == 0
    {
        return Err("a Notion image proxy URL requires a source, record, and space".to_string());
    }
    let mut render_url =
        Url::parse(NOTION_IMAGE_ORIGIN).expect("the Notion image proxy base URL must be valid");
    render_url.set_path(&format!("/image/{}", encode_notion_image_source(source)));
    render_url
        .query_pairs_mut()
        .append_pair("table", table)
        .append_pair("id", record_id)
        .append_pair("spaceId", space_id)
        .append_pair("width", &width.to_string())
        .append_pair("cache", "v2");
    Ok(render_url.to_string())
}

fn encode_notion_image_source(source: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(source.len());
    for byte in source.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(byte));
            continue;
        }
        encoded.push('%');
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub(crate) struct NotionIconFile<'a> {
    pub(crate) name: &'a str,
    pub(crate) content_type: &'a str,
    pub(crate) bytes: &'a [u8],
}

pub(crate) fn upload_notion_page_icon(
    session: &NotionDesktopSession,
    block_id: &str,
    space_id: &str,
    file: NotionIconFile<'_>,
) -> Result<PageShellIcon, NotionLiveError> {
    let NotionIconFile {
        name,
        content_type,
        bytes,
    } = file;
    let response: Value = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::GetUploadSpaceFileUrl,
        space_id,
        &json!({
            "bucket": "secure",
            "name": name,
            "contentType": content_type,
            "record": {
                "table": "block",
                "id": block_id,
                "spaceId": space_id,
            },
            "supportExtraHeaders": true,
            "contentLength": bytes.len(),
            "spaceId": space_id,
        }),
    )?;
    let attachment = transfer_authorized_notion_icon(&response, name, content_type, bytes)?;
    let render_url = notion_page_icon_render_url(&attachment, block_id, space_id)?;
    PageShellIcon::external_with_render_url(attachment, render_url).map_err(NotionLiveError::Fatal)
}

pub(crate) fn create_notion_custom_emoji_page_icon(
    session: &NotionDesktopSession,
    space_id: &str,
    emoji_name: &str,
    source: crate::model::CustomEmojiPageIconImage,
) -> Result<PageShellIcon, NotionLiveError> {
    let url = match source {
        crate::model::CustomEmojiPageIconImage::ExternalUrl(url) => {
            PageShellIcon::external(url)?.value
        }
        crate::model::CustomEmojiPageIconImage::LocalFile {
            name,
            content_type,
            bytes,
        } => {
            let response: Value = post_private_api_in_space_with_session(
                session,
                NotionPrivateApiEndpoint::GetUploadGenericFileUrl,
                space_id,
                &json!({
                    "bucket": "public",
                    "name": name,
                    "contentType": content_type,
                    "supportExtraHeaders": true,
                    "contentLength": bytes.len(),
                }),
            )?;
            let url =
                transfer_authorized_notion_icon(&response, &name, &content_type, bytes.as_ref())?;
            PageShellIcon::external(url)?.value
        }
    };
    let response: Value = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::CreateCustomEmoji,
        space_id,
        &json!({
            "spaceId": space_id,
            "name": emoji_name,
            "url": url,
        }),
    )?;
    let custom_emoji_id = response
        .get("customEmojiId")
        .or_else(|| {
            response
                .get("data")
                .and_then(|data| data.get("customEmojiId"))
        })
        .and_then(Value::as_str)
        .ok_or_else(|| "Notion custom-emoji creation omitted its emoji ID".to_string())?;
    PageShellIcon::custom(format!(
        "notion://custom_emoji/{space_id}/{custom_emoji_id}"
    ))
    .map_err(NotionLiveError::Fatal)
}

pub(crate) fn load_notion_custom_emoji_library(
    session: &NotionDesktopSession,
    space_id: &str,
    creation_allowed: bool,
) -> Result<NotionCustomEmojiLibrary, NotionLiveError> {
    if space_id.is_empty() {
        return Err(NotionLiveError::Fatal(
            "custom-emoji loading requires a Notion space".to_string(),
        ));
    }
    let mut custom_emojis = Vec::new();
    let mut offset = 0usize;
    let total_count = loop {
        let (total_count, page) = load_notion_custom_emoji_page(session, space_id, offset)?;
        custom_emojis.extend(page);
        offset = offset.saturating_add(NOTION_CUSTOM_EMOJI_PAGE_SIZE);
        if offset >= total_count {
            break total_count;
        }
    };
    Ok(NotionCustomEmojiLibrary {
        emojis: custom_emojis,
        total_count,
        creation_allowed,
        // Notion's limit is a remotely configured Statsig value, not part of
        // this endpoint's response. The create endpoint remains authoritative
        // until that configuration is available through a stable API.
        limit: None,
    })
}

/// The library's total emoji count and one page of its emojis.
type CustomEmojiPage = (usize, Vec<NotionCustomEmoji>);

fn load_notion_custom_emoji_page(
    session: &NotionDesktopSession,
    space_id: &str,
    offset: usize,
) -> Result<CustomEmojiPage, NotionLiveError> {
    let response: Value = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::GetCustomEmojisForSpace,
        space_id,
        &json!({
            "spaceId": space_id,
            "limit": NOTION_CUSTOM_EMOJI_PAGE_SIZE,
            "offset": offset,
        }),
    )?;
    let total_count = response
        .get("totalCount")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| "Notion custom-emoji loading omitted its total count".to_string())?;
    let records = response
        .get("recordMap")
        .and_then(|record_map| record_map.get("custom_emoji"))
        .and_then(Value::as_object);
    if total_count > offset && records.is_none() {
        return Err(NotionLiveError::Fatal(
            "Notion custom-emoji loading omitted its record map".to_string(),
        ));
    }
    let mut page = Vec::new();
    if let Some(records) = records {
        for (record_id, entry) in records {
            if let Some(custom_emoji) = parse_notion_custom_emoji(space_id, record_id, entry)? {
                page.push(custom_emoji);
            }
        }
    }
    Ok((total_count, page))
}

fn parse_notion_custom_emoji(
    space_id: &str,
    record_id: &str,
    entry: &Value,
) -> Result<Option<NotionCustomEmoji>, String> {
    let record = entry
        .get("value")
        .and_then(|value| value.get("value").or(Some(value)))
        .ok_or_else(|| format!("Notion custom emoji {record_id} omitted its value"))?;
    if record.get("alive").and_then(Value::as_bool) == Some(false) {
        return Ok(None);
    }
    let id = record
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| *id == record_id)
        .ok_or_else(|| format!("Notion custom emoji {record_id} has an invalid ID"))?;
    let record_space_id = record
        .get("space_id")
        .and_then(Value::as_str)
        .filter(|record_space_id| *record_space_id == space_id)
        .ok_or_else(|| format!("Notion custom emoji {record_id} belongs to another space"))?;
    let name = record
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("Notion custom emoji {record_id} omitted its name"))?;
    let source_url = record
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion custom emoji {record_id} omitted its image URL"))?;
    let source_url = PageShellIcon::external(source_url)?.value;
    let pointer =
        PageShellIcon::custom(format!("notion://custom_emoji/{record_space_id}/{id}"))?.value;
    let render_url = notion_image_proxy_url(&source_url, "custom_emoji", id, record_space_id, 320)?;
    Ok(Some(NotionCustomEmoji {
        id: id.to_string(),
        name: name.to_string(),
        pointer,
        render_url,
    }))
}
