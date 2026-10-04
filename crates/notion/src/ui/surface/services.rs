use std::{rc::Rc, sync::Arc};

use remote_image_model::RemoteImageApi;

use crate::model::NotionLocalIconFileApi;
use crate::ui::{NotionBlockImageCache, NotionExternalIconCache, NotionNamedIconCache};

#[derive(Clone)]
pub(crate) struct NotionSurfaceServices {
    remote_images: Arc<dyn RemoteImageApi>,
    local_icon_files: Arc<dyn NotionLocalIconFileApi>,
}

#[derive(Clone)]
pub(crate) struct NotionSurfaceResources {
    services: NotionSurfaceServices,
    external_icon_cache: Rc<NotionExternalIconCache>,
    block_image_cache: Rc<NotionBlockImageCache>,
    named_icon_cache: Rc<NotionNamedIconCache>,
}

impl NotionSurfaceResources {
    pub(crate) fn new(services: NotionSurfaceServices) -> Self {
        Self {
            services,
            external_icon_cache: Rc::new(NotionExternalIconCache::default()),
            block_image_cache: Rc::new(NotionBlockImageCache::default()),
            named_icon_cache: Rc::new(NotionNamedIconCache::default()),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn fixture_without_icon_io() -> Self {
        Self::new(NotionSurfaceServices::fixture_without_icon_io())
    }

    pub(crate) fn remote_images(&self) -> Arc<dyn RemoteImageApi> {
        self.services.remote_images.clone()
    }

    pub(crate) fn local_icon_files(&self) -> Arc<dyn NotionLocalIconFileApi> {
        self.services.local_icon_files.clone()
    }

    pub(crate) fn external_icon_cache(&self) -> &NotionExternalIconCache {
        self.external_icon_cache.as_ref()
    }

    pub(crate) fn named_icon_cache(&self) -> &NotionNamedIconCache {
        self.named_icon_cache.as_ref()
    }

    pub(crate) fn block_image_cache(&self) -> &NotionBlockImageCache {
        self.block_image_cache.as_ref()
    }

    pub(crate) fn block_image_cache_shared(&self) -> Rc<NotionBlockImageCache> {
        self.block_image_cache.clone()
    }
}

impl NotionSurfaceServices {
    pub(crate) fn new(
        remote_images: Arc<dyn RemoteImageApi>,
        local_icon_files: Arc<dyn NotionLocalIconFileApi>,
    ) -> Self {
        Self {
            remote_images,
            local_icon_files,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    fn fixture_without_icon_io() -> Self {
        let remote_images = Arc::new(
            |_url: &str| -> Result<Option<remote_image_model::RemoteImageData>, String> {
                Ok(None)
            },
        );
        let local_icon_files = Arc::new(
            |_path: std::path::PathBuf| -> Result<crate::model::PreparedPageIconFile, String> {
                Err("local Notion icon files are unavailable in this fixture".to_string())
            },
        );
        Self::new(remote_images, local_icon_files)
    }
}
