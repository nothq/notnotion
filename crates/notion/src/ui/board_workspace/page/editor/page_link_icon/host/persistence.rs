use std::sync::Arc;

use gpui::Context;

use super::super::super::PageEditSession;
use super::super::{
    persistence::{prepare_page_link_icon_edit, PageLinkIconPersistence},
    selection::PageLinkIconSetEffect,
    PageLinkIconController, PageLinkIconPickerState,
};
use super::{PageLinkIconEffectHost, PageLinkIconRootEffect};
use crate::ui::board_workspace::PageMutationAction;
use crate::ui::{AppearanceMode, PageShellIcon, SurfaceState};

pub(super) struct PageLinkIconPreparedEdit {
    pub(super) transition: super::PageEditTransition<()>,
    pub(super) mutation: Option<PageMutationAction>,
    pub(super) finish: PageLinkIconSetFinish,
}

pub(super) enum PageLinkIconSetFinish {
    None,
    Queue {
        previous_picker: Option<Box<PageLinkIconPickerState>>,
        icon: Option<PageShellIcon>,
        applied: bool,
        keep_picker_open: bool,
        appearance_mode: AppearanceMode,
    },
}

impl PageLinkIconSetFinish {
    pub(super) fn apply(
        self,
        controller: &mut PageLinkIconController,
        cx: &mut Context<SurfaceState>,
    ) {
        let Self::Queue {
            previous_picker,
            icon,
            applied,
            keep_picker_open,
            appearance_mode,
        } = self
        else {
            return;
        };
        if !applied {
            return;
        }
        controller.finish_applied_set_icon(
            previous_picker.map(|picker| *picker),
            icon.as_ref(),
            keep_picker_open,
            appearance_mode,
        );
        cx.notify();
    }
}

impl PageLinkIconEffectHost<'_> {
    pub(super) fn prepare_set_icon(
        &mut self,
        effect: PageLinkIconSetEffect,
        persistence: PageLinkIconPersistence,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        if persistence == PageLinkIconPersistence::Queue && self.queue_edit_is_blocked(&effect) {
            return Vec::new();
        }
        if let (Some(icon), Some(rendered)) = (&effect.icon, effect.rendered.as_ref()) {
            self.resources.external_icon_cache().insert(
                icon.render_value().to_string(),
                Arc::clone(rendered),
                cx,
            );
        }
        let previous_picker = effect
            .keep_picker_open
            .then(|| self.editor.page_link_icons.picker().cloned())
            .flatten();
        let Some(page) = self.documents.page_with_id(&effect.target.page_id) else {
            return Vec::new();
        };
        let Some(edit) =
            prepare_page_link_icon_edit(page, &effect.target.block_id, effect.icon.clone())
        else {
            return Vec::new();
        };
        let workspace_available = self.workspace_api.is_some() || self.cached_workspace_visible;
        let (transition, application) = {
            let mut session = PageEditSession::new(self.editor, self.documents);
            let application = session.apply_page_link_icon_edit(
                edit,
                persistence,
                workspace_available,
                self.focus.take(),
            );
            (session.finish(()), application)
        };
        let finish = match persistence {
            PageLinkIconPersistence::Queue => PageLinkIconSetFinish::Queue {
                previous_picker: previous_picker.map(Box::new),
                icon: effect.icon,
                applied: application.applied,
                keep_picker_open: effect.keep_picker_open,
                appearance_mode: self.appearance_mode,
            },
            PageLinkIconPersistence::AlreadyCommitted => PageLinkIconSetFinish::None,
        };
        vec![PageLinkIconRootEffect::ApplyEdit(Box::new(
            PageLinkIconPreparedEdit {
                transition,
                mutation: application.mutation,
                finish,
            },
        ))]
    }

    fn queue_edit_is_blocked(&self, effect: &PageLinkIconSetEffect) -> bool {
        self.editor
            .page_link_icons
            .commit_in_flight(&effect.target.page_id, &effect.target.block_id)
            || self.editor.page_link_icons.picker().is_some_and(|picker| {
                picker.page_id == effect.target.page_id
                    && picker.block_id == effect.target.block_id
                    && picker.upload_committed
            })
    }
}
