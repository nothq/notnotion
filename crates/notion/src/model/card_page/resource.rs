use serde::{Deserialize, Serialize};
use url::{Host, Url};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageBlockApiType(String);

impl CardPageBlockApiType {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        Self::try_from(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardPageBlockApiType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("a Notion block API type cannot be empty".to_string());
        }
        Ok(Self(value))
    }
}

impl From<CardPageBlockApiType> for String {
    fn from(value: CardPageBlockApiType) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageHttpsUrl(String);

impl CardPageHttpsUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        Self::try_from(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardPageHttpsUrl {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let url = Url::parse(&value).map_err(|_| "expected an absolute HTTPS URL".to_string())?;
        if url.scheme() != "https"
            || url.host_str().is_none_or(str::is_empty)
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("expected a credential-free absolute HTTPS URL".to_string());
        }
        Ok(Self(value))
    }
}

impl From<CardPageHttpsUrl> for String {
    fn from(value: CardPageHttpsUrl) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageImageDisplayHost(String);

impl CardPageImageDisplayHost {
    pub fn from_url(url: &CardPageHttpsUrl) -> Self {
        let url = Url::parse(url.as_str()).expect("a typed HTTPS URL must remain parseable");
        Self::try_from(
            url.host_str()
                .expect("a typed HTTPS URL must retain a host")
                .to_string(),
        )
        .expect("a host parsed from a typed HTTPS URL must remain valid")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardPageImageDisplayHost {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let host = Host::parse(&value).map_err(|_| "expected a valid HTTPS host".to_string())?;
        Ok(Self(host.to_string()))
    }
}

impl From<CardPageImageDisplayHost> for String {
    fn from(value: CardPageImageDisplayHost) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageImageWidthTier {
    Px2000,
}

impl CardPageImageWidthTier {
    pub const fn pixels(self) -> u16 {
        match self {
            Self::Px2000 => 2000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardPageImageFetchKey {
    url: CardPageHttpsUrl,
    width_tier: CardPageImageWidthTier,
}

impl CardPageImageFetchKey {
    pub fn new(url: CardPageHttpsUrl, width_tier: CardPageImageWidthTier) -> Self {
        Self { url, width_tier }
    }

    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    pub const fn width_tier(&self) -> CardPageImageWidthTier {
        self.width_tier
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageNotionAttachmentPointer(String);

impl CardPageNotionAttachmentPointer {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardPageNotionAttachmentPointer {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (file_id, file_name) = value
            .strip_prefix("attachment:")
            .and_then(|pointer| pointer.split_once(':'))
            .ok_or_else(|| "expected a nonempty Notion attachment pointer".to_string())?;
        uuid::Uuid::parse_str(file_id)
            .map_err(|error| format!("invalid Notion attachment ID: {error}"))?;
        if file_name.is_empty() || file_name.len() > 1024 || file_name.chars().any(char::is_control)
        {
            return Err("invalid Notion attachment file name".to_string());
        }
        Ok(Self(value))
    }
}

impl From<CardPageNotionAttachmentPointer> for String {
    fn from(value: CardPageNotionAttachmentPointer) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageImageSizeHint {
    width: u32,
    height: u32,
}

impl CardPageImageSizeHint {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("Notion image size hint dimensions must be positive".to_string());
        }
        Ok(Self { width, height })
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CardPageAttachmentDisplaySource {
    Https(CardPageHttpsUrl),
    NotionAttachment(CardPageNotionAttachmentPointer),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "image_source", rename_all = "snake_case")]
pub enum CardPageImageSource {
    Empty,
    ExternalHttps {
        source: CardPageHttpsUrl,
        display_host: CardPageImageDisplayHost,
        fetch_key: CardPageImageFetchKey,
    },
    NotionAttachment {
        pointer: CardPageNotionAttachmentPointer,
        display_source: CardPageAttachmentDisplaySource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_host: Option<CardPageImageDisplayHost>,
        fetch_key: CardPageImageFetchKey,
    },
}

impl CardPageImageSource {
    pub const fn fetch_key(&self) -> Option<&CardPageImageFetchKey> {
        match self {
            Self::Empty => None,
            Self::ExternalHttps { fetch_key, .. } | Self::NotionAttachment { fetch_key, .. } => {
                Some(fetch_key)
            }
        }
    }

    pub const fn display_host(&self) -> Option<&CardPageImageDisplayHost> {
        match self {
            Self::Empty => None,
            Self::ExternalHttps { display_host, .. } => Some(display_host),
            Self::NotionAttachment { display_host, .. } => display_host.as_ref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageImageBlock {
    source: CardPageImageSource,
    #[serde(
        default,
        alias = "dimensions",
        alias = "intrinsic_size",
        skip_serializing_if = "Option::is_none"
    )]
    size_hint: Option<CardPageImageSizeHint>,
}

impl CardPageImageBlock {
    pub const fn new(
        source: CardPageImageSource,
        size_hint: Option<CardPageImageSizeHint>,
    ) -> Self {
        Self { source, size_hint }
    }

    pub const fn source(&self) -> &CardPageImageSource {
        &self.source
    }

    pub const fn size_hint(&self) -> Option<CardPageImageSizeHint> {
        self.size_hint
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "snake_case")]
pub enum CardPageResourceBlock {
    Image { image: CardPageImageBlock },
}

impl CardPageResourceBlock {
    pub const fn image(&self) -> &CardPageImageBlock {
        match self {
            Self::Image { image: block_image } => block_image,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageUnsupportedLeafBlock {
    unsupported_leaf: CardPageBlockApiType,
}

impl CardPageUnsupportedLeafBlock {
    pub const fn new(api_type: CardPageBlockApiType) -> Self {
        Self {
            unsupported_leaf: api_type,
        }
    }

    pub const fn api_type(&self) -> &CardPageBlockApiType {
        &self.unsupported_leaf
    }
}
