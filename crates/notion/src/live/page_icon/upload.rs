use std::{sync::OnceLock, time::Duration};

use reqwest::{
    blocking::{multipart, Client, RequestBuilder},
    header::{HeaderName, HeaderValue, CONTENT_TYPE},
    redirect::Policy as RedirectPolicy,
    Url,
};
use serde_json::{Map, Value};

const NOTION_UPLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const NOTION_UPLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub(super) fn transfer_authorized_notion_icon(
    response: &Value,
    name: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let data = authorized_upload_data(response)?;
    let url = required_upload_string(data, "url")?.to_string();
    match required_upload_string(data, "type")? {
        "POST" => upload_notion_page_icon_post(data, name, content_type, bytes)?,
        "PUT" => upload_notion_page_icon_put(data, content_type, bytes)?,
        _ => {
            return Err("Notion returned an unsupported page-icon upload method".to_string());
        }
    }
    Ok(url)
}

fn authorized_upload_data(response: &Value) -> Result<&Map<String, Value>, String> {
    let response_type = response.get("type").and_then(Value::as_str);
    match response_type {
        Some("POST" | "PUT") => response.as_object(),
        Some("success") => response.get("data").and_then(Value::as_object),
        _ => None,
    }
    .ok_or_else(|| "Notion did not authorize the page-icon upload".to_string())
}

fn upload_notion_page_icon_post(
    data: &Map<String, Value>,
    name: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let url = validated_signed_upload_url(required_upload_string(data, "signedUploadPostUrl")?)?;
    let fields = data
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(|| "Notion page-icon POST upload omitted its signed fields".to_string())?;
    let mut form = multipart::Form::new();
    for (field, value) in fields {
        let value = value.as_str().ok_or_else(|| {
            "Notion page-icon POST upload returned a non-string field".to_string()
        })?;
        form = form.text(field.clone(), value.to_string());
    }
    let file = multipart::Part::bytes(bytes.to_vec())
        .file_name(name.to_string())
        .mime_str(content_type)
        .map_err(|_| "Notion page-icon upload has an invalid content type".to_string())?;
    form = form.part("file", file);
    let request = notion_upload_client()?.post(url).multipart(form);
    let request = apply_upload_headers(request, data.get("postHeaders"), true)?;
    send_upload_request(request)
}

fn upload_notion_page_icon_put(
    data: &Map<String, Value>,
    content_type: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let url = validated_signed_upload_url(required_upload_string(data, "signedPutUrl")?)?;
    let request = notion_upload_client()?
        .put(url)
        .header(CONTENT_TYPE, content_type)
        .body(bytes.to_vec());
    let request = apply_upload_headers(request, data.get("putHeaders"), false)?;
    send_upload_request(request)
}

fn required_upload_string<'a>(
    data: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, String> {
    data.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Notion page-icon upload omitted {field}"))
}

fn validated_signed_upload_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value)
        .map_err(|_| "Notion returned an invalid signed page-icon upload URL".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Notion returned an unsafe signed page-icon upload URL".to_string());
    }
    Ok(url)
}

fn apply_upload_headers(
    mut request: RequestBuilder,
    headers: Option<&Value>,
    multipart_request: bool,
) -> Result<RequestBuilder, String> {
    let Some(headers) = headers.filter(|headers| !headers.is_null()) else {
        return Ok(request);
    };
    match headers {
        Value::Array(headers) => {
            for header in headers {
                let header = header.as_object().ok_or_else(|| {
                    "Notion page-icon upload returned an invalid signed request header".to_string()
                })?;
                let name = required_upload_string(header, "name")?;
                let value = required_upload_string(header, "value")?;
                request = apply_upload_header(request, name, value, multipart_request)?;
            }
        }
        Value::Object(headers) => {
            for (name, value) in headers {
                let value = value.as_str().ok_or_else(|| {
                    "Notion returned a non-string signed upload header".to_string()
                })?;
                request = apply_upload_header(request, name, value, multipart_request)?;
            }
        }
        _ => {
            return Err(
                "Notion page-icon upload returned invalid signed request headers".to_string(),
            );
        }
    }
    Ok(request)
}

fn apply_upload_header(
    request: RequestBuilder,
    name: &str,
    value: &str,
    multipart_request: bool,
) -> Result<RequestBuilder, String> {
    if name.eq_ignore_ascii_case("host")
        || name.eq_ignore_ascii_case("content-length")
        || (multipart_request && name.eq_ignore_ascii_case("content-type"))
    {
        return Ok(request);
    }
    let name = HeaderName::from_bytes(name.as_bytes())
        .map_err(|_| "Notion returned an invalid signed upload header name".to_string())?;
    let value = HeaderValue::from_str(value)
        .map_err(|_| "Notion returned an invalid signed upload header value".to_string())?;
    Ok(request.header(name, value))
}

fn send_upload_request(request: RequestBuilder) -> Result<(), String> {
    let response = request
        .send()
        .map_err(|_| "failed to transfer the page-icon image to Notion storage".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Notion page-icon storage returned HTTP {}",
            response.status().as_u16()
        ));
    }
    Ok(())
}

fn notion_upload_client() -> Result<Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .redirect(RedirectPolicy::none())
                .connect_timeout(NOTION_UPLOAD_CONNECT_TIMEOUT)
                .timeout(NOTION_UPLOAD_REQUEST_TIMEOUT)
                .build()
                .map_err(|error| format!("failed to build Notion upload client: {error}"))
        })
        .clone()
}
