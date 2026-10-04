use std::{rc::Rc, sync::Arc};

use gpui::Context;
use gpui_components::text_input::{TextInputGhost, TextInputKeyDownPreAction};

use super::super::code::PageCodeSyntaxRenderer;
use super::super::context_menu::{page_block_context_menu_renderer, PageBlockContextMenuRenderer};
use super::super::input::PageInputBindings;
use super::super::mention::{
    page_mention_action_sink, PageMentionClock, PageMentionController, PageMentionRenderer,
    PageMentionRendererResources,
};
use super::super::simple_table::simple_table_on_key_down;
use super::super::{page_link_icon::PageLinkIconPresentation, slash::PageSlashMenuRenderer};
use super::{
    handle_page_render_action, BlockMentionPillRange, PageBlockRenderer, PageDocumentColumn,
    PageDocumentLayout, PageMentionFrame, PageRenderAction, PageRenderInteraction,
};
use crate::ui::block_images::NotionBlockImageView;
use crate::ui::board_workspace::inline_database::InlineDatabaseBlockRenderer;
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::surface::{
    NotionSurfaceResources, PageDocumentDragRuntime, PageDocumentFlowRuntime, PageEditorState,
    PageInputResources, PageSimpleTableRuntime,
};
use crate::ui::view_actions::{ViewActionSink, ViewNotifier};
use crate::ui::{
    AppearanceMode, CardPage, IconSet, LoadedCardPage, LoadedCardPageData, SurfaceState, Theme,
    Viewport,
};

struct PageBlockVisualResources {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    viewport: Viewport,
    notion: NotionSurfaceResources,
}

struct PageBlockRuntimeCapture {
    input_resources: PageInputResources,
    flow: PageDocumentFlowRuntime,
    drag: PageDocumentDragRuntime,
    tables: PageSimpleTableRuntime,
    interaction: Rc<PageRenderInteraction>,
    block_images: NotionBlockImageView,
    code_syntax: PageCodeSyntaxRenderer,
    input_bindings: PageInputBindings,
    actions: ViewActionSink<PageRenderAction>,
    notifier: ViewNotifier,
    simple_table_key_down: TextInputKeyDownPreAction,
}

struct PageBlockHostFrame {
    authority_page: Option<Arc<CardPage>>,
    board_url: Option<String>,
    column: PageDocumentColumn,
    page_icons: PageShellIconRenderer,
    inline_databases: InlineDatabaseBlockRenderer,
    context_menu: Option<PageBlockContextMenuRenderer>,
    page_link_icons: Option<PageLinkIconPresentation>,
    slash_menu: Option<PageSlashMenuRenderer>,
}

struct PageMentionCapture {
    renderer: PageMentionRenderer,
    menu: Option<PageMentionFrame>,
    clock: PageMentionClock,
    ghost: Option<(String, TextInputGhost)>,
    pill_range: Option<BlockMentionPillRange>,
}

impl SurfaceState {
    pub(in crate::ui::board_workspace) fn page_block_renderer(
        &self,
        page: &LoadedCardPage,
        layout: PageDocumentLayout,
        cx: &mut Context<Self>,
    ) -> PageBlockRenderer {
        let data = page.data.clone();
        let column = PageDocumentColumn::new(
            layout,
            data.page.format,
            self.page_layout().main_pane_width(),
        );
        let visual = PageBlockVisualResources::new(
            self.theme,
            self.appearance_mode,
            Arc::clone(&self.icons),
            self.viewport,
            self.notion_resources.clone(),
        );
        let runtime = PageBlockRuntimeCapture::capture(&self.page_editor, &visual, page, &data, cx);
        let mention_resources = visual.mention_resources(
            runtime.notifier.clone(),
            PageMentionClock::from(&self.board),
        );
        let mention = PageMentionCapture::capture(&self.page_editor.mention, mention_resources, cx);
        let context_menu = self
            .page_editor
            .page_block_context_menu
            .as_ref()
            .and_then(|menu| page_block_context_menu_renderer(self, &menu.block_id, cx));
        let host = PageBlockHostFrame {
            authority_page: self
                .page_documents
                .page_authority_with_id(&data.page.block_id),
            board_url: self.notion_startup.board_url(),
            column,
            page_icons: self.page_shell_icon_renderer(cx),
            inline_databases: self.inline_database_block_renderer(page, column, cx),
            context_menu,
            page_link_icons: self.page_link_icon_presentation(cx),
            slash_menu: self.page_slash_menu_renderer(cx),
        };
        PageBlockRenderer::from_captures(data, visual, host, runtime, mention)
    }
}

impl PageBlockRenderer {
    fn from_captures(
        data: Arc<LoadedCardPageData>,
        visual: PageBlockVisualResources,
        host: PageBlockHostFrame,
        runtime: PageBlockRuntimeCapture,
        mention: PageMentionCapture,
    ) -> Self {
        Self {
            data,
            authority_page: host.authority_page,
            theme: visual.theme,
            appearance_mode: visual.appearance_mode,
            icons: visual.icons,
            viewport: visual.viewport,
            board_url: host.board_url,
            resources: visual.notion,
            column: host.column,
            interaction: runtime.interaction,
            input_resources: runtime.input_resources,
            flow: runtime.flow,
            drag: runtime.drag,
            tables: runtime.tables,
            page_icons: host.page_icons,
            block_images: runtime.block_images,
            inline_databases: host.inline_databases,
            context_menu: host.context_menu,
            page_link_icons: host.page_link_icons,
            code_syntax: runtime.code_syntax,
            input_bindings: runtime.input_bindings,
            slash_menu: host.slash_menu,
            mention_renderer: mention.renderer,
            mention_menu: mention.menu,
            mention_clock: mention.clock,
            mention_ghost: mention.ghost,
            mention_pill_range: mention.pill_range,
            actions: runtime.actions,
            notifier: runtime.notifier,
            simple_table_key_down: runtime.simple_table_key_down,
        }
    }
}

impl PageBlockVisualResources {
    fn new(
        theme: Theme,
        appearance_mode: AppearanceMode,
        icons: Arc<IconSet>,
        viewport: Viewport,
        notion: NotionSurfaceResources,
    ) -> Self {
        Self {
            theme,
            appearance_mode,
            icons,
            viewport,
            notion,
        }
    }

    fn mention_resources(
        &self,
        notifier: ViewNotifier,
        clock: PageMentionClock,
    ) -> PageMentionRendererResources {
        PageMentionRendererResources {
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            icons: Arc::clone(&self.icons),
            notion_resources: self.notion.clone(),
            notifier,
            viewport_height: self.viewport.app_height(),
            chrome_top_inset: self.viewport.chrome_top_inset(),
            clock,
        }
    }
}

impl PageBlockRuntimeCapture {
    fn capture(
        editor: &PageEditorState,
        visual: &PageBlockVisualResources,
        page: &LoadedCardPage,
        data: &Arc<LoadedCardPageData>,
        cx: &mut Context<SurfaceState>,
    ) -> Self {
        let notifier = ViewNotifier::new(cx);
        let input_resources = editor.input.resources();
        let actions = ViewActionSink::new(cx, handle_page_render_action);
        Self {
            input_resources: input_resources.clone(),
            flow: editor.flow.clone(),
            drag: editor.drag.clone(),
            tables: editor.tables.clone(),
            interaction: Rc::new(PageRenderInteraction::capture(
                editor,
                &data.page.block_id,
                cx.has_active_drag(),
            )),
            block_images: NotionBlockImageView::new(
                visual.notion.clone(),
                notifier.clone(),
                editor.flow.clone(),
                page.list_state.clone(),
            ),
            code_syntax: PageCodeSyntaxRenderer::new(
                input_resources,
                visual.appearance_mode,
                notifier.clone(),
            ),
            input_bindings: PageInputBindings::capture(data, cx),
            actions: actions.clone(),
            notifier,
            simple_table_key_down: simple_table_on_key_down(editor.tables.clone(), actions),
        }
    }
}

impl PageMentionCapture {
    fn capture(
        mention: &PageMentionController,
        resources: PageMentionRendererResources,
        cx: &mut Context<SurfaceState>,
    ) -> Self {
        let clock = resources.clock.clone();
        let mention_menu = mention.menu().and_then(|menu| {
            mention
                .presentation_seed_for_block(&menu.block_id, &clock)
                .map(|presentation| PageMentionFrame {
                    block_id: menu.block_id.clone(),
                    trigger_offset: menu.trigger_offset,
                    presentation,
                })
        });
        let mention_block_id = mention_menu.as_ref().map(|menu| menu.block_id.as_str());
        let mention_ghost = mention_block_id.and_then(|block_id| {
            mention
                .input_ghost(block_id, &clock, resources.appearance_mode)
                .map(|ghost| (block_id.to_string(), ghost))
        });
        let mention_pill_range = mention_block_id.and_then(|block_id| {
            mention
                .pill_range(block_id)
                .map(|range| (block_id.to_string(), range))
        });
        let mention_renderer = PageMentionRenderer::new(resources, page_mention_action_sink(cx));
        Self {
            renderer: mention_renderer,
            menu: mention_menu,
            clock,
            ghost: mention_ghost,
            pill_range: mention_pill_range,
        }
    }
}
