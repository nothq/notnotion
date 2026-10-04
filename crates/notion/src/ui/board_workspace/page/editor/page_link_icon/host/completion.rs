use gpui::Context;

use super::super::{
    picker::PageLinkCustomEmojiLibraryResolution, preview::PageLinkIconExternalPreviewResolution,
};
use super::{
    PageLinkIconCompletion, PageLinkIconEffectHost, PageLinkIconRootEffect,
    PageLinkIconWorkspaceFailure,
};
use crate::ui::SurfaceState;

impl PageLinkIconEffectHost<'_> {
    pub(super) fn resolve_completion(
        &mut self,
        completion: PageLinkIconCompletion,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        match completion {
            PageLinkIconCompletion::CustomEmojiLibrary { request, result } => {
                self.resolve_library_completion(request, result)
            }
            PageLinkIconCompletion::PromptedLocalFile { target, path } => {
                let preview = self
                    .editor
                    .page_link_icons
                    .begin_local_file_preview(target, path, cx);
                preview.map_or_else(Vec::new, |preview| self.start_preview(preview, cx))
            }
            PageLinkIconCompletion::LocalPreview { target, result } => self
                .editor
                .page_link_icons
                .finish_local_file_preview(target, result, cx)
                .then_some(PageLinkIconRootEffect::Notify)
                .into_iter()
                .collect(),
            PageLinkIconCompletion::ExternalPreview {
                target,
                value,
                result,
            } => self.resolve_external_preview(target, value, result, cx),
            PageLinkIconCompletion::Commit(completion) => self.resolve_commit(completion, cx),
        }
    }

    fn resolve_library_completion(
        &mut self,
        request: super::super::picker::PageLinkCustomEmojiLibraryRequest,
        result: crate::model::NotionWorkspaceResult<crate::model::NotionCustomEmojiLibrary>,
    ) -> Vec<PageLinkIconRootEffect> {
        match self
            .editor
            .page_link_icons
            .finish_custom_emoji_library_load(&request, result)
        {
            PageLinkCustomEmojiLibraryResolution::Stale => Vec::new(),
            PageLinkCustomEmojiLibraryResolution::Applied => {
                vec![PageLinkIconRootEffect::Notify]
            }
            PageLinkCustomEmojiLibraryResolution::Failed(error) => {
                vec![PageLinkIconRootEffect::WorkspaceFailure(Box::new(
                    PageLinkIconWorkspaceFailure {
                        operation: "custom-emoji library loading failed",
                        error,
                        recovery: None,
                        notify_after_recovery: true,
                    },
                ))]
            }
        }
    }

    fn resolve_external_preview(
        &mut self,
        target: super::super::preview::PageLinkIconPreviewTarget,
        value: String,
        result: Result<crate::ui::LoadedNotionExternalIcon, String>,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        match self
            .editor
            .page_link_icons
            .finish_external_url_preview(target, value, result, cx)
        {
            PageLinkIconExternalPreviewResolution::Stale => Vec::new(),
            PageLinkIconExternalPreviewResolution::Finished { cache } => {
                if let Some((value, rendered)) = cache {
                    self.resources
                        .external_icon_cache()
                        .insert(value, rendered, cx);
                }
                vec![PageLinkIconRootEffect::Notify]
            }
        }
    }
}
