use super::{
    Context, InlineDatabaseView, NotionCachedSurfaceRegion, NotionCachedSurfaceView, SurfaceState,
};
use gpui::{AppContext, Entity, WeakEntity};
use std::{cell::RefCell, collections::HashMap};

/// Inline database views keyed by their database block id.
pub(crate) type InlineDatabaseViews = HashMap<String, Entity<InlineDatabaseView>>;

#[derive(Default)]
pub(crate) struct SurfacePresentationState {
    pub(crate) cached_sidebar: Option<Entity<NotionCachedSurfaceView>>,
    pub(crate) cached_main: Option<Entity<NotionCachedSurfaceView>>,
    pub(crate) inline_database_views: RefCell<InlineDatabaseViews>,
    pub(crate) page_host: Option<WeakEntity<SurfaceState>>,
}

impl SurfacePresentationState {
    pub(crate) fn inherit_cached_regions(&mut self, previous: &Self) {
        self.cached_sidebar = previous.cached_sidebar.clone();
        self.cached_main = previous.cached_main.clone();
    }

    pub(crate) fn initialize_cached_regions(
        &mut self,
        surface: Entity<SurfaceState>,
        cx: &mut Context<SurfaceState>,
    ) {
        if self.cached_sidebar.is_some() || self.cached_main.is_some() {
            return;
        }
        self.cached_sidebar = Some(cx.new({
            let surface = surface.clone();
            move |cx| NotionCachedSurfaceView::new(surface, NotionCachedSurfaceRegion::Sidebar, cx)
        }));
        self.cached_main = Some(cx.new(move |cx| {
            NotionCachedSurfaceView::new(surface, NotionCachedSurfaceRegion::Main, cx)
        }));
    }
}
