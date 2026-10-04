use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageShellIcon {
    pub kind: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_url: Option<String>,
}

impl PageShellIcon {
    const MAX_EXTERNAL_URL_LENGTH: usize = 8 * 1024;

    pub(crate) fn persisted_value_eq(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => left.kind == right.kind && left.value == right.value,
            (None, None) => true,
            _ => false,
        }
    }

    pub fn emoji(value: impl Into<String>) -> Self {
        Self {
            kind: "emoji".to_string(),
            value: value.into(),
            render_url: None,
        }
    }

    pub fn named(value: impl Into<String>) -> Self {
        Self {
            kind: "named".to_string(),
            value: value.into(),
            render_url: None,
        }
    }

    pub fn external(value: impl Into<String>) -> Result<Self, String> {
        let value = canonical_external_icon_url(value.into())?;
        Ok(Self {
            kind: "external".to_string(),
            value,
            render_url: None,
        })
    }

    pub fn custom(value: impl Into<String>) -> Result<Self, String> {
        let value = canonical_custom_emoji_value(value.into())?;
        Ok(Self {
            kind: "custom".to_string(),
            value,
            render_url: None,
        })
    }

    pub(crate) fn external_with_render_url(
        value: impl Into<String>,
        render_url: impl Into<String>,
    ) -> Result<Self, String> {
        let value = value.into();
        let value = if value.starts_with("attachment:") {
            validate_attachment_icon_value(&value)?;
            value
        } else {
            canonical_external_icon_url(value)?
        };
        let render_url = canonical_external_icon_url(render_url.into())?;
        Ok(Self {
            kind: "external".to_string(),
            value,
            render_url: Some(render_url),
        })
    }

    pub fn render_value(&self) -> &str {
        self.render_url.as_deref().unwrap_or(&self.value)
    }

    pub(crate) fn validate_external(&self) -> Result<(), String> {
        if self.kind != "external" {
            return Err("only external Notion icons have external URL validation".to_string());
        }
        if self.value.starts_with("attachment:") {
            validate_attachment_icon_value(&self.value)?;
            if self.render_url.is_none() {
                return Err("a Notion attachment icon requires a render URL".to_string());
            }
        } else if canonical_external_icon_url(self.value.clone())? != self.value {
            return Err("a Notion external icon URL must be canonical".to_string());
        }
        if let Some(render_url) = &self.render_url {
            if canonical_external_icon_url(render_url.clone())? != *render_url {
                return Err("a Notion external icon render URL must be canonical".to_string());
            }
        }
        Ok(())
    }

    pub(crate) fn validate_custom(&self) -> Result<(), String> {
        if self.kind != "custom" {
            return Err("only custom Notion icons have custom-emoji validation".to_string());
        }
        if canonical_custom_emoji_value(self.value.clone())? != self.value {
            return Err("a Notion custom emoji pointer must be canonical".to_string());
        }
        if self.render_url.is_some() {
            return Err(
                "a Notion custom emoji pointer cannot contain a persisted render URL".to_string(),
            );
        }
        Ok(())
    }

    pub(crate) fn custom_pointer(&self) -> Result<(&str, &str), String> {
        self.validate_custom()?;
        custom_emoji_pointer(&self.value)
    }
}

fn canonical_external_icon_url(value: String) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > PageShellIcon::MAX_EXTERNAL_URL_LENGTH {
        return Err("a Notion external icon requires a reasonably sized URL".to_string());
    }
    let mut url = url::Url::parse(value)
        .map_err(|error| format!("invalid Notion external icon URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("a Notion external icon URL must use HTTPS".to_string());
    }
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err(
            "a Notion external icon URL requires a host and cannot contain credentials".to_string(),
        );
    }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn validate_attachment_icon_value(value: &str) -> Result<(), String> {
    let (file_id, file_name) = value
        .strip_prefix("attachment:")
        .and_then(|value| value.split_once(':'))
        .ok_or_else(|| "invalid Notion attachment icon value".to_string())?;
    uuid::Uuid::parse_str(file_id)
        .map_err(|error| format!("invalid Notion attachment icon ID: {error}"))?;
    if file_name.is_empty() || file_name.len() > 1024 || file_name.chars().any(char::is_control) {
        return Err("invalid Notion attachment icon file name".to_string());
    }
    Ok(())
}

fn canonical_custom_emoji_value(value: String) -> Result<String, String> {
    let value = value.trim();
    let (space_id, custom_emoji_id) = custom_emoji_pointer(value)?;
    let space_id = uuid::Uuid::parse_str(space_id)
        .expect("a parsed Notion custom emoji space ID must remain valid");
    let custom_emoji_id = uuid::Uuid::parse_str(custom_emoji_id)
        .expect("a parsed Notion custom emoji ID must remain valid");
    Ok(format!(
        "notion://custom_emoji/{space_id}/{custom_emoji_id}"
    ))
}

fn custom_emoji_pointer(value: &str) -> Result<(&str, &str), String> {
    let path = value
        .strip_prefix("notion://custom_emoji/")
        .ok_or_else(|| "invalid Notion custom emoji pointer".to_string())?;
    let (space_id, custom_emoji_id) = path
        .split_once('/')
        .filter(|(_, custom_emoji_id)| !custom_emoji_id.contains('/'))
        .ok_or_else(|| "invalid Notion custom emoji pointer".to_string())?;
    uuid::Uuid::parse_str(space_id)
        .map_err(|error| format!("invalid Notion custom emoji space ID: {error}"))?;
    uuid::Uuid::parse_str(custom_emoji_id)
        .map_err(|error| format!("invalid Notion custom emoji ID: {error}"))?;
    Ok((space_id, custom_emoji_id))
}
