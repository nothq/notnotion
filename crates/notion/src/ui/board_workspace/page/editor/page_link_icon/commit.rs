use std::sync::Arc;

use gpui::RenderImage;

use super::{
    selection::PageLinkIconSetEffect, PageLinkIconController, PageLinkIconPickerTab,
    PageLinkIconTarget, PageLinkIconUploadPreview, PageLinkIconUploadSource,
};
use crate::model::{
    CreateCustomEmojiPageIconRequest, NotionWorkspaceApi, PageShellIcon, UploadPageIconRequest,
};

mod operation;

use operation::PageLinkIconCommitRequest;

pub(super) enum PreparedPageLinkIconSave {
    SetIcon(PageLinkIconSetEffect),
    Commit(PageLinkIconCommitStart),
}

pub(super) struct PageLinkIconCommitStart {
    pub(super) page_id: String,
    pub(super) block_id: String,
    pub(super) rendered: Arc<RenderImage>,
    request: PageLinkIconCommitRequest,
    pub(super) operation: &'static str,
}

pub(super) struct PageLinkIconCommitContext {
    pub(super) instance_id: u64,
    pub(super) page_id: String,
    pub(super) block_id: String,
    pub(super) generation: u64,
    pub(super) rendered: Arc<RenderImage>,
    pub(super) operation: &'static str,
}

pub(super) struct PendingPageLinkIconCommit {
    pub(super) context: PageLinkIconCommitContext,
    request: PageLinkIconCommitRequest,
}

impl PendingPageLinkIconCommit {
    pub(super) fn execute(
        self,
        workspace_api: &dyn NotionWorkspaceApi,
    ) -> PageLinkIconCommitCompletion {
        let result =
            self.request
                .execute(workspace_api, &self.context.page_id, &self.context.block_id);
        PageLinkIconCommitCompletion {
            context: self.context,
            result,
        }
    }
}

pub(super) struct PageLinkIconCommitCompletion {
    pub(super) context: PageLinkIconCommitContext,
    pub(super) result: crate::model::NotionWorkspaceResult<PageShellIcon>,
}

pub(super) struct PageLinkIconCommitResolution {
    pub(super) context: PageLinkIconCommitContext,
    pub(super) result: crate::model::NotionWorkspaceResult<PageShellIcon>,
    pub(super) commit_matches: bool,
    pub(super) picker_matches: bool,
}

impl PageLinkIconController {
    pub(super) fn prepare_save(
        &self,
        page_mutation_idle: bool,
    ) -> Option<PreparedPageLinkIconSave> {
        let picker = self.picker.as_ref()?;
        if picker.tab != PageLinkIconPickerTab::Upload
            || picker.upload_pending
            || self.commit_in_flight(&picker.page_id, &picker.block_id)
            || !page_mutation_idle
        {
            return None;
        }
        let preview = picker.upload_preview.clone()?;
        let page_id = picker.page_id.clone();
        let block_id = picker.block_id.clone();
        if picker.upload_add_to_library {
            let name = picker.upload_name.clone();
            if picker.custom_emoji_name_is_taken(&name) {
                println!("notnotion: cannot create custom emoji because its name is already taken");
                return None;
            }
            return prepare_custom_emoji_commit(page_id, block_id, name, preview)
                .map(PreparedPageLinkIconSave::Commit);
        }
        prepare_regular_icon_save(page_id, block_id, preview)
    }

    pub(super) fn start_commit(
        &mut self,
        start: PageLinkIconCommitStart,
    ) -> Option<PendingPageLinkIconCommit> {
        let picker = self.picker.as_mut()?;
        if picker.page_id != start.page_id
            || picker.block_id != start.block_id
            || picker.tab != PageLinkIconPickerTab::Upload
            || picker.upload_pending
        {
            return None;
        }
        picker.upload_generation = picker.upload_generation.wrapping_add(1);
        let pending = PendingPageLinkIconCommit {
            context: PageLinkIconCommitContext {
                instance_id: picker.instance_id,
                page_id: start.page_id,
                block_id: start.block_id,
                generation: picker.upload_generation,
                rendered: start.rendered,
                operation: start.operation,
            },
            request: start.request,
        };
        picker.upload_pending = true;
        picker.upload_committed = true;
        assert!(
            self.begin_commit(
                &pending.context.page_id,
                &pending.context.block_id,
                pending.context.instance_id,
                pending.context.generation,
            ),
            "a page-link icon target cannot have concurrent committed operations"
        );
        Some(pending)
    }

    pub(super) fn resolve_commit(
        &mut self,
        completion: PageLinkIconCommitCompletion,
    ) -> PageLinkIconCommitResolution {
        let PageLinkIconCommitCompletion { context, result } = completion;
        let commit_matches = self.finish_commit(
            &context.page_id,
            &context.block_id,
            context.instance_id,
            context.generation,
        );
        let picker_matches = commit_matches
            && self.picker.as_ref().is_some_and(|picker| {
                picker.matches_instance(context.instance_id, &context.page_id, &context.block_id)
                    && picker.tab == PageLinkIconPickerTab::Upload
                    && picker.upload_generation == context.generation
                    && picker.upload_committed
            });
        PageLinkIconCommitResolution {
            context,
            result,
            commit_matches,
            picker_matches,
        }
    }

    pub(super) fn finish_successful_commit(&mut self, picker_matches: bool) {
        if picker_matches {
            self.picker = None;
        }
    }

    pub(super) fn finish_failed_commit(&mut self, picker_matches: bool) {
        if !picker_matches {
            return;
        }
        let picker = self
            .picker
            .as_mut()
            .expect("a matching icon picker must remain present");
        picker.upload_pending = false;
        picker.upload_committed = false;
    }
}

fn prepare_regular_icon_save(
    page_id: String,
    block_id: String,
    preview: PageLinkIconUploadPreview,
) -> Option<PreparedPageLinkIconSave> {
    let PageLinkIconUploadPreview { source, rendered } = preview;
    match source {
        PageLinkIconUploadSource::ExternalUrl(value) => {
            let icon = match PageShellIcon::external(value) {
                Ok(icon) => icon,
                Err(error) => {
                    println!("notnotion: cannot save pasted icon URL: {error}");
                    return None;
                }
            };
            Some(PreparedPageLinkIconSave::SetIcon(PageLinkIconSetEffect {
                target: PageLinkIconTarget { page_id, block_id },
                icon: Some(icon),
                rendered: Some(rendered),
                keep_picker_open: false,
            }))
        }
        PageLinkIconUploadSource::LocalFile(upload) => {
            let request = match UploadPageIconRequest::from_prepared(
                page_id.clone(),
                block_id.clone(),
                upload,
            ) {
                Ok(request) => request,
                Err(error) => {
                    println!("notnotion: cannot upload the selected icon image: {error}");
                    return None;
                }
            };
            Some(PreparedPageLinkIconSave::Commit(PageLinkIconCommitStart {
                page_id,
                block_id,
                rendered,
                request: PageLinkIconCommitRequest::UploadPageIcon(request),
                operation: "page-icon upload",
            }))
        }
    }
}

fn prepare_custom_emoji_commit(
    page_id: String,
    block_id: String,
    name: String,
    preview: PageLinkIconUploadPreview,
) -> Option<PageLinkIconCommitStart> {
    let PageLinkIconUploadPreview { source, rendered } = preview;
    let request = match source {
        PageLinkIconUploadSource::ExternalUrl(url) => {
            CreateCustomEmojiPageIconRequest::from_external_url(
                page_id.clone(),
                block_id.clone(),
                name,
                url,
            )
        }
        PageLinkIconUploadSource::LocalFile(upload) => {
            CreateCustomEmojiPageIconRequest::from_prepared_local_file(
                page_id.clone(),
                block_id.clone(),
                name,
                upload,
            )
        }
    };
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            println!("notnotion: cannot create the custom emoji: {error}");
            return None;
        }
    };
    Some(PageLinkIconCommitStart {
        page_id,
        block_id,
        rendered,
        request: PageLinkIconCommitRequest::CreateCustomEmoji(request),
        operation: "custom-emoji creation",
    })
}
