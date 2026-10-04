use std::{path::PathBuf, sync::Arc};

use gpui::{App, Image, ImageFormat, RenderImage};

use super::{
    PageLinkIconController, PageLinkIconPickerState, PageLinkIconPickerTab,
    PageLinkIconUploadPreview, PageLinkIconUploadSource,
};
use crate::{
    model::{PageShellIcon, PreparedPageIconFile},
    ui::{render_notion_external_icon, LoadedNotionExternalIcon},
};

mod controls;

#[derive(Clone)]
pub(super) struct PageLinkIconPreviewTarget {
    pub(super) instance_id: u64,
    pub(super) page_id: String,
    pub(super) block_id: String,
    pub(super) generation: u64,
}

pub(super) struct PageLinkIconUploadTarget {
    pub(super) instance_id: u64,
    pub(super) page_id: String,
    pub(super) block_id: String,
}

/// A pasted external icon URL and its rendered preview image.
type PageLinkIconExternalPreviewCacheEntry = (String, Arc<RenderImage>);

pub(super) enum PageLinkIconExternalPreviewResolution {
    Stale,
    Finished {
        cache: Option<PageLinkIconExternalPreviewCacheEntry>,
    },
}

impl PageLinkIconPreviewTarget {
    fn matches(&self, picker: &PageLinkIconPickerState) -> bool {
        picker.matches_instance(self.instance_id, &self.page_id, &self.block_id)
            && picker.tab == PageLinkIconPickerTab::Upload
            && picker.upload_generation == self.generation
    }
}

pub(super) enum PageLinkIconPreviewEffect {
    LocalFile {
        target: PageLinkIconPreviewTarget,
        path: PathBuf,
    },
    ExternalUrl {
        target: PageLinkIconPreviewTarget,
        value: String,
    },
}

impl PageLinkIconController {
    pub(super) fn upload_prompt_target(&self) -> Option<PageLinkIconUploadTarget> {
        let picker = self.picker.as_ref()?;
        if picker.tab != PageLinkIconPickerTab::Upload || picker.upload_pending {
            return None;
        }
        Some(PageLinkIconUploadTarget {
            instance_id: picker.instance_id,
            page_id: picker.page_id.clone(),
            block_id: picker.block_id.clone(),
        })
    }

    pub(super) fn begin_local_file_preview(
        &mut self,
        target: PageLinkIconUploadTarget,
        path: PathBuf,
        cx: &mut App,
    ) -> Option<PageLinkIconPreviewEffect> {
        let PageLinkIconUploadTarget {
            instance_id,
            page_id,
            block_id,
        } = target;
        let picker = self.picker.as_mut()?;
        if !picker.matches_instance(instance_id, &page_id, &block_id)
            || picker.tab != PageLinkIconPickerTab::Upload
            || picker.upload_pending
        {
            return None;
        }
        let generation = begin_page_link_icon_preview(picker, cx);
        Some(PageLinkIconPreviewEffect::LocalFile {
            target: PageLinkIconPreviewTarget {
                instance_id,
                page_id,
                block_id,
                generation,
            },
            path,
        })
    }

    pub(super) fn begin_external_url_preview(
        &mut self,
        value: String,
        cx: &mut App,
    ) -> Result<Option<PageLinkIconPreviewEffect>, String> {
        let icon = PageShellIcon::external(value)
            .map_err(|error| format!("ignored invalid pasted icon URL: {error}"))?;
        let value = icon.value;
        let Some(picker) = self.picker.as_mut() else {
            return Ok(None);
        };
        if picker.tab != PageLinkIconPickerTab::Upload {
            return Ok(None);
        }
        let target = PageLinkIconPreviewTarget {
            instance_id: picker.instance_id,
            page_id: picker.page_id.clone(),
            block_id: picker.block_id.clone(),
            generation: begin_page_link_icon_preview(picker, cx),
        };
        Ok(Some(PageLinkIconPreviewEffect::ExternalUrl {
            target,
            value,
        }))
    }

    pub(super) fn finish_external_url_preview(
        &mut self,
        target: PageLinkIconPreviewTarget,
        value: String,
        result: Result<LoadedNotionExternalIcon, String>,
        cx: &mut App,
    ) -> PageLinkIconExternalPreviewResolution {
        let rendered = match result.and_then(|loaded| render_notion_external_icon(loaded, cx)) {
            Ok(rendered) => Some(rendered),
            Err(error) => {
                println!("notnotion: pasted icon preview failed: {error}");
                None
            }
        };
        let Some(picker) = self.picker.as_mut().filter(|picker| target.matches(picker)) else {
            return PageLinkIconExternalPreviewResolution::Stale;
        };
        picker.upload_pending = false;
        let cache = if let Some(rendered) = rendered {
            picker.upload_preview = Some(PageLinkIconUploadPreview {
                source: PageLinkIconUploadSource::ExternalUrl(value.clone()),
                rendered: Arc::clone(&rendered),
            });
            Some((value, rendered))
        } else {
            None
        };
        PageLinkIconExternalPreviewResolution::Finished { cache }
    }

    pub(super) fn finish_local_file_preview(
        &mut self,
        target: PageLinkIconPreviewTarget,
        result: Result<PreparedPageIconFile, String>,
        cx: &mut App,
    ) -> bool {
        let loaded = match result {
            Ok(loaded) => loaded,
            Err(error) => {
                println!("notnotion: selected icon image was rejected: {error}");
                return self.finish_failed_preview(&target);
            }
        };
        let (upload_name, preview) = match render_prepared_page_link_icon(loaded, cx) {
            Ok(preview) => preview,
            Err(error) => {
                println!("notnotion: selected icon preview failed: {error}");
                return self.finish_failed_preview(&target);
            }
        };
        let Some(picker) = self.picker.as_mut().filter(|picker| target.matches(picker)) else {
            return false;
        };
        picker.upload_pending = false;
        picker.upload_name.clone_from(&upload_name);
        picker.upload_name_input.update(cx, |input, cx| {
            input.set_text(upload_name, cx);
        });
        picker.upload_preview = Some(preview);
        true
    }

    fn finish_failed_preview(&mut self, target: &PageLinkIconPreviewTarget) -> bool {
        let Some(picker) = self.picker.as_mut().filter(|picker| target.matches(picker)) else {
            return false;
        };
        picker.upload_pending = false;
        true
    }
}

fn begin_page_link_icon_preview(picker: &mut PageLinkIconPickerState, cx: &mut App) -> u64 {
    picker.upload_generation = picker.upload_generation.wrapping_add(1);
    picker.upload_preview = None;
    picker.upload_pending = true;
    picker.upload_committed = false;
    picker.upload_add_to_library = false;
    picker.upload_name.clear();
    picker.upload_name_input.update(cx, |input, cx| {
        input.set_text(String::new(), cx);
    });
    picker.upload_generation
}

fn render_prepared_page_link_icon(
    loaded: PreparedPageIconFile,
    cx: &mut App,
) -> Result<(String, PageLinkIconUploadPreview), String> {
    let upload_name = loaded.default_custom_emoji_name();
    let (upload, preview) = loaded.into_parts();
    let (preview_mime_type, preview_bytes) = preview.into_parts();
    let preview_format = ImageFormat::from_mime_type(preview_mime_type)
        .expect("every prepared Notion icon preview must be renderable by GPUI");
    let preview_image = Image::from_bytes(preview_format, preview_bytes);
    let rendered = preview_image
        .to_image_data(cx.svg_renderer())
        .map_err(|error| error.to_string())?;
    Ok((
        upload_name,
        PageLinkIconUploadPreview {
            source: PageLinkIconUploadSource::LocalFile(upload),
            rendered,
        },
    ))
}
