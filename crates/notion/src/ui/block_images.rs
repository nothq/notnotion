use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{App, Image, ImageFormat, ListState, RenderImage};
use remote_image_model::RemoteImageApi;

use super::{
    surface::{NotionSurfaceResources, PageDocumentFlowRuntime, PageFlowObservationToken},
    view_actions::ViewNotifier,
    CardPageImageFetchKey, LoadedCardPageData,
};

const IMAGE_ENTRY_CAPACITY: usize = 32;
const IMAGE_DECODED_BYTE_CAPACITY: u64 = 96 * 1024 * 1024;
const IMAGE_FAILURE_CAPACITY: usize = 128;
const IMAGE_LOAD_CONCURRENCY: usize = 3;
const IMAGE_PENDING_CAPACITY: usize = 32;
const IMAGE_WAITER_CAPACITY_PER_KEY: usize = 8;
const IMAGE_FAILURE_RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct LoadedNotionBlockImage {
    format: ImageFormat,
    bytes: Vec<u8>,
}

#[derive(Clone)]
pub(crate) struct CachedNotionBlockImage {
    pub(crate) rendered: Arc<RenderImage>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    decoded_bytes: u64,
}

#[derive(Clone)]
pub(crate) enum NotionBlockImageState {
    Ready(CachedNotionBlockImage),
    Pending,
    Failed,
}

#[derive(Clone)]
struct NotionBlockImageWaiter {
    notifier: ViewNotifier,
    flow: PageDocumentFlowRuntime,
    list_state: ListState,
    data: std::sync::Weak<LoadedCardPageData>,
    observation: PageFlowObservationToken,
    block_id: String,
    key: CardPageImageFetchKey,
}

/// The page block that shows an image and is re-measured once it loads.
pub(crate) struct NotionBlockImageRequester<'a> {
    pub(crate) data: &'a Arc<LoadedCardPageData>,
    pub(crate) observation: &'a PageFlowObservationToken,
    pub(crate) block_id: &'a str,
}

#[derive(Clone)]
pub(crate) struct NotionBlockImageView {
    resources: NotionSurfaceResources,
    notifier: ViewNotifier,
    flow: PageDocumentFlowRuntime,
    list_state: ListState,
}

impl NotionBlockImageView {
    pub(crate) fn new(
        resources: NotionSurfaceResources,
        notifier: ViewNotifier,
        flow: PageDocumentFlowRuntime,
        list_state: ListState,
    ) -> Self {
        Self {
            resources,
            notifier,
            flow,
            list_state,
        }
    }

    pub(crate) fn image(
        &self,
        key: &CardPageImageFetchKey,
        requester: NotionBlockImageRequester<'_>,
        cx: &mut App,
    ) -> NotionBlockImageState {
        let NotionBlockImageRequester {
            data,
            observation,
            block_id,
        } = requester;
        let waiter = NotionBlockImageWaiter {
            notifier: self.notifier.clone(),
            flow: self.flow.clone(),
            list_state: self.list_state.clone(),
            data: std::sync::Arc::downgrade(data),
            observation: observation.clone(),
            block_id: block_id.to_string(),
            key: key.clone(),
        };
        let state = self.resources.block_image_cache().request(key, waiter);
        self.resources.start_next_notion_block_image_load(cx);
        state
    }
}

#[derive(Clone)]
struct NotionBlockImageFailure {
    failed_at: Instant,
}

#[derive(Default)]
struct NotionBlockImageCacheState {
    entries: HashMap<CardPageImageFetchKey, CachedNotionBlockImage>,
    lru: VecDeque<CardPageImageFetchKey>,
    decoded_bytes: u64,
    pending: VecDeque<CardPageImageFetchKey>,
    active: HashSet<CardPageImageFetchKey>,
    failed: HashMap<CardPageImageFetchKey, NotionBlockImageFailure>,
    waiters: HashMap<CardPageImageFetchKey, Vec<NotionBlockImageWaiter>>,
}

#[derive(Default)]
pub(crate) struct NotionBlockImageCache {
    state: RefCell<NotionBlockImageCacheState>,
}

impl NotionBlockImageCache {
    pub(crate) fn cached(&self, key: &CardPageImageFetchKey) -> Option<CachedNotionBlockImage> {
        let mut state = self.state.borrow_mut();
        let cached_image = state.entries.get(key).cloned()?;
        touch_lru(&mut state.lru, key);
        Some(cached_image)
    }

    fn request(
        &self,
        key: &CardPageImageFetchKey,
        waiter: NotionBlockImageWaiter,
    ) -> NotionBlockImageState {
        let mut state = self.state.borrow_mut();
        if let Some(cached_image) = state.entries.get(key).cloned() {
            touch_lru(&mut state.lru, key);
            return NotionBlockImageState::Ready(cached_image);
        }
        if let Some(failure) = state.failed.get(key) {
            if failure.failed_at.elapsed() < IMAGE_FAILURE_RETRY_DELAY {
                return NotionBlockImageState::Failed;
            }
        }
        state.failed.remove(key);
        register_waiter(&mut state.waiters, key, waiter);
        if state.active.contains(key) {
            return NotionBlockImageState::Pending;
        }
        if let Some(index) = state.pending.iter().position(|pending| pending == key) {
            let key = state
                .pending
                .remove(index)
                .expect("a located block-image request must remain pending");
            state.pending.push_back(key);
            return NotionBlockImageState::Pending;
        }
        if state.pending.len() == IMAGE_PENDING_CAPACITY {
            if let Some(dropped) = state.pending.pop_front() {
                state.waiters.remove(&dropped);
            }
        }
        state.pending.push_back(key.clone());
        NotionBlockImageState::Pending
    }

    fn next_request(&self) -> Option<CardPageImageFetchKey> {
        let mut state = self.state.borrow_mut();
        if state.active.len() >= IMAGE_LOAD_CONCURRENCY {
            return None;
        }
        let key = state.pending.pop_back()?;
        state.active.insert(key.clone());
        Some(key)
    }

    fn finish(
        &self,
        key: CardPageImageFetchKey,
        result: Result<CachedNotionBlockImage, ()>,
    ) -> Vec<NotionBlockImageWaiter> {
        let mut state = self.state.borrow_mut();
        state.active.remove(&key);
        state.pending.retain(|pending| pending != &key);
        let waiters = state.waiters.remove(&key).unwrap_or_default();
        match result {
            Ok(cached_image) => insert_image(&mut state, key, cached_image),
            Err(()) => insert_failure(&mut state, key),
        }
        waiters
    }
}

fn register_waiter(
    waiters: &mut HashMap<CardPageImageFetchKey, Vec<NotionBlockImageWaiter>>,
    key: &CardPageImageFetchKey,
    waiter: NotionBlockImageWaiter,
) {
    let entry = waiters.entry(key.clone()).or_default();
    if entry.iter().any(|existing| {
        std::sync::Weak::ptr_eq(&existing.data, &waiter.data)
            && existing.block_id == waiter.block_id
            && existing.key == waiter.key
    }) {
        return;
    }
    if entry.len() == IMAGE_WAITER_CAPACITY_PER_KEY {
        entry.remove(0);
    }
    entry.push(waiter);
}

fn touch_lru(lru: &mut VecDeque<CardPageImageFetchKey>, key: &CardPageImageFetchKey) {
    if let Some(index) = lru.iter().position(|candidate| candidate == key) {
        lru.remove(index);
    }
    lru.push_back(key.clone());
}

fn insert_image(
    state: &mut NotionBlockImageCacheState,
    key: CardPageImageFetchKey,
    cached_image: CachedNotionBlockImage,
) {
    state.failed.remove(&key);
    if let Some(previous) = state.entries.insert(key.clone(), cached_image.clone()) {
        state.decoded_bytes = state.decoded_bytes.saturating_sub(previous.decoded_bytes);
    }
    state.decoded_bytes = state
        .decoded_bytes
        .saturating_add(cached_image.decoded_bytes);
    touch_lru(&mut state.lru, &key);
    while state.entries.len() > IMAGE_ENTRY_CAPACITY
        || state.decoded_bytes > IMAGE_DECODED_BYTE_CAPACITY
    {
        let Some(oldest) = state.lru.pop_front() else {
            break;
        };
        if let Some(evicted) = state.entries.remove(&oldest) {
            state.decoded_bytes = state.decoded_bytes.saturating_sub(evicted.decoded_bytes);
        }
    }
}

fn insert_failure(state: &mut NotionBlockImageCacheState, key: CardPageImageFetchKey) {
    if state.failed.len() == IMAGE_FAILURE_CAPACITY && !state.failed.contains_key(&key) {
        if let Some(oldest) = state
            .failed
            .iter()
            .min_by_key(|(_, failure)| failure.failed_at)
            .map(|(key, _)| key.clone())
        {
            state.failed.remove(&oldest);
        }
    }
    state.failed.insert(
        key,
        NotionBlockImageFailure {
            failed_at: Instant::now(),
        },
    );
}

impl NotionSurfaceResources {
    fn start_next_notion_block_image_load(&self, cx: &mut App) {
        while let Some(key) = self.block_image_cache().next_request() {
            let remote_images = self.remote_images();
            let task = cx.background_executor().spawn(async move {
                let loaded = load_notion_block_image(remote_images.as_ref(), &key);
                (key, loaded)
            });
            let resources = self.clone();
            cx.spawn(async move |cx| {
                let (key, result) = task.await;
                cx.update(|cx| {
                    let result = result.and_then(|loaded| decode_notion_block_image(loaded, cx));
                    if let Err(diagnostic) = &result {
                        println!("notnotion: failed to load block image: {diagnostic}");
                    }
                    let waiters = resources
                        .block_image_cache()
                        .finish(key, result.map_err(|_| ()));
                    notify_image_waiters(waiters, cx);
                    resources.start_next_notion_block_image_load(cx);
                });
            })
            .detach();
        }
    }
}

fn notify_image_waiters(waiters: Vec<NotionBlockImageWaiter>, cx: &mut App) {
    for waiter in waiters {
        let Some(data) = waiter.data.upgrade() else {
            continue;
        };
        if !waiter
            .flow
            .state()
            .observations
            .borrow()
            .is_current(&waiter.observation)
            || data
                .page_block(&waiter.block_id)
                .and_then(|block| block.resource_content())
                .and_then(|resource| resource.image().source().fetch_key())
                != Some(&waiter.key)
        {
            continue;
        }
        if data.has_column_structure() {
            if let Some(unit) = data.document_unit_key_for_block(&waiter.block_id) {
                waiter
                    .flow
                    .state()
                    .virtualizer
                    .borrow_mut()
                    .invalidate_document_unit_layout(&data.page.block_id, &unit);
            }
        } else {
            waiter.list_state.remeasure();
        }
        waiter.notifier.notify(cx);
    }
}

fn load_notion_block_image(
    remote_images: &dyn RemoteImageApi,
    key: &CardPageImageFetchKey,
) -> Result<LoadedNotionBlockImage, String> {
    let data = remote_images
        .load_remote_image(key.url())?
        .ok_or_else(|| "Notion block image was not found".to_string())?;
    let format = ImageFormat::from_mime_type(&data.mimetype)
        .ok_or_else(|| "Notion block image returned an unsupported image type".to_string())?;
    Ok(LoadedNotionBlockImage {
        format,
        bytes: data.bytes,
    })
}

fn decode_notion_block_image(
    loaded: LoadedNotionBlockImage,
    cx: &mut App,
) -> Result<CachedNotionBlockImage, String> {
    let source = Image::from_bytes(loaded.format, loaded.bytes);
    let rendered = source
        .to_image_data(cx.svg_renderer())
        .map_err(|error| format!("Notion block image decoding failed: {error}"))?;
    let (width, height, decoded_bytes) = decoded_image_weight(rendered.as_ref())?;
    if decoded_bytes > IMAGE_DECODED_BYTE_CAPACITY {
        return Err("Notion block image exceeds the decoded image limit".to_string());
    }
    Ok(CachedNotionBlockImage {
        rendered,
        width,
        height,
        decoded_bytes,
    })
}

/// The first frame's width and height, and the decoded bytes of every frame.
type DecodedImageWeight = (u32, u32, u64);

fn decoded_image_weight(rendered_image: &RenderImage) -> Result<DecodedImageWeight, String> {
    if rendered_image.frame_count() == 0 {
        return Err("Notion block image decoded without frames".to_string());
    }
    let first = rendered_image.size(0);
    let width = u32::from(first.width);
    let height = u32::from(first.height);
    if width == 0 || height == 0 {
        return Err("Notion block image decoded with empty dimensions".to_string());
    }
    let decoded_bytes = (0..rendered_image.frame_count()).fold(0u64, |weight, index| {
        let size = rendered_image.size(index);
        weight.saturating_add(
            u64::from(u32::from(size.width))
                .saturating_mul(u64::from(u32::from(size.height)))
                .saturating_mul(4),
        )
    });
    Ok((width, height, decoded_bytes))
}
