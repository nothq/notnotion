use std::sync::{Arc, OnceLock};

use crate::model::{PageShellIcon, PreparedPageIconUpload};

#[derive(Clone)]
pub struct UploadPageIconRequest {
    page_block_id: String,
    block_id: String,
    upload: PreparedPageIconUpload,
}

#[derive(Clone)]
pub(crate) enum CustomEmojiPageIconImage {
    ExternalUrl(String),
    LocalFile {
        name: String,
        content_type: String,
        bytes: Arc<[u8]>,
    },
}

#[derive(Clone)]
pub struct CreateCustomEmojiPageIconRequest {
    page_block_id: String,
    block_id: String,
    name: String,
    image: PreparedCustomEmojiPageIconImage,
}

#[derive(Clone)]
enum PreparedCustomEmojiPageIconImage {
    ExternalUrl(String),
    LocalFile(PreparedPageIconUpload),
}

#[derive(Clone, Debug)]
pub struct NotionCustomEmoji {
    pub id: String,
    pub name: String,
    pub pointer: String,
    pub render_url: String,
}

#[derive(Clone, Debug)]
pub struct NotionCustomEmojiLibrary {
    pub emojis: Vec<NotionCustomEmoji>,
    pub total_count: usize,
    pub creation_allowed: bool,
    pub limit: Option<usize>,
}

impl CreateCustomEmojiPageIconRequest {
    pub(crate) fn name_is_valid(name: &str) -> bool {
        static VALID_NAME: OnceLock<regex::Regex> = OnceLock::new();
        let valid_name = VALID_NAME.get_or_init(|| {
            regex::Regex::new(r"^[\p{Ll}\p{Lo}0-9_-]+$")
                .expect("the Notion custom emoji name expression must be valid")
        });
        !name.is_empty() && name.encode_utf16().count() < 50 && valid_name.is_match(name)
    }

    pub fn from_external_url(
        page_block_id: impl Into<String>,
        block_id: impl Into<String>,
        name: impl Into<String>,
        url: impl Into<String>,
    ) -> Result<Self, String> {
        let (page_block_id, block_id, name) = validated_custom_emoji_request_identity(
            page_block_id.into(),
            block_id.into(),
            name.into(),
        )?;
        let url = PageShellIcon::external(url)?.value;
        Ok(Self {
            page_block_id,
            block_id,
            name,
            image: PreparedCustomEmojiPageIconImage::ExternalUrl(url),
        })
    }

    pub(crate) fn from_prepared_local_file(
        page_block_id: impl Into<String>,
        block_id: impl Into<String>,
        name: impl Into<String>,
        upload: PreparedPageIconUpload,
    ) -> Result<Self, String> {
        let (page_block_id, block_id, name) = validated_custom_emoji_request_identity(
            page_block_id.into(),
            block_id.into(),
            name.into(),
        )?;
        Ok(Self {
            page_block_id,
            block_id,
            name,
            image: PreparedCustomEmojiPageIconImage::LocalFile(upload),
        })
    }

    pub(crate) fn into_parts(self) -> (String, String, String, CustomEmojiPageIconImage) {
        let source = match self.image {
            PreparedCustomEmojiPageIconImage::ExternalUrl(url) => {
                CustomEmojiPageIconImage::ExternalUrl(url)
            }
            PreparedCustomEmojiPageIconImage::LocalFile(upload) => {
                let (name, content_type, bytes) = upload.into_parts();
                CustomEmojiPageIconImage::LocalFile {
                    name,
                    content_type,
                    bytes,
                }
            }
        };
        (self.page_block_id, self.block_id, self.name, source)
    }
}

/// The page block ID, target block ID, and name of a custom emoji request.
type CustomEmojiRequestIdentity = (String, String, String);

fn validated_custom_emoji_request_identity(
    page_block_id: String,
    block_id: String,
    name: String,
) -> Result<CustomEmojiRequestIdentity, String> {
    if page_block_id.trim().is_empty() || block_id.trim().is_empty() {
        return Err("a custom emoji requires a loaded page and target block".to_string());
    }
    if !CreateCustomEmojiPageIconRequest::name_is_valid(&name) {
        return Err(
            "a custom emoji name must contain fewer than 50 lowercase letters, numbers, hyphens, or underscores"
                .to_string(),
        );
    }
    Ok((page_block_id, block_id, name))
}

/// The page block ID, target block ID, file name, MIME type, and bytes of an upload.
type PageIconUploadParts = (String, String, String, String, Arc<[u8]>);

impl UploadPageIconRequest {
    pub const MAX_FILE_SIZE: usize = PreparedPageIconUpload::MAX_FILE_SIZE;

    pub(crate) fn from_prepared(
        page_block_id: impl Into<String>,
        block_id: impl Into<String>,
        upload: PreparedPageIconUpload,
    ) -> Result<Self, String> {
        let page_block_id = page_block_id.into();
        let block_id = block_id.into();
        if page_block_id.trim().is_empty() || block_id.trim().is_empty() {
            return Err("a page-icon upload requires a loaded page and target block".to_string());
        }
        Ok(Self {
            page_block_id,
            block_id,
            upload,
        })
    }

    pub(crate) fn into_parts(self) -> PageIconUploadParts {
        let (name, content_type, bytes) = self.upload.into_parts();
        (self.page_block_id, self.block_id, name, content_type, bytes)
    }
}
