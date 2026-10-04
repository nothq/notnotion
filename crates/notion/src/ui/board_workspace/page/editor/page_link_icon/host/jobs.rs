use std::{path::PathBuf, sync::Arc};

use gpui::Context;

use super::super::{
    commit::PendingPageLinkIconCommit, picker::PageLinkCustomEmojiLibraryRequest,
    preview::PageLinkIconPreviewTarget,
};
use super::{PageLinkIconCompletion, PageLinkIconEvent};
use crate::model::{NotionLocalIconFileApi, NotionWorkspaceApi};
use crate::ui::{load_notion_external_icon, SurfaceState};

pub(super) struct PageLinkIconLibraryJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) request: PageLinkCustomEmojiLibraryRequest,
}

pub(super) enum PageLinkIconPreviewJob {
    Local {
        target: PageLinkIconPreviewTarget,
        path: PathBuf,
        local_icon_files: Arc<dyn NotionLocalIconFileApi>,
    },
    External {
        target: PageLinkIconPreviewTarget,
        value: String,
        remote_images: Arc<dyn remote_image_model::RemoteImageApi>,
    },
}

pub(super) struct PageLinkIconCommitJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) pending: PendingPageLinkIconCommit,
}

impl PageLinkIconLibraryJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        gpui_components::spawn_background_task_for_entity(
            (self.workspace_api, self.request),
            cx,
            |(workspace_api, request)| {
                let result = workspace_api.load_custom_emoji_library();
                PageLinkIconCompletion::CustomEmojiLibrary { request, result }
            },
            |surface, completion, cx| {
                surface.handle_page_link_icon_event(
                    PageLinkIconEvent::Completion(Box::new(completion)),
                    None,
                    cx,
                );
            },
        );
    }
}

impl PageLinkIconPreviewJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        match self {
            Self::Local {
                target,
                path,
                local_icon_files,
            } => gpui_components::spawn_background_task_for_entity(
                (target, path, local_icon_files),
                cx,
                |(target, path, local_icon_files)| {
                    let result = local_icon_files.prepare_page_icon(path);
                    PageLinkIconCompletion::LocalPreview { target, result }
                },
                |surface, completion, cx| {
                    surface.handle_page_link_icon_event(
                        PageLinkIconEvent::Completion(Box::new(completion)),
                        None,
                        cx,
                    );
                },
            ),
            Self::External {
                target,
                value,
                remote_images,
            } => gpui_components::spawn_background_task_for_entity(
                (target, value, remote_images),
                cx,
                |(target, value, remote_images)| {
                    let result = load_notion_external_icon(remote_images.as_ref(), &value);
                    PageLinkIconCompletion::ExternalPreview {
                        target,
                        value,
                        result,
                    }
                },
                |surface, completion, cx| {
                    surface.handle_page_link_icon_event(
                        PageLinkIconEvent::Completion(Box::new(completion)),
                        None,
                        cx,
                    );
                },
            ),
        }
    }
}

impl PageLinkIconCommitJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        gpui_components::spawn_background_task_for_entity(
            (self.workspace_api, self.pending),
            cx,
            |(workspace_api, pending)| {
                PageLinkIconCompletion::Commit(pending.execute(workspace_api.as_ref()))
            },
            |surface, completion, cx| {
                surface.handle_page_link_icon_event(
                    PageLinkIconEvent::Completion(Box::new(completion)),
                    None,
                    cx,
                );
            },
        );
    }
}
