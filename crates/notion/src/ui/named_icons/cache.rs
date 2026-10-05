use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{App, RenderImage};

use crate::ui::{
    load_notion_external_icon, render_notion_external_icon,
    surface::NotionSurfaceResources,
    view_actions::{ViewNotifier, ViewNotifiers},
    LoadedNotionExternalIcon,
};

use super::asset::NotionNamedIconAsset;

const NOTION_NAMED_ICON_CACHE_CAPACITY: usize = 1024;
const NOTION_NAMED_ICON_FAILURE_CAPACITY: usize = 1024;
const NOTION_NAMED_ICON_LOAD_CONCURRENCY: usize = 8;
const NOTION_NAMED_ICON_PENDING_CAPACITY: usize = 128;
const NOTION_NAMED_ICON_FAILURE_RETRY_DELAY: Duration = Duration::from_secs(15);

#[derive(Default)]
struct NotionNamedIconCacheState {
    entries: HashMap<NotionNamedIconAsset, Arc<RenderImage>>,
    insertion_order: VecDeque<NotionNamedIconAsset>,
    pending: VecDeque<NotionNamedIconAsset>,
    active: HashSet<NotionNamedIconAsset>,
    waiters: HashMap<NotionNamedIconAsset, ViewNotifiers>,
    failed: HashMap<NotionNamedIconAsset, Instant>,
}

#[derive(Default)]
pub(crate) struct NotionNamedIconCache {
    state: RefCell<NotionNamedIconCacheState>,
}

impl NotionNamedIconCache {
    fn image(&self, asset: NotionNamedIconAsset) -> Option<Arc<RenderImage>> {
        self.state.borrow().entries.get(&asset).cloned()
    }

    fn enqueue(&self, asset: NotionNamedIconAsset, notifier: &ViewNotifier) {
        let mut state = self.state.borrow_mut();
        if state.entries.contains_key(&asset) {
            return;
        }
        if state
            .failed
            .get(&asset)
            .is_some_and(|failed_at| failed_at.elapsed() < NOTION_NAMED_ICON_FAILURE_RETRY_DELAY)
        {
            return;
        }
        state.waiters.entry(asset).or_default().insert(notifier);
        if state.active.contains(&asset) {
            return;
        }
        state.failed.remove(&asset);
        if let Some(index) = state.pending.iter().position(|pending| pending == &asset) {
            state.pending.remove(index);
            state.pending.push_back(asset);
            return;
        }
        if state.pending.len() == NOTION_NAMED_ICON_PENDING_CAPACITY {
            let evicted = state
                .pending
                .pop_front()
                .expect("a full request queue has an oldest request");
            state.waiters.remove(&evicted);
        }
        state.pending.push_back(asset);
    }

    fn next_request(&self) -> Option<NotionNamedIconAsset> {
        let mut state = self.state.borrow_mut();
        if state.active.len() >= NOTION_NAMED_ICON_LOAD_CONCURRENCY {
            return None;
        }
        // The list is virtualized, so assets requested most recently are the
        // ones most likely to remain visible after a fast scroll or color
        // change. Serve those first and bound stale offscreen work.
        let asset = state.pending.pop_back()?;
        state.active.insert(asset);
        Some(asset)
    }

    fn finish(
        &self,
        asset: NotionNamedIconAsset,
        rendered: Option<Arc<RenderImage>>,
        cx: &mut App,
    ) {
        let mut state = self.state.borrow_mut();
        state.active.remove(&asset);
        let waiters = state.waiters.remove(&asset).unwrap_or_default();
        let Some(rendered) = rendered else {
            if state.failed.len() == NOTION_NAMED_ICON_FAILURE_CAPACITY
                && !state.failed.contains_key(&asset)
            {
                let oldest = state
                    .failed
                    .iter()
                    .min_by_key(|(_, failed_at)| **failed_at)
                    .map(|(asset, _)| *asset)
                    .expect("a full named-icon failure cache must have an oldest entry");
                state.failed.remove(&oldest);
            }
            state.failed.insert(asset, Instant::now());
            drop(state);
            waiters.notify(cx);
            return;
        };
        state.failed.remove(&asset);
        if state.entries.insert(asset, rendered).is_none() {
            state.insertion_order.push_back(asset);
        }
        while state.entries.len() > NOTION_NAMED_ICON_CACHE_CAPACITY {
            let oldest = state
                .insertion_order
                .pop_front()
                .expect("a non-empty named-icon cache must have an oldest entry");
            state.entries.remove(&oldest);
        }
        drop(state);
        waiters.notify(cx);
    }
}

impl NotionSurfaceResources {
    pub(crate) fn named_icon_image(
        &self,
        asset: NotionNamedIconAsset,
        notifier: &ViewNotifier,
        cx: &mut App,
    ) -> Option<Arc<RenderImage>> {
        if let Some(rendered) = self.named_icon_cache().image(asset) {
            return Some(rendered);
        }
        self.named_icon_cache().enqueue(asset, notifier);
        self.start_next_named_icon_load(cx);
        None
    }

    fn start_next_named_icon_load(&self, cx: &mut App) {
        while let Some(asset) = self.named_icon_cache().next_request() {
            let remote_images = self.remote_images();
            let task = cx.background_executor().spawn(async move {
                load_notion_external_icon(remote_images.as_ref(), &asset.url())
            });
            let resources = self.clone();
            cx.spawn(async move |cx| {
                let result = task.await;
                cx.update(|cx| {
                    let rendered = render_loaded_named_icon(asset, result, cx);
                    resources.named_icon_cache().finish(asset, rendered, cx);
                    resources.start_next_named_icon_load(cx);
                });
            })
            .detach();
        }
    }
}

fn render_loaded_named_icon(
    asset: NotionNamedIconAsset,
    result: Result<LoadedNotionExternalIcon, String>,
    cx: &mut App,
) -> Option<Arc<RenderImage>> {
    match result {
        Ok(loaded) => match render_notion_external_icon(loaded, cx) {
            Ok(rendered) => Some(rendered),
            Err(error) => {
                println!(
                    "notnotion: failed to render named icon {}: {error}",
                    asset.slug()
                );
                None
            }
        },
        Err(error) => {
            println!(
                "notnotion: failed to load named icon {}: {error}",
                asset.slug()
            );
            None
        }
    }
}
