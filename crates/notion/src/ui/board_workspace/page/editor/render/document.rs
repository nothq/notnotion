use gpui::{list, App, FocusHandle, ListSizingBehavior, ListState};

use crate::ui::surface::PageFlowSurfaceKey;

use super::super::{
    div, px, AnyElement, Arc, Context, FluentBuilder, InteractiveElement, IntoElement,
    LoadedCardPage, LoadedCardPageData, PageBlockDragScrollTarget, PageBlockDragSelection,
    PageFlowNode, ParentElement, Styled, SurfaceState,
};
use crate::ui::board_workspace::page::composer::{
    PageComposerRenderer, PageComposerVisualResources,
};
use crate::ui::board_workspace::page::properties::PagePropertiesRenderer;
use crate::ui::board_workspace::PageShellBodyRenderer;
use crate::ui::{view_actions::ViewActionSink, PageShellSnapshot};

mod column;
mod flow;
mod linear;
mod preparation;

use super::title::PageTitleRenderer;
use super::{handle_page_render_action, PageBlockRenderer};
pub(crate) use column::PageDocumentColumn;
use flow::{PageFlowOuterListFrame, PageFlowRootNodeRender};
pub(super) use linear::normalize_page_block_drag_row_bounds;
use linear::PageBlockSectionRender;
use preparation::{PageDocumentRenderMode, PageDocumentRenderState};

#[derive(Clone, Copy)]
pub(crate) enum PageDocumentLayout {
    Standalone,
    SelectedPage,
}

#[derive(Clone)]
struct PageDocumentRenderer {
    blocks: PageBlockRenderer,
    title: PageTitleRenderer,
    properties: PagePropertiesRenderer,
    composer: PageComposerRenderer,
    shell_body: PageShellBodyRenderer,
    page_shell: Option<PageShellSnapshot>,
    column: PageDocumentColumn,
}

impl PageDocumentLayout {
    fn scroll_target(self) -> PageBlockDragScrollTarget {
        match self {
            Self::Standalone => PageBlockDragScrollTarget::Standalone,
            Self::SelectedPage => PageBlockDragScrollTarget::SelectedPage,
        }
    }

    fn flow_surface_key(self) -> PageFlowSurfaceKey {
        match self {
            Self::Standalone => PageFlowSurfaceKey::Standalone,
            Self::SelectedPage => PageFlowSurfaceKey::SelectedPage,
        }
    }

    pub(crate) fn for_flow_surface(surface: PageFlowSurfaceKey) -> Self {
        match surface {
            PageFlowSurfaceKey::Standalone => Self::Standalone,
            PageFlowSurfaceKey::SelectedPage => Self::SelectedPage,
        }
    }
}

struct PageDocumentItemRender {
    item_index: usize,
    section_count: usize,
    layout: PageDocumentLayout,
    data: Arc<LoadedCardPageData>,
    nesting_offsets: Arc<[f32]>,
    drag_selection: Arc<PageBlockDragSelection>,
    generation: u64,
    observation: crate::ui::surface::PageFlowObservationToken,
    focus_handle: FocusHandle,
    render_mode: PageDocumentRenderMode,
    outer_viewport: Option<crate::ui::surface::PageFlowViewport>,
    outer_layout_width: Option<crate::ui::surface::PageFlowLayoutWidth>,
    recursive_commit_list_state: Option<ListState>,
}

impl PageDocumentItemRender {
    fn new(
        item_index: usize,
        state: &PageDocumentRenderState,
        outer_frame: Option<&PageFlowOuterListFrame>,
        recursive_commit_list_state: Option<&ListState>,
    ) -> Self {
        Self {
            item_index,
            section_count: state.section_count,
            layout: state.layout,
            data: state.data.clone(),
            nesting_offsets: state.nesting_offsets.clone(),
            drag_selection: state.drag_selection.clone(),
            generation: state.generation,
            observation: state.observation.clone(),
            focus_handle: state.focus_handles[item_index].clone(),
            render_mode: state.render_mode.clone(),
            outer_viewport: outer_frame.map(|frame| frame.item_viewport(item_index)),
            outer_layout_width: outer_frame.map(PageFlowOuterListFrame::layout_width),
            recursive_commit_list_state: recursive_commit_list_state.cloned(),
        }
    }
}

/// Places an item in the page's column. The margin sits on each item
/// because a GPUI `list` lays items out across its whole width and ignores
/// its own horizontal padding.
fn center_page_document_item(item: AnyElement, column: PageDocumentColumn) -> AnyElement {
    div()
        .w_full()
        .px(px(column.margin()))
        .flex()
        .justify_center()
        .child(item)
        .into_any_element()
}

impl SurfaceState {
    pub(crate) fn render_page_document(
        &self,
        page: &LoadedCardPage,
        layout: PageDocumentLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.page_editor.defer_stale_page_column_resize(cx);
        let state = self.page_editor.prepare_page_document(page, layout, cx);
        let blocks = self.page_block_renderer(page, layout, cx);
        let actions = ViewActionSink::new(cx, handle_page_render_action);
        PageDocumentRenderer {
            blocks,
            title: PageTitleRenderer::new(
                self.theme,
                self.page_editor.input.resources(),
                page.data.page.block_id.clone(),
                cx,
            ),
            properties: PagePropertiesRenderer::new(
                self.theme,
                self.appearance_mode,
                Arc::clone(&self.icons),
                !self.board.is_locked && self.notion_startup.workspace_api().is_some(),
                actions.clone(),
            ),
            composer: PageComposerRenderer::capture(
                &self.page_editor,
                PageComposerVisualResources::new(
                    self.theme,
                    self.appearance_mode,
                    Arc::clone(&self.icons),
                ),
                &page.data.page.block_id,
                actions,
                cx,
            ),
            shell_body: self.page_shell_body_renderer(cx),
            page_shell: self.board.page_shell.clone(),
            column: PageDocumentColumn::new(
                layout,
                page.data.page.format,
                self.page_layout().main_pane_width(),
            ),
        }
        .render_page_document_list(page, state, cx)
    }
}

impl PageDocumentRenderer {
    fn render_page_document_list(
        &self,
        page: &LoadedCardPage,
        state: PageDocumentRenderState,
        _cx: &mut App,
    ) -> AnyElement {
        let section_count = state.section_count;
        let recursive = state.render_mode.recursive_frame().is_some();
        let outer_frame = recursive.then(|| {
            PageFlowOuterListFrame::capture(&page.list_state, section_count + 2, self.column)
        });
        let recursive_commit_list_state = recursive.then(|| page.list_state.clone());
        let renderer = self.clone();
        let document = list(page.list_state.clone(), move |item_index, _window, cx| {
            let item = PageDocumentItemRender::new(
                item_index,
                &state,
                outer_frame.as_ref(),
                recursive_commit_list_state.as_ref(),
            );
            renderer.render_page_document_item(item, cx)
        })
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .flex_grow(1.0)
        .min_w(px(0.0))
        .min_h(px(0.0))
        .w_full();
        document.into_any_element()
    }

    fn render_page_document_item(&self, item: PageDocumentItemRender, cx: &mut App) -> AnyElement {
        if item.item_index == 0 {
            return center_page_document_item(
                self.render_page_document_lead(item.layout, &item.data, &item.focus_handle, cx),
                self.column,
            );
        }
        if item.item_index == item.section_count + 1 {
            return center_page_document_item(
                self.render_page_document_footer(&item, cx),
                self.column,
            );
        }
        if let Some(frame) = item.render_mode.recursive_frame() {
            return self.render_recursive_page_document_item(&item, frame, cx);
        }
        self.render_existing_page_document_item(&item, cx)
    }

    fn render_recursive_page_document_item(
        &self,
        item: &PageDocumentItemRender,
        frame: &crate::ui::surface::PageFlowLayoutFrameToken,
        cx: &mut App,
    ) -> AnyElement {
        let node = &item.data.flow.root.nodes[item.item_index - 1];
        center_page_document_item(
            self.blocks.render_page_flow_root_node(
                PageFlowRootNodeRender {
                    data: &item.data,
                    node,
                    nesting_offsets: &item.nesting_offsets,
                    drag_selection: &item.drag_selection,
                    scroll_target: item.layout.scroll_target(),
                    observation: &item.observation,
                    frame,
                    viewport: item
                        .outer_viewport
                        .expect("recursive Notion root nodes require an outer viewport"),
                    layout_width: item
                        .outer_layout_width
                        .expect("recursive Notion root nodes require an outer layout width"),
                    commit_list_state: item
                        .recursive_commit_list_state
                        .as_ref()
                        .expect("recursive Notion root nodes require their outer list"),
                    focus_handle: &item.focus_handle,
                },
                cx,
            ),
            self.column,
        )
    }

    fn render_existing_page_document_item(
        &self,
        item: &PageDocumentItemRender,
        cx: &mut App,
    ) -> AnyElement {
        let PageFlowNode::Section(section) = &item.data.flow.root.nodes[item.item_index - 1] else {
            panic!("linear Notion rendering cannot receive a Columns flow item")
        };
        let document_unit_range = section.document_unit_range.clone();
        center_page_document_item(
            self.blocks.render_page_block_section(
                PageBlockSectionRender {
                    data: &item.data,
                    document_unit_range,
                    nesting_offsets: &item.nesting_offsets,
                    drag_selection: &item.drag_selection,
                    scroll_target: item.layout.scroll_target(),
                    generation: item.generation,
                    observation: &item.observation,
                    first_section: item.item_index == 1,
                    focus_handle: &item.focus_handle,
                },
                cx,
            ),
            self.column,
        )
    }

    fn render_page_document_lead(
        &self,
        layout: PageDocumentLayout,
        data: &Arc<LoadedCardPageData>,
        focus_handle: &FocusHandle,
        cx: &mut App,
    ) -> AnyElement {
        match layout {
            PageDocumentLayout::Standalone => div()
                .pt(px(
                    if self
                        .page_shell
                        .as_ref()
                        .is_some_and(|page_shell| page_shell.page_icon_is_explicit)
                    {
                        95.5
                    } else {
                        81.5
                    },
                ))
                .w_full()
                .when_some(self.column.maximum_width(), |lead, width| {
                    lead.max_w(px(width))
                })
                .flex()
                .flex_col()
                .items_start()
                .track_focus(focus_handle)
                .child(
                    self.shell_body.standalone_header(
                        self.title.render(
                            &data.page,
                            if data.page.format.small_text {
                                32.0
                            } else {
                                40.0
                            },
                            cx,
                        ),
                        (!data.page.properties.is_empty())
                            .then(|| self.properties.render(&data.page, cx)),
                        cx,
                    ),
                )
                .into_any_element(),
            PageDocumentLayout::SelectedPage => div()
                .h(px(16.0))
                .track_focus(focus_handle)
                .into_any_element(),
        }
    }

    fn render_page_document_footer(
        &self,
        item: &PageDocumentItemRender,
        cx: &mut App,
    ) -> AnyElement {
        let layout = item.layout;
        let data = &item.data;
        let has_no_sections = item.section_count == 0;
        let focus_handle = &item.focus_handle;
        let page_links = self
            .page_shell
            .as_ref()
            .map(|page_shell| page_shell.links.as_slice())
            .unwrap_or_default();
        let render_composer = !data.page.blocks.is_empty() || page_links.is_empty();
        div()
            .w_full()
            .when_some(self.column.maximum_width(), |footer, width| {
                footer.max_w(px(width))
            })
            .pb(px(40.0))
            .flex()
            .flex_col()
            .track_focus(focus_handle)
            .when(has_no_sections && render_composer, |footer| {
                footer.pt(px(16.0))
            })
            .when(render_composer, |footer| {
                footer.child(self.composer.render(data, cx))
            })
            .when(
                matches!(layout, PageDocumentLayout::Standalone) && !page_links.is_empty(),
                |footer| {
                    footer.child(
                        self.shell_body.standalone_links(
                            self.page_shell
                                .as_ref()
                                .expect("standalone page links require page shell"),
                            cx,
                        ),
                    )
                },
            )
            .into_any_element()
    }
}
