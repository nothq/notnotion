use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::{PageMutationAction, PageMutationPlan};
use gpui::{Context, IntoElement, Render, Window};

use crate::model::{CardPageColumnEffectiveShare, PageColumnPair, PageMutation};
use crate::ui::surface::{
    PageColumnResizeGeometry, PageColumnResizeOrigin, PageColumnResizeSession,
    PageColumnResizeSource, PageEditorState, PageFlowDragAuthority, PageFlowSurfaceKey,
};
use crate::ui::{LoadedCardPageData, PageColumnFlow};

use super::projection::PageEditorDragReset;
use super::{Arc, SurfaceState};

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct PageColumnResizeDrag {
    data: Arc<LoadedCardPageData>,
    surface: PageFlowSurfaceKey,
    column_list_id: Arc<str>,
    left_column_id: Arc<str>,
    right_column_id: Arc<str>,
    source_left_share: CardPageColumnEffectiveShare,
    pair_share_total: f64,
}

impl PageColumnResizeDrag {
    pub(super) fn new(
        data: Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        column_list_id: Arc<str>,
        left: &PageColumnFlow,
        right: &PageColumnFlow,
    ) -> Self {
        Self {
            data,
            surface,
            column_list_id,
            left_column_id: left.column_block_id.clone(),
            right_column_id: right.column_block_id.clone(),
            source_left_share: left.effective_share,
            pair_share_total: left.effective_share.fraction() + right.effective_share.fraction(),
        }
    }

    fn strict_pair(&self, data: &LoadedCardPageData) -> Option<PageColumnPair> {
        PageColumnPair::new(
            &data.page,
            self.column_list_id.as_ref(),
            self.left_column_id.as_ref(),
            self.right_column_id.as_ref(),
        )
        .ok()
    }

    fn resize_source(&self) -> PageColumnResizeSource<'_> {
        PageColumnResizeSource {
            data: &self.data,
            surface: self.surface,
            column_list_id: &self.column_list_id,
            left_column_id: &self.left_column_id,
            right_column_id: &self.right_column_id,
        }
    }
}

impl Render for PageColumnResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

impl SurfaceState {
    pub(super) fn begin_page_column_resize(
        &mut self,
        drag: &PageColumnResizeDrag,
        pointer_x: f32,
        cx: &mut Context<Self>,
    ) -> bool {
        if (self.notion_startup.workspace_api().is_none()
            && !self.notion_startup.cached_workspace_visible())
            || self.page_mutations.is_recovering(&drag.data.page.block_id)
        {
            return false;
        }
        let Some(data) = self
            .page_documents
            .active_page_data_with_id(&drag.data.page.block_id)
        else {
            return false;
        };
        if !Arc::ptr_eq(&data, &drag.data) {
            return false;
        }
        let Some(pair) = drag.strict_pair(&data) else {
            return false;
        };
        let (layout_authority, left, right) = {
            let observations = self.page_editor.flow.state().observations.borrow();
            let Some(authority) = observations.drag_authority(drag.surface, &data) else {
                return false;
            };
            let Some((left, right)) = divider_lane_bounds(&authority, &pair) else {
                return false;
            };
            (authority.layout_authority(), left, right)
        };
        let Some(geometry) =
            PageColumnResizeGeometry::new(pointer_x, left, right, drag.pair_share_total)
        else {
            return false;
        };
        self.page_editor
            .reset_drag(PageEditorDragReset::PreserveHandleInteraction);
        self.page_editor.drag.borrow_mut().column_resize = Some(PageColumnResizeSession::new(
            data,
            drag.surface,
            layout_authority,
            pair,
            PageColumnResizeOrigin {
                source_left_share: drag.source_left_share,
                geometry,
            },
        ));
        cx.notify();
        true
    }

    pub(super) fn update_page_column_resize_from_pointer(
        &mut self,
        drag: &PageColumnResizeDrag,
        pointer_x: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(data) = self
            .page_documents
            .active_page_data_with_id(&drag.data.page.block_id)
        else {
            self.page_editor.cancel_page_column_resize(cx);
            return;
        };
        let current = self
            .page_editor
            .drag
            .borrow()
            .column_resize
            .as_ref()
            .is_some_and(|session| {
                self.page_editor
                    .flow
                    .state()
                    .observations
                    .borrow()
                    .layout_authority_is_current(session.authority(), &data)
            });
        let mut drag_state = self.page_editor.drag.borrow_mut();
        let Some(session) = drag_state.column_resize.as_mut() else {
            return;
        };
        if !session.matches_source(drag.resize_source()) || !current {
            drop(drag_state);
            self.page_editor.cancel_page_column_resize(cx);
            return;
        }
        if session.update(pointer_x) {
            cx.notify();
        }
    }

    pub(super) fn finish_page_column_resize(
        &mut self,
        drag: &PageColumnResizeDrag,
        pointer_x: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(mut session) = self.page_editor.drag.borrow_mut().column_resize.take() else {
            return;
        };
        session.update(pointer_x);
        let Some(data) = self
            .page_documents
            .active_page_data_with_id(&drag.data.page.block_id)
        else {
            cx.notify();
            return;
        };
        let current = self
            .page_editor
            .flow
            .state()
            .observations
            .borrow()
            .layout_authority_is_current(session.authority(), &data);
        if !valid_resize_finish(self, &session, drag, &data, current) {
            cx.notify();
            return;
        }
        let Ok((page, request)) = session.apply_final_request(&data.page) else {
            cx.notify();
            return;
        };
        let focus = self.page_editor.current_page_edit_focus(cx);
        if self
            .page_editor
            .record_page_column_ratio_edit(&data.page, session.pair().clone(), focus)
            .is_err()
        {
            cx.notify();
            return;
        }
        let page_id = page.block_id.clone();
        self.dispatch_page_mutation_action(
            PageMutationAction::ApplyPlan(PageMutationPlan::mutation(
                &page_id,
                PageMutation::ResizeColumns(request),
            )),
            cx,
        );
        self.dispatch_page_document_action(PageDocumentAction::replace_loaded(page), cx);
        cx.notify();
    }
}

impl PageEditorState {
    pub(super) fn defer_stale_page_column_resize(&self, cx: &mut Context<SurfaceState>) {
        if self.drag.borrow().column_resize.is_none() || cx.has_active_drag() {
            return;
        }
        let weak = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = weak.update(cx, |this, cx| {
                if this
                    .page_editor
                    .drag
                    .borrow_mut()
                    .column_resize
                    .take()
                    .is_some()
                {
                    cx.notify();
                }
            });
        });
    }

    fn cancel_page_column_resize(&mut self, cx: &mut Context<SurfaceState>) {
        if self.drag.borrow_mut().column_resize.take().is_some() {
            cx.notify();
        }
    }
}

type LeftRightLaneBounds = (gpui::Bounds<gpui::Pixels>, gpui::Bounds<gpui::Pixels>);

fn divider_lane_bounds(
    authority: &PageFlowDragAuthority<'_>,
    pair: &PageColumnPair,
) -> Option<LeftRightLaneBounds> {
    let find = |column_id: &str| {
        authority.lanes().iter().find(|lane| {
            lane.lane_path.column_ids() == Some((pair.column_list_block_id(), column_id))
        })
    };
    Some((
        find(pair.left_column_block_id())?.lane_bounds,
        find(pair.right_column_block_id())?.lane_bounds,
    ))
}

fn valid_resize_finish(
    surface: &SurfaceState,
    session: &PageColumnResizeSession,
    drag: &PageColumnResizeDrag,
    data: &Arc<LoadedCardPageData>,
    current: bool,
) -> bool {
    (surface.notion_startup.workspace_api().is_some()
        || surface.notion_startup.cached_workspace_visible())
        && !surface.page_mutations.is_recovering(&data.page.block_id)
        && session.matches_source(drag.resize_source())
        && current
        && session.preview().is_some()
}
