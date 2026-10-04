use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use gpui::{App, Image, ImageFormat, RenderImage};

pub(crate) const BOARD_WIDTH: f32 = 2304.0;
pub(crate) const BOARD_VIEWPORT_X: f32 = 104.0;
pub(crate) const BOARD_VIEWPORT_RIGHT_GUTTER: f32 = 64.0;
pub(crate) const BOARD_VIEWPORT_Y: f32 = 130.0;
pub(crate) const ACTIVE_VIEW_HEIGHT: f32 = 626.5;
pub(crate) const COLUMN_WIDTH: f32 = 276.0;
pub(crate) const COLUMN_STRIDE: f32 = 288.0;
pub(crate) const CARD_WIDTH: f32 = 260.0;
pub(crate) const VIEW_TAB_ICON_SIZE: f32 = 18.0;
pub(crate) const VIEW_TAB_ICON_GAP: f32 = 8.0;
pub(crate) const CARD_GAP: f32 = 8.0;
pub(crate) const CARD_X_OFFSET: f32 = 8.0;
pub(crate) const CARD_TOP: f32 = 205.0;
pub(crate) const CARD_PEEK_WIDTH: f32 = 856.0;
pub(crate) const DRAG_THRESHOLD: f32 = 4.0;
pub(crate) const DRAG_AUTO_SCROLL_EDGE: f32 = 44.0;
pub(crate) const DRAG_AUTO_SCROLL_STEP: f32 = 22.0;
pub(crate) const BOARD_SCROLL_LINE_MULTIPLIER: f32 = 42.0;
pub(crate) const BOARD_SCROLL_PIXEL_MULTIPLIER: f32 = 1.35;
pub(crate) const TABLE_HEADER_HEIGHT: f32 = 36.0;
pub(crate) const TABLE_ROW_HEIGHT: f32 = 36.0;
pub(crate) const TIMELINE_HEADER_HEIGHT: f32 = 66.0;
pub(crate) const TIMELINE_ROW_HEIGHT: f32 = 34.0;
pub(crate) const TIMELINE_EMPTY_BODY_HEIGHT: f32 = 94.0;
pub(crate) const TIMELINE_MONTH_LABEL_TOP_INSET: f32 = 7.0;
pub(crate) const TIMELINE_MONTH_LABEL_HEIGHT: f32 = 32.0;
pub(crate) const TIMELINE_VISIBLE_DAY_COUNT: usize = 43;
pub(crate) const TIMELINE_TODAY_COLUMN_INDEX: usize = 18;
pub(crate) const TIMELINE_MONTH_CENTER_LEAD_DAYS: i64 = 32;
pub(crate) const TIMELINE_MONTH_DAY_CELL_WIDTH: f32 = 40.0;
pub(crate) const TIMELINE_DAY_ROW_LEFT_INSET: f32 = -112.0;
pub(crate) const TIMELINE_DAY_ROW_RIGHT_INSET: f32 = -80.0;
pub(crate) const TIMELINE_DAY_LABEL_TOP_INSET: f32 = 1.0;
pub(crate) const TIMELINE_DAY_MARKER_SIZE: f32 = 20.0;
pub(crate) const TIMELINE_DAY_MARKER_TOP_INSET: f32 = 5.0;
pub(crate) const TIMELINE_CONTROLS_TOP_INSET: f32 = 12.0;
pub(crate) const TIMELINE_CONTROLS_RIGHT_INSET: f32 = 7.0;
pub(crate) const TIMELINE_TODAY_COLOR: u32 = 0xe56458;
pub(crate) const CALENDAR_HEADER_HEIGHT: f32 = 42.0;
pub(crate) const CALENDAR_HEADER_BOTTOM_MARGIN: f32 = 2.0;
pub(crate) const CALENDAR_WEEKDAY_HEADER_HEIGHT: f32 = 24.0;
pub(crate) const CALENDAR_WEEK_COUNT: usize = 6;
pub(crate) const CALENDAR_DAY_COUNT: usize = CALENDAR_WEEK_COUNT * 7;
pub(crate) const CALENDAR_DAY_CELL_HEIGHT: f32 = 124.0;
pub(crate) const CALENDAR_DAY_EVENT_TOP_INSET: f32 = 30.0;
pub(crate) const CALENDAR_DAY_BOTTOM_INSET: f32 = 4.0;
pub(crate) const CALENDAR_EVENT_ROW_PITCH: f32 = 34.0;
pub(crate) const CALENDAR_MIN_ACTIVE_VIEW_HEIGHT: f32 = CALENDAR_HEADER_HEIGHT
    + CALENDAR_HEADER_BOTTOM_MARGIN
    + CALENDAR_WEEKDAY_HEADER_HEIGHT
    + 1.0
    + CALENDAR_DAY_CELL_HEIGHT * CALENDAR_WEEK_COUNT as f32;
#[derive(Clone, Copy)]
pub struct ColorSpec {
    pub hex: u32,
    pub opacity: f32,
}

impl ColorSpec {
    pub const fn new(hex: u32, opacity: f32) -> Self {
        Self { hex, opacity }
    }
}

pub(crate) struct IconAsset {
    pub(crate) source: Arc<Image>,
    pub(crate) rendered: OnceLock<Arc<RenderImage>>,
}

impl IconAsset {
    pub(crate) fn new(source: Arc<Image>) -> Self {
        Self {
            source,
            rendered: OnceLock::new(),
        }
    }

    pub(crate) fn render(&self, cx: &mut App) -> Arc<RenderImage> {
        self.rendered
            .get_or_init(|| render_svg_image(self.source.clone(), cx))
            .clone()
    }
}

pub(crate) fn render_svg_image(source: Arc<Image>, cx: &mut App) -> Arc<RenderImage> {
    if source.format() == ImageFormat::Svg {
        return cached_svg_render_image(source.bytes(), || rasterize_image(&source, cx));
    }
    rasterize_image(&source, cx)
}

fn rasterize_image(source: &Image, cx: &mut App) -> Arc<RenderImage> {
    if source.format() == ImageFormat::Svg {
        return cx
            .svg_renderer()
            .render_single_frame(source.bytes(), 1.0)
            .expect("failed to render svg icon");
    }
    source
        .to_image_data(cx.svg_renderer())
        .expect("failed to render image")
}

/// Comfortably above the working set of one appearance mode's icon set plus the
/// per-render call sites, so eviction stays a safety valve rather than routine.
/// Eviction drops the raster without releasing its sprite-atlas tile, which is
/// why the bound matters.
const SVG_RENDER_CACHE_LIMIT: usize = 1_024;

struct SvgRenderCacheEntry {
    rendered: Arc<RenderImage>,
    touch: u64,
}

#[derive(Default)]
struct SvgRenderCache {
    entries: HashMap<Box<[u8]>, SvgRenderCacheEntry>,
    next_touch: u64,
}

impl SvgRenderCache {
    fn get(&mut self, bytes: &[u8]) -> Option<Arc<RenderImage>> {
        let touch = self.allocate_touch();
        let entry = self.entries.get_mut(bytes)?;
        entry.touch = touch;
        Some(entry.rendered.clone())
    }

    fn insert(&mut self, bytes: &[u8], rendered: Arc<RenderImage>) -> Arc<RenderImage> {
        if let Some(existing) = self.get(bytes) {
            return existing;
        }
        while self.entries.len() >= SVG_RENDER_CACHE_LIMIT {
            let Some(stale) = self.least_recent_key() else {
                break;
            };
            self.entries.remove(&stale);
        }
        let touch = self.allocate_touch();
        self.entries.insert(
            bytes.into(),
            SvgRenderCacheEntry {
                rendered: rendered.clone(),
                touch,
            },
        );
        rendered
    }

    fn least_recent_key(&self) -> Option<Box<[u8]>> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.touch)
            .map(|(key, _)| key.clone())
    }

    fn allocate_touch(&mut self) -> u64 {
        let touch = self.next_touch;
        self.next_touch = self
            .next_touch
            .checked_add(1)
            .expect("SVG raster cache recency must not overflow");
        touch
    }
}

fn svg_render_cache() -> &'static Mutex<SvgRenderCache> {
    static CACHE: OnceLock<Mutex<SvgRenderCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(SvgRenderCache::default()))
}

/// Rasterizing an SVG through resvg costs a large fraction of a frame, and both
/// the icon set and the per-render call sites rebuild the same markup. Reuse
/// rasters by SVG content so identical markup never re-rasterizes, including
/// across the icon-set rebuild that follows an appearance-mode change.
fn cached_svg_render_image(
    bytes: &[u8],
    render: impl FnOnce() -> Arc<RenderImage>,
) -> Arc<RenderImage> {
    if let Some(rendered) = svg_render_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(bytes)
    {
        return rendered;
    }
    // The renderer runs with the lock released so it cannot re-enter the cache.
    let rendered = render();
    svg_render_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(bytes, rendered)
}

pub(crate) fn notion_ai_button_image() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let bytes = decode_embedded_base64_image(
                include_str!("assets/notion_ai_button.png.b64"),
                "Notion AI button",
            );
            Arc::new(Image::from_bytes(ImageFormat::Png, bytes))
        })
        .clone()
}

pub(crate) fn notion_ai_face_image() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let bytes = decode_embedded_base64_image(
                include_str!("assets/notion_ai_face.png.b64"),
                "Notion AI face",
            );
            Arc::new(Image::from_bytes(ImageFormat::Png, bytes))
        })
        .clone()
}

fn decode_embedded_base64_image(encoded: &str, label: &str) -> Vec<u8> {
    let encoded = encoded.lines().collect::<String>();
    BASE64_STANDARD
        .decode(encoded)
        .unwrap_or_else(|error| panic!("embedded {label} asset must be valid base64: {error}"))
}

pub(crate) use gpui_components::alpha;

pub(crate) fn rgba(color: ColorSpec) -> gpui::Hsla {
    alpha(color.hex, color.opacity)
}

#[cfg(test)]
mod tests {
    use super::render_svg_image;
    use crate::ui::svg_from_body;
    use gpui::{AppContext as _, TestAppContext};
    use std::sync::Arc;

    #[gpui::test]
    fn render_svg_image_reuses_rasters_for_identical_markup(cx: &mut TestAppContext) {
        let body = r##"<path fill="#123456" d="M0 0h4v4z"/>"##;
        let first = svg_from_body("0 0 4 4", body);
        let second = svg_from_body("0 0 4 4", body);
        let other = svg_from_body("0 0 4 4", r##"<path fill="#654321" d="M0 0h4v4z"/>"##);
        assert!(!Arc::ptr_eq(&first, &second));

        let host = cx.new(|_| ());
        let (first, second, other) = host.update(cx, |_, cx| {
            (
                render_svg_image(first, cx),
                render_svg_image(second, cx),
                render_svg_image(other, cx),
            )
        });

        assert!(Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &other));
    }
}
