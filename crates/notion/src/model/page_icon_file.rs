use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NotionPageIconMediaType {
    Jpeg,
    Png,
    Gif,
    Webp,
    Avif,
    Svg,
}

impl NotionPageIconMediaType {
    pub(crate) const fn as_mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
            Self::Svg => "image/svg+xml",
        }
    }
}

/// The file name, MIME type, and bytes of a prepared page-icon upload.
type PreparedPageIconUploadParts = (String, String, Arc<[u8]>);

#[derive(Clone)]
pub(crate) struct PreparedPageIconUpload {
    name: String,
    media_type: NotionPageIconMediaType,
    bytes: Arc<[u8]>,
}

impl PreparedPageIconUpload {
    pub(crate) const MAX_FILE_SIZE: usize = 5_000_000;

    pub(crate) fn new(
        name: String,
        media_type: NotionPageIconMediaType,
        bytes: Vec<u8>,
    ) -> Result<Self, String> {
        if name.is_empty()
            || name.len() > 1024
            || name.chars().any(char::is_control)
            || name.contains('/')
            || name.contains('\\')
        {
            return Err("the selected icon has an invalid file name".to_string());
        }
        if bytes.is_empty() || bytes.len() > Self::MAX_FILE_SIZE {
            return Err("Notion icon images must be smaller than 5 MB".to_string());
        }
        Ok(Self {
            name,
            media_type,
            bytes: bytes.into(),
        })
    }

    pub(crate) fn default_custom_emoji_name(&self) -> String {
        let stem = self
            .name
            .rfind('.')
            .map_or(self.name.as_str(), |extension_index| {
                &self.name[..extension_index]
            });
        let lowercase = stem.to_lowercase();
        static INVALID_TOKEN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        INVALID_TOKEN
            .get_or_init(|| {
                regex::Regex::new(r"[^\p{Ll}\p{Lo}0-9_-]")
                    .expect("the Notion custom emoji replacement expression must be valid")
            })
            .replace_all(&lowercase, "-")
            .into_owned()
    }

    pub(crate) fn into_parts(self) -> PreparedPageIconUploadParts {
        (
            self.name,
            self.media_type.as_mime_type().to_string(),
            self.bytes,
        )
    }
}

pub(crate) struct PreparedPageIconPreview {
    media_type: NotionPageIconMediaType,
    bytes: Vec<u8>,
}

impl PreparedPageIconPreview {
    pub(crate) fn new(media_type: NotionPageIconMediaType, bytes: Vec<u8>) -> Self {
        Self { media_type, bytes }
    }

    pub(crate) fn into_parts(self) -> (&'static str, Vec<u8>) {
        (self.media_type.as_mime_type(), self.bytes)
    }
}

pub(crate) struct PreparedPageIconFile {
    upload: PreparedPageIconUpload,
    preview: PreparedPageIconPreview,
}

impl PreparedPageIconFile {
    pub(crate) fn new(upload: PreparedPageIconUpload, preview: PreparedPageIconPreview) -> Self {
        Self { upload, preview }
    }

    pub(crate) fn default_custom_emoji_name(&self) -> String {
        self.upload.default_custom_emoji_name()
    }

    pub(crate) fn into_parts(self) -> (PreparedPageIconUpload, PreparedPageIconPreview) {
        (self.upload, self.preview)
    }
}

pub(crate) trait NotionLocalIconFileApi: Send + Sync + 'static {
    fn prepare_page_icon(&self, path: PathBuf) -> Result<PreparedPageIconFile, String>;
}

impl<F> NotionLocalIconFileApi for F
where
    F: Fn(PathBuf) -> Result<PreparedPageIconFile, String> + Send + Sync + 'static,
{
    fn prepare_page_icon(&self, path: PathBuf) -> Result<PreparedPageIconFile, String> {
        self(path)
    }
}
