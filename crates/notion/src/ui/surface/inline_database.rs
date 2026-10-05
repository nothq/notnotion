use gpui::{AppContext, Entity, ListState, Render, WeakEntity};

use super::{NotionStartup, NotionSurfaceResources, SurfaceState};
use crate::model::NotionCollectionViewId;
use crate::ui::board_workspace::inline_database::InlineDatabaseLayout;
use crate::ui::board_workspace::{InlineDatabaseToolbarTarget, PageDocumentColumn};
use crate::ui::{
    alpha, div, px, AppearanceMode, CardPageStructuralBlock, Context, Div, IntoElement,
    LoadedCardPage, NotionWorkspaceApi, NotionWorkspaceBootstrap, ParentElement, Styled, Theme,
    Viewport, Window,
};

const INLINE_DATABASE_LOADING_HEIGHT: f32 = 874.0;

enum InlineDatabaseLoadState {
    Loading,
    Loaded { surface: Entity<SurfaceState> },
    Failed,
}

pub(crate) struct InlineDatabaseView {
    collection_view_block_id: String,
    page_id: String,
    layout: InlineDatabaseLayout,
    title: String,
    appearance_mode: AppearanceMode,
    viewport: Viewport,
    parent_surface: WeakEntity<SurfaceState>,
    page_list_state: Option<ListState>,
    workspace_api: crate::ui::Arc<dyn NotionWorkspaceApi>,
    notion_resources: NotionSurfaceResources,
    view_request_id: u64,
    active_view_id: Option<NotionCollectionViewId>,
    pending_view_id: Option<NotionCollectionViewId>,
    measured_height: f32,
    state: InlineDatabaseLoadState,
}

struct InlineDatabaseViewInput {
    collection_view_block_id: String,
    page_id: String,
    layout: InlineDatabaseLayout,
    title: String,
    appearance_mode: AppearanceMode,
    viewport: Viewport,
    parent_surface: WeakEntity<SurfaceState>,
    page_list_state: Option<ListState>,
    workspace_api: crate::ui::Arc<dyn NotionWorkspaceApi>,
    notion_resources: NotionSurfaceResources,
}

impl InlineDatabaseView {
    fn new(input: InlineDatabaseViewInput, cx: &mut Context<Self>) -> Self {
        let InlineDatabaseViewInput {
            collection_view_block_id,
            page_id,
            layout,
            title,
            appearance_mode,
            viewport,
            parent_surface,
            page_list_state,
            workspace_api,
            notion_resources,
        } = input;
        let load_block_id = collection_view_block_id.clone();
        let load_api = workspace_api.clone();
        let task = cx
            .background_executor()
            .spawn(async move { load_api.load_inline_database(&load_block_id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| this.finish_load(result, cx));
        })
        .detach();

        Self {
            collection_view_block_id,
            page_id,
            layout,
            title,
            appearance_mode,
            viewport,
            parent_surface,
            page_list_state,
            workspace_api,
            notion_resources,
            view_request_id: 0,
            active_view_id: None,
            pending_view_id: None,
            measured_height: INLINE_DATABASE_LOADING_HEIGHT,
            state: InlineDatabaseLoadState::Loading,
        }
    }

    fn finish_load(
        &mut self,
        result: crate::model::NotionWorkspaceResult<NotionWorkspaceBootstrap>,
        cx: &mut Context<Self>,
    ) {
        self.state = match result {
            Ok(bootstrap) => {
                self.active_view_id = Some(active_provider_view_id(&bootstrap));
                let surface = self.surface_state_for_bootstrap(bootstrap);
                let surface = cx.new(|_| surface);
                self.measured_height = surface
                    .read(cx)
                    .inline_database_metrics(self.layout)
                    .total_height();
                cx.observe(&surface, |this, surface, cx| {
                    let next_height = surface
                        .read(cx)
                        .inline_database_metrics(this.layout)
                        .total_height();
                    if (this.measured_height - next_height).abs() > f32::EPSILON {
                        this.measured_height = next_height;
                        if let Some(list_state) = this.page_list_state.as_ref() {
                            list_state.remeasure();
                        }
                    }
                    cx.notify();
                })
                .detach();
                InlineDatabaseLoadState::Loaded { surface }
            }
            Err(error) => {
                let operation = format!("inline database {} failed to load", self.title);
                let _ = self.parent_surface.update(cx, |surface, cx| {
                    surface.handle_notion_workspace_failure(&operation, error, cx);
                });
                InlineDatabaseLoadState::Failed
            }
        };
        if let Some(list_state) = self.page_list_state.as_ref() {
            list_state.remeasure();
        }
        cx.notify();
    }

    fn surface_state_for_bootstrap(&self, bootstrap: NotionWorkspaceBootstrap) -> SurfaceState {
        let mut surface = SurfaceState::from_startup(
            NotionStartup::Ready(bootstrap),
            self.appearance_mode,
            self.viewport,
            true,
            self.notion_resources.clone(),
        );
        surface.preview_width = self.layout.database_width;
        surface.notion_chrome.notion_sidebar_visible = false;
        surface.presentation.page_host = Some(self.parent_surface.clone());
        surface
    }

    pub(crate) fn select_provider_view(
        &mut self,
        provider_view_id: NotionCollectionViewId,
        cx: &mut Context<Self>,
    ) {
        let already_active = self.active_view_id.as_ref() == Some(&provider_view_id);
        if already_active {
            if self.pending_view_id.take().is_some() {
                self.view_request_id = self
                    .view_request_id
                    .checked_add(1)
                    .expect("inline database view request id exhausted");
            }
            return;
        }
        if self.pending_view_id.as_ref() == Some(&provider_view_id) {
            return;
        }

        self.view_request_id = self
            .view_request_id
            .checked_add(1)
            .expect("inline database view request id exhausted");
        let request_id = self.view_request_id;
        self.pending_view_id = Some(provider_view_id.clone());
        let workspace_api = self.workspace_api.clone();
        let database_block_id = self.collection_view_block_id.clone();
        let requested_view_id = provider_view_id.clone();
        let task = cx.background_executor().spawn(async move {
            workspace_api.load_inline_database_view(&database_block_id, &requested_view_id)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.finish_view_load(request_id, provider_view_id, result, cx);
            });
        })
        .detach();
    }

    fn finish_view_load(
        &mut self,
        request_id: u64,
        provider_view_id: NotionCollectionViewId,
        result: crate::model::NotionWorkspaceResult<NotionWorkspaceBootstrap>,
        cx: &mut Context<Self>,
    ) {
        if self.view_request_id != request_id
            || self.pending_view_id.as_ref() != Some(&provider_view_id)
        {
            return;
        }
        self.pending_view_id = None;

        match result {
            Ok(bootstrap) => {
                let InlineDatabaseLoadState::Loaded { surface } = &self.state else {
                    return;
                };
                let surface = surface.clone();
                let mut replacement = self.surface_state_for_bootstrap(bootstrap);
                surface.update(cx, |surface, cx| {
                    replacement
                        .page_editor
                        .inherit_session_state(&surface.page_editor);
                    *surface = replacement;
                    cx.notify();
                });
                self.active_view_id = Some(provider_view_id);
                self.measured_height = surface
                    .read(cx)
                    .inline_database_metrics(self.layout)
                    .total_height();
                if let Some(list_state) = self.page_list_state.as_ref() {
                    list_state.remeasure();
                }
                cx.notify();
            }
            Err(error) => {
                let _ = self.parent_surface.update(cx, |surface, cx| {
                    surface.handle_notion_workspace_failure(
                        "inline database view load failed",
                        error,
                        cx,
                    );
                });
            }
        }
    }

    fn render_loading(&self) -> Div {
        let theme = Theme::for_appearance_mode(self.appearance_mode);
        div()
            .w(px(self.layout.database_width))
            .h(px(INLINE_DATABASE_LOADING_HEIGHT))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(40.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .children([92.0, 88.0, 96.0].map(|width| {
                        div()
                            .w(px(width))
                            .h(px(28.0))
                            .rounded(px(20.0))
                            .bg(alpha(theme.text_primary, 0.045))
                    })),
            )
            .child(
                div()
                    .h(px(INLINE_DATABASE_LOADING_HEIGHT - 40.0))
                    .pl(px(8.0))
                    .pt(px(8.0))
                    .flex()
                    .gap(px(12.0))
                    .children((0..3).map(|column| {
                        div()
                            .w(px(276.0))
                            .h(px(520.0 - column as f32 * 72.0))
                            .rounded(px(10.0))
                            .bg(alpha(theme.text_primary, 0.025))
                    })),
            )
    }

    fn render_failed(&self) -> Div {
        div().w(px(self.layout.database_width)).min_h(px(64.0))
    }
}

fn active_provider_view_id(bootstrap: &NotionWorkspaceBootstrap) -> NotionCollectionViewId {
    bootstrap
        .workspace
        .view_tabs
        .iter()
        .find(|tab| tab.active)
        .expect("inline database snapshot must retain one active provider view")
        .provider_view_id
        .clone()
}

impl Render for InlineDatabaseView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(parent) = self.parent_surface.upgrade() {
            let parent = parent.read(cx);
            self.layout = InlineDatabaseLayout::for_page_column(
                parent
                    .page_documents
                    .hosted_page_column(&self.page_id, parent.page_layout().main_pane_width()),
            );
        }
        let layout = self.layout;
        match &self.state {
            InlineDatabaseLoadState::Loading => self.render_loading().into_any_element(),
            InlineDatabaseLoadState::Loaded { surface } => {
                let inline_view = cx.entity().downgrade();
                let board_url = surface
                    .read(cx)
                    .notion_startup
                    .board_url()
                    .expect("loaded inline database must retain its board URL");
                let toolbar_target = InlineDatabaseToolbarTarget {
                    parent_surface: self.parent_surface.clone(),
                    inline_view,
                    database_block_id: self.collection_view_block_id.clone(),
                    board_url,
                    database_title: self.title.clone(),
                };
                surface.update(cx, |surface, cx| {
                    surface.preview_width = layout.database_width;
                    surface.render_inline_database_surface(layout, toolbar_target, cx)
                })
            }
            InlineDatabaseLoadState::Failed => self.render_failed().into_any_element(),
        }
    }
}

impl SurfaceState {
    pub(crate) fn inline_database_block_renderer(
        &self,
        page: &LoadedCardPage,
        column: PageDocumentColumn,
        cx: &mut Context<Self>,
    ) -> crate::ui::board_workspace::inline_database::InlineDatabaseBlockRenderer {
        let mut views = std::collections::HashMap::new();
        for block in &page.data.page.blocks {
            let Some(CardPageStructuralBlock::CollectionView { title }) =
                block.structural_content()
            else {
                continue;
            };
            let key = format!("{}:{}", page.data.page.block_id, block.block_id);
            let existing = self
                .presentation
                .inline_database_views
                .borrow()
                .get(&key)
                .cloned();
            let view = existing.or_else(|| {
                let workspace_api = self.notion_startup.workspace_api()?;
                let parent_surface = cx.entity().downgrade();
                let collection_view_block_id = block.block_id.clone();
                let page_id = page.data.page.block_id.clone();
                let title = title.as_deref().unwrap_or("Untitled database").to_string();
                let appearance_mode = self.appearance_mode;
                let viewport = self.viewport;
                let page_list_state = page.list_state.clone();
                let notion_resources = self.notion_resources.clone();
                let view = cx.new(move |cx| {
                    InlineDatabaseView::new(
                        InlineDatabaseViewInput {
                            collection_view_block_id,
                            page_id,
                            layout: InlineDatabaseLayout::for_page_column(column),
                            title,
                            appearance_mode,
                            viewport,
                            parent_surface,
                            page_list_state: Some(page_list_state),
                            workspace_api,
                            notion_resources,
                        },
                        cx,
                    )
                });
                self.presentation
                    .inline_database_views
                    .borrow_mut()
                    .insert(key.clone(), view.clone());
                Some(view)
            });
            if let Some(view) = view {
                views.insert(key, view);
            }
        }
        crate::ui::board_workspace::inline_database::InlineDatabaseBlockRenderer::new(views, column)
    }
}
