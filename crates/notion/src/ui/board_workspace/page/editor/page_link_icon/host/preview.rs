use gpui::{ClipboardEntry, Context, TaskExt, Window};

use super::super::preview::{PageLinkIconPreviewEffect, PageLinkIconUploadTarget};
use super::jobs::PageLinkIconPreviewJob;
use super::{
    PageLinkIconCompletion, PageLinkIconEffectHost, PageLinkIconEvent, PageLinkIconRootEffect,
};
use crate::ui::SurfaceState;

impl PageLinkIconEffectHost<'_> {
    pub(super) fn paste_upload(
        &mut self,
        target: PageLinkIconUploadTarget,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        let Some(clipboard) = cx.read_from_clipboard() else {
            return Vec::new();
        };
        let paths = clipboard.entries.iter().find_map(|entry| match entry {
            ClipboardEntry::ExternalPaths(paths) => Some(paths.paths()),
            _ => None,
        });
        if let Some(paths) = paths {
            if let Some(path) = paths.first() {
                let preview =
                    self.editor
                        .page_link_icons
                        .begin_local_file_preview(target, path.clone(), cx);
                return preview.map_or_else(Vec::new, |preview| self.start_preview(preview, cx));
            }
            return Vec::new();
        }
        let Some(value) = clipboard.text() else {
            return Vec::new();
        };
        match self
            .editor
            .page_link_icons
            .begin_external_url_preview(value, cx)
        {
            Ok(Some(preview)) => self.start_preview(preview, cx),
            Ok(None) => Vec::new(),
            Err(error) => {
                println!("notnotion: {error}");
                Vec::new()
            }
        }
    }

    pub(super) fn prompt_upload(
        &mut self,
        target: PageLinkIconUploadTarget,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) {
        let entity = cx.entity();
        let paths_receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Upload Image".into()),
        });
        window
            .spawn(cx, async move |cx| {
                let paths = match paths_receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    Ok(Ok(None)) | Err(_) => return Ok::<(), String>(()),
                    Ok(Err(error)) => {
                        println!("notnotion: failed to open the icon file picker: {error}");
                        return Ok(());
                    }
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(());
                };
                entity.update(cx, |surface, cx| {
                    surface.handle_page_link_icon_event(
                        PageLinkIconEvent::Completion(Box::new(
                            PageLinkIconCompletion::PromptedLocalFile { target, path },
                        )),
                        None,
                        cx,
                    );
                });
                Ok(())
            })
            .detach_and_log_err(cx);
    }

    pub(super) fn start_preview(
        &mut self,
        preview: PageLinkIconPreviewEffect,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        let job = match preview {
            PageLinkIconPreviewEffect::LocalFile { target, path } => {
                PageLinkIconPreviewJob::Local {
                    target,
                    path,
                    local_icon_files: self.resources.local_icon_files(),
                }
            }
            PageLinkIconPreviewEffect::ExternalUrl { target, value } => {
                PageLinkIconPreviewJob::External {
                    target,
                    value,
                    remote_images: self.resources.remote_images(),
                }
            }
        };
        job.spawn(cx);
        vec![PageLinkIconRootEffect::Notify]
    }
}
