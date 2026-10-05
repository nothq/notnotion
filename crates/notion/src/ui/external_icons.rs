use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{App, Image, ImageFormat, RenderImage};
use remote_image_model::RemoteImageApi;

use super::{
    surface::NotionSurfaceResources,
    view_actions::{ViewNotifier, ViewNotifiers},
};

const NOTION_EXTERNAL_ICON_CACHE_CAPACITY: usize = 128;
const NOTION_EXTERNAL_ICON_FAILURE_CAPACITY: usize = 128;
const NOTION_EXTERNAL_ICON_LOAD_CONCURRENCY: usize = 3;
const NOTION_EXTERNAL_ICON_PENDING_CAPACITY: usize = 64;
const NOTION_EXTERNAL_ICON_FAILURE_RETRY_DELAY: Duration = Duration::from_secs(30);

pub(crate) struct LoadedNotionExternalIcon {
    format: ImageFormat,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct NotionExternalIconCacheState {
    entries: HashMap<String, Arc<RenderImage>>,
    insertion_order: VecDeque<String>,
    pending: VecDeque<String>,
    active: HashSet<String>,
    waiters: HashMap<String, ViewNotifiers>,
    failed: HashMap<String, Instant>,
}

#[derive(Default)]
pub(crate) struct NotionExternalIconCache {
    state: RefCell<NotionExternalIconCacheState>,
}

impl NotionExternalIconCache {
    fn image(&self, value: &str) -> Option<Arc<RenderImage>> {
        self.state.borrow().entries.get(value).cloned()
    }

    pub(crate) fn insert(&self, value: String, rendered: Arc<RenderImage>, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        state.failed.remove(&value);
        state.pending.retain(|pending| pending != &value);
        state.active.remove(&value);
        let waiters = state.waiters.remove(&value).unwrap_or_default();
        if state.entries.insert(value.clone(), rendered).is_none() {
            state.insertion_order.push_back(value);
        }
        while state.entries.len() > NOTION_EXTERNAL_ICON_CACHE_CAPACITY {
            let oldest = state
                .insertion_order
                .pop_front()
                .expect("a non-empty external-icon cache must have an oldest entry");
            state.entries.remove(&oldest);
        }
        drop(state);
        waiters.notify(cx);
    }

    fn enqueue(&self, value: &str, notifier: &ViewNotifier) {
        let mut state = self.state.borrow_mut();
        if state.entries.contains_key(value) {
            return;
        }
        if state
            .failed
            .get(value)
            .is_some_and(|failed_at| failed_at.elapsed() < NOTION_EXTERNAL_ICON_FAILURE_RETRY_DELAY)
        {
            return;
        }
        state
            .waiters
            .entry(value.to_string())
            .or_default()
            .insert(notifier);
        if state.active.contains(value) {
            return;
        }
        state.failed.remove(value);
        if let Some(index) = state.pending.iter().position(|pending| pending == value) {
            let value = state
                .pending
                .remove(index)
                .expect("a located external-icon request must remain pending");
            state.pending.push_back(value);
            return;
        }
        if state.pending.len() == NOTION_EXTERNAL_ICON_PENDING_CAPACITY {
            let evicted = state
                .pending
                .pop_front()
                .expect("a full request queue has an oldest request");
            state.waiters.remove(&evicted);
        }
        state.pending.push_back(value.to_string());
    }

    fn next_request(&self) -> Option<String> {
        let mut state = self.state.borrow_mut();
        if state.active.len() >= NOTION_EXTERNAL_ICON_LOAD_CONCURRENCY {
            return None;
        }
        let value = state.pending.pop_back()?;
        state.active.insert(value.clone());
        Some(value)
    }

    fn finish(&self, value: String, rendered: Option<Arc<RenderImage>>, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        state.active.remove(&value);
        let Some(rendered) = rendered else {
            if state.failed.len() == NOTION_EXTERNAL_ICON_FAILURE_CAPACITY
                && !state.failed.contains_key(&value)
            {
                let oldest = state
                    .failed
                    .iter()
                    .min_by_key(|(_, failed_at)| **failed_at)
                    .map(|(value, _)| value.clone())
                    .expect("a full external-icon failure cache must have an oldest entry");
                state.failed.remove(&oldest);
            }
            let waiters = state.waiters.remove(&value).unwrap_or_default();
            state.failed.insert(value, Instant::now());
            drop(state);
            waiters.notify(cx);
            return;
        };
        drop(state);
        self.insert(value, rendered, cx);
    }
}

impl NotionSurfaceResources {
    pub(crate) fn external_icon_image(
        &self,
        value: &str,
        notifier: &ViewNotifier,
        cx: &mut App,
    ) -> Option<Arc<RenderImage>> {
        if let Some(rendered) = self.external_icon_cache().image(value) {
            return Some(rendered);
        }
        self.external_icon_cache().enqueue(value, notifier);
        self.start_next_external_icon_load(cx);
        None
    }

    fn start_next_external_icon_load(&self, cx: &mut App) {
        while let Some(value) = self.external_icon_cache().next_request() {
            let remote_images = self.remote_images();
            let task = cx.background_executor().spawn(async move {
                let loaded = load_notion_external_icon(remote_images.as_ref(), &value);
                (value, loaded)
            });
            let resources = self.clone();
            cx.spawn(async move |cx| {
                let (value, result) = task.await;
                cx.update(|cx| {
                    let rendered = render_loaded_external_icon(result, cx);
                    resources.external_icon_cache().finish(value, rendered, cx);
                    resources.start_next_external_icon_load(cx);
                });
            })
            .detach();
        }
    }
}

fn render_loaded_external_icon(
    result: Result<LoadedNotionExternalIcon, String>,
    cx: &mut App,
) -> Option<Arc<RenderImage>> {
    match result {
        Ok(loaded) => match render_notion_external_icon(loaded, cx) {
            Ok(rendered) => Some(rendered),
            Err(error) => {
                println!("notnotion: failed to render external icon: {error}");
                None
            }
        },
        Err(error) => {
            println!("notnotion: failed to load external icon: {error}");
            None
        }
    }
}

pub(crate) fn load_notion_external_icon(
    remote_images: &dyn RemoteImageApi,
    value: &str,
) -> Result<LoadedNotionExternalIcon, String> {
    let data = remote_images
        .load_remote_image(value)?
        .ok_or_else(|| "Notion external icon was not found".to_string())?;
    let format = ImageFormat::from_mime_type(&data.mimetype)
        .ok_or_else(|| "Notion external icon returned an unsupported image type".to_string())?;
    Ok(LoadedNotionExternalIcon {
        format,
        bytes: data.bytes,
    })
}

pub(crate) fn render_notion_external_icon(
    loaded: LoadedNotionExternalIcon,
    cx: &mut App,
) -> Result<Arc<RenderImage>, String> {
    let source = Image::from_bytes(loaded.format, loaded.bytes);
    source
        .to_image_data(cx.svg_renderer())
        .map_err(|error| format!("image decoding failed: {error}"))
}
