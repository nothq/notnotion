mod actions;
mod block_content;
mod callout;
mod code;
mod content;
mod controls;
mod document;
mod drag_container;
mod host;
mod links;
mod resource;
mod row;
mod title;
mod units;

pub(in crate::ui::board_workspace) use actions::{
    handle_page_render_action, PageBlockRenderAction, PageComposerRenderAction,
    PageDocumentRenderAction, PageDragRenderAction, PageLinearRowsObservation, PageRenderAction,
    PageTableHistoryRestore, PageTableRenderAction,
};
pub(super) use interaction::PageRenderInteraction;
pub(in crate::ui::board_workspace) use title::PageTitleRenderer;

mod interaction;

pub(super) use content::render_page_block_structural_content;
pub(in crate::ui::board_workspace::page) use content::{
    render_page_block_editable_content, NumberedEditableBlock, PageBlockVisualStyle,
};
pub(in crate::ui::board_workspace::page::editor) use controls::page_toggle_disclosure_focus_shadow;
pub(crate) use document::{PageDocumentColumn, PageDocumentLayout};
pub(in crate::ui::board_workspace::page::editor) use resource::{
    render_page_block_image_preview, render_unsupported_page_leaf,
};

use row::PageBlockRenderContext;

use gpui::IntoElement;
use std::{rc::Rc, sync::Arc};

use crate::ui::{
    block_images::NotionBlockImageView,
    surface::{
        NotionSurfaceResources, PageDocumentDragRuntime, PageDocumentFlowRuntime,
        PageInputResources, PageSimpleTableRuntime,
    },
    view_actions::{ViewActionSink, ViewNotifier},
    AppearanceMode, CardPage, CardPageBlock, IconSet, LoadedCardPageData, Theme, Viewport,
};

use super::code::PageCodeSyntaxRenderer;
use super::input::PageInputBindings;
use super::mention::{PageMentionClock, PageMentionMenuPresentationSeed, PageMentionRenderer};
use super::slash::PageSlashMenuRenderer;
use super::{context_menu::PageBlockContextMenuRenderer, page_link_icon::PageLinkIconPresentation};
use crate::ui::board_workspace::inline_database::InlineDatabaseBlockRenderer;
use crate::ui::board_workspace::PageShellIconRenderer;

/// Immutable frame presentation plus cloneable live resources used by every
/// virtualized page row. It deliberately has no handle to `SurfaceState`.
#[derive(Clone)]
pub(in crate::ui::board_workspace) struct PageBlockRenderer {
    pub(super) data: Arc<LoadedCardPageData>,
    pub(super) authority_page: Option<Arc<CardPage>>,
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) viewport: Viewport,
    pub(super) board_url: Option<String>,
    pub(super) resources: NotionSurfaceResources,
    pub(super) column: PageDocumentColumn,
    pub(super) interaction: Rc<PageRenderInteraction>,
    pub(super) input_resources: PageInputResources,
    pub(super) flow: PageDocumentFlowRuntime,
    pub(super) drag: PageDocumentDragRuntime,
    pub(super) tables: PageSimpleTableRuntime,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) block_images: NotionBlockImageView,
    pub(super) inline_databases: InlineDatabaseBlockRenderer,
    pub(super) context_menu: Option<PageBlockContextMenuRenderer>,
    pub(super) page_link_icons: Option<PageLinkIconPresentation>,
    pub(super) code_syntax: PageCodeSyntaxRenderer,
    pub(super) input_bindings: PageInputBindings,
    pub(super) slash_menu: Option<PageSlashMenuRenderer>,
    pub(super) mention_renderer: PageMentionRenderer,
    pub(super) mention_menu: Option<PageMentionFrame>,
    pub(super) mention_clock: PageMentionClock,
    pub(super) mention_ghost: Option<(String, gpui_components::text_input::TextInputGhost)>,
    pub(super) mention_pill_range: Option<BlockMentionPillRange>,
    pub(super) actions: ViewActionSink<PageRenderAction>,
    pub(super) notifier: ViewNotifier,
    pub(super) simple_table_key_down: gpui_components::text_input::TextInputKeyDownPreAction,
}

/// The block id showing a mention pill and the pill's text range in that block.
type BlockMentionPillRange = (String, std::ops::Range<usize>);

#[derive(Clone)]
pub(super) struct PageMentionFrame {
    pub(super) block_id: String,
    pub(super) trigger_offset: usize,
    pub(super) presentation: PageMentionMenuPresentationSeed,
}

impl PageBlockRenderer {
    pub(super) fn render_native_page_slash_menu(
        &self,
        block: &CardPageBlock,
        format: crate::model::CardPageFormat,
        cx: &mut gpui::App,
    ) -> gpui::AnyElement {
        self.slash_menu.as_ref().map_or_else(
            || gpui::div().into_any_element(),
            |menu| menu.render(block, format, cx),
        )
    }

    pub(super) fn render_page_block_context_menu(
        &self,
        block: &CardPageBlock,
        row_center: f32,
        cx: &mut gpui::App,
    ) -> gpui::Div {
        self.context_menu
            .as_ref()
            .map_or_else(gpui::div, |menu| menu.render_block(block, row_center, cx))
    }

    pub(super) fn render_native_page_mention_menu(
        &self,
        block: &CardPageBlock,
        cx: &mut gpui::App,
    ) -> gpui::AnyElement {
        let Some(frame) = self
            .mention_menu
            .as_ref()
            .filter(|frame| frame.block_id == block.block_id)
        else {
            return gpui::div().into_any_element();
        };
        let Some(input) = self
            .input_resources
            .state()
            .block_inputs
            .borrow()
            .get(&frame.block_id)
            .cloned()
        else {
            return gpui::div().into_any_element();
        };
        let input = input.read(cx);
        let end = frame.trigger_offset + '@'.len_utf8();
        if end > input.text().len() || !input.text()[frame.trigger_offset..].starts_with('@') {
            return gpui::div().into_any_element();
        }
        let Some(trigger) = input.window_bounds_for_byte_range(frame.trigger_offset..end) else {
            return gpui::div().into_any_element();
        };
        let presentation = frame.presentation.clone().with_trigger(trigger);
        self.mention_renderer.render_menu(&presentation, cx)
    }
}
