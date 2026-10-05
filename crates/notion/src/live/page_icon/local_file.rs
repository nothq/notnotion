use std::{
    fs::File,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use crate::model::{
    NotionLocalIconFileApi, NotionPageIconMediaType, PreparedPageIconFile, PreparedPageIconPreview,
    PreparedPageIconUpload,
};

const NOTION_PAGE_ICON_MAX_PIXELS: u64 = 40_000_000;
const NOTION_PAGE_ICON_MAX_SIDE: u32 = 8192;

pub(crate) fn production_notion_local_icon_file_api() -> Arc<dyn NotionLocalIconFileApi> {
    Arc::new(ProductionNotionLocalIconFileApi)
}

struct ProductionNotionLocalIconFileApi;

impl NotionLocalIconFileApi for ProductionNotionLocalIconFileApi {
    fn prepare_page_icon(&self, path: PathBuf) -> Result<PreparedPageIconFile, String> {
        prepare_page_icon(path)
    }
}

fn prepare_page_icon(path: PathBuf) -> Result<PreparedPageIconFile, String> {
    let file =
        File::open(&path).map_err(|error| format!("failed to open the selected file: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect the selected file: {error}"))?;
    if !metadata.is_file() {
        return Err("the selected icon is not a regular file".to_string());
    }
    if metadata.len() == 0 || metadata.len() > PreparedPageIconUpload::MAX_FILE_SIZE as u64 {
        return Err("Notion icon images must be smaller than 5 MB".to_string());
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "the selected icon has an invalid file name".to_string())?
        .to_string();
    let media_type = media_type_for_path(&path)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(PreparedPageIconUpload::MAX_FILE_SIZE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read the selected file: {error}"))?;
    if bytes.is_empty() || bytes.len() > PreparedPageIconUpload::MAX_FILE_SIZE {
        return Err("Notion icon images must be smaller than 5 MB".to_string());
    }

    let preview = prepare_preview(media_type, &bytes)?;
    let upload = PreparedPageIconUpload::new(name, media_type, bytes)?;
    Ok(PreparedPageIconFile::new(upload, preview))
}

fn media_type_for_path(path: &std::path::Path) -> Result<NotionPageIconMediaType, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "jpg" | "jpeg" => Ok(NotionPageIconMediaType::Jpeg),
        "png" => Ok(NotionPageIconMediaType::Png),
        "gif" => Ok(NotionPageIconMediaType::Gif),
        "webp" => Ok(NotionPageIconMediaType::Webp),
        "avif" => Ok(NotionPageIconMediaType::Avif),
        "svg" => Ok(NotionPageIconMediaType::Svg),
        _ => Err("Notion icons must be JPEG, PNG, GIF, WebP, AVIF, or SVG images".to_string()),
    }
}

fn prepare_preview(
    media_type: NotionPageIconMediaType,
    bytes: &[u8],
) -> Result<PreparedPageIconPreview, String> {
    if media_type == NotionPageIconMediaType::Svg {
        remote_image::validate_safe_svg(bytes)
            .map_err(|error| format!("the selected SVG icon is unsafe: {error}"))?;
        return Ok(PreparedPageIconPreview::new(media_type, bytes.to_vec()));
    }

    let format = raster_format(media_type)
        .expect("every non-SVG Notion page-icon media type must have a raster format");
    let guessed = image::guess_format(bytes)
        .map_err(|_| "the selected file is not a recognized image".to_string())?;
    if guessed != format {
        return Err("the selected icon contents do not match its file type".to_string());
    }
    if format == image::ImageFormat::Avif {
        let decoded = decode_raster(bytes, format)?;
        let mut encoded = Cursor::new(Vec::new());
        decoded
            .write_to(&mut encoded, image::ImageFormat::Png)
            .map_err(|_| "failed to prepare the AVIF icon preview".to_string())?;
        return Ok(PreparedPageIconPreview::new(
            NotionPageIconMediaType::Png,
            encoded.into_inner(),
        ));
    }
    validate_raster_dimensions(bytes, format)?;
    Ok(PreparedPageIconPreview::new(media_type, bytes.to_vec()))
}

fn raster_format(media_type: NotionPageIconMediaType) -> Option<image::ImageFormat> {
    match media_type {
        NotionPageIconMediaType::Jpeg => Some(image::ImageFormat::Jpeg),
        NotionPageIconMediaType::Png => Some(image::ImageFormat::Png),
        NotionPageIconMediaType::Gif => Some(image::ImageFormat::Gif),
        NotionPageIconMediaType::Webp => Some(image::ImageFormat::WebP),
        NotionPageIconMediaType::Avif => Some(image::ImageFormat::Avif),
        NotionPageIconMediaType::Svg => None,
    }
}

fn image_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(NOTION_PAGE_ICON_MAX_SIDE);
    limits.max_image_height = Some(NOTION_PAGE_ICON_MAX_SIDE);
    limits.max_alloc = Some(NOTION_PAGE_ICON_MAX_PIXELS.saturating_mul(8));
    limits
}

fn validate_raster_dimensions(bytes: &[u8], format: image::ImageFormat) -> Result<(), String> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(image_limits());
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| "the selected icon image could not be decoded".to_string())?;
    validate_dimensions(width, height)
}

fn decode_raster(bytes: &[u8], format: image::ImageFormat) -> Result<image::DynamicImage, String> {
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(image_limits());
    let decoded = reader
        .decode()
        .map_err(|_| "the selected icon image could not be decoded".to_string())?;
    validate_dimensions(decoded.width(), decoded.height())?;
    Ok(decoded)
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > NOTION_PAGE_ICON_MAX_SIDE
        || height > NOTION_PAGE_ICON_MAX_SIDE
        || u64::from(width).saturating_mul(u64::from(height)) > NOTION_PAGE_ICON_MAX_PIXELS
    {
        return Err("the selected icon image dimensions are unsupported".to_string());
    }
    Ok(())
}
