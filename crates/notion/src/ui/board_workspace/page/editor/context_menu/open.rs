use super::super::{
    Context, SurfaceState, Theme, Window, PAGE_CODE_LANGUAGES, PAGE_CODE_LANGUAGE_OPTION_STRIDE,
};
use super::actions::handle_page_block_context_menu_action;
use super::search::{
    new_page_block_context_menu_search_input, new_page_code_language_search_input,
};
use super::state::{PageBlockMenuFocusTarget, PageBlockMenuSelection, PageCodeLanguageSelection};
use super::{
    PageBlockContextMenuPanel, PageBlockContextMenuPresentation, PageBlockContextMenuState,
};
use crate::ui::surface::PageEditorState;
use crate::ui::view_actions::ViewActionSink;

struct PageBlockContextMenuOpenSpec {
    block_id: String,
    presentation: PageBlockContextMenuPresentation,
    panel: PageBlockContextMenuPanel,
    selection: PageBlockMenuSelection,
}

impl PageBlockContextMenuOpenSpec {
    fn block_actions(block_id: String) -> Self {
        Self {
            block_id,
            presentation: PageBlockContextMenuPresentation::BlockActions,
            panel: PageBlockContextMenuPanel::Root,
            selection: PageBlockMenuSelection::Hidden,
        }
    }

    fn code_language(block_id: String, selection: PageCodeLanguageSelection) -> Self {
        Self {
            block_id,
            presentation: PageBlockContextMenuPresentation::CodeLanguagePicker,
            panel: PageBlockContextMenuPanel::CodeLanguage,
            selection: selection.menu_selection(),
        }
    }
}

impl SurfaceState {
    pub(in crate::ui::board_workspace::page::editor) fn toggle_page_block_context_menu(
        &mut self,
        block_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page_editor.close_matching_page_block_menu(
            &block_id,
            PageBlockContextMenuPresentation::BlockActions,
        ) {
            cx.notify();
            return;
        }
        self.page_editor
            .select_page_block_context_target(&block_id, cx);
        let menu = new_page_block_context_menu_state(
            PageBlockContextMenuOpenSpec::block_actions(block_id),
            self.theme,
            ViewActionSink::new(cx, handle_page_block_context_menu_action),
            cx,
        );
        window.focus(&menu.focus_handle, cx);
        self.page_editor.page_block_context_menu = Some(menu);
        cx.notify();
    }

    pub(in crate::ui) fn toggle_page_block_context_menu_from_keyboard(
        &mut self,
        block_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.page_editor.close_matching_page_block_menu(
            &block_id,
            PageBlockContextMenuPresentation::BlockActions,
        ) {
            cx.notify();
            return;
        }
        self.page_editor
            .select_page_block_context_target(&block_id, cx);
        self.page_editor.page_block_context_menu = Some(new_page_block_context_menu_state(
            PageBlockContextMenuOpenSpec::block_actions(block_id),
            self.theme,
            ViewActionSink::new(cx, handle_page_block_context_menu_action),
            cx,
        ));
        cx.notify();
    }

    pub(in crate::ui::board_workspace::page::editor) fn toggle_page_code_language_picker(
        &mut self,
        block_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page_editor.close_matching_page_block_menu(
            &block_id,
            PageBlockContextMenuPresentation::CodeLanguagePicker,
        ) {
            cx.notify();
            return;
        }
        let Some(selection) = self
            .page_documents
            .page_containing_block(&block_id)
            .and_then(|page| {
                page.blocks
                    .into_iter()
                    .find(|block| block.block_id == block_id)
            })
            .and_then(|block| block.editable_content()?.code_language().cloned())
            .map(|language| PageCodeLanguageSelection::from_language(language.as_str()))
        else {
            return;
        };
        self.page_editor.clear_page_document_selection(cx);
        self.page_editor
            .page_block_selection
            .block_ids
            .push(block_id.clone());
        let menu = new_page_block_context_menu_state(
            PageBlockContextMenuOpenSpec::code_language(block_id, selection),
            self.theme,
            ViewActionSink::new(cx, handle_page_block_context_menu_action),
            cx,
        );
        window.focus(&menu.focus_handle, cx);
        self.page_editor.page_block_context_menu = Some(menu);
        cx.notify();
    }
}

impl PageEditorState {
    fn select_page_block_context_target(&mut self, block_id: &str, cx: &mut Context<SurfaceState>) {
        if self
            .page_block_selection
            .block_ids
            .iter()
            .any(|selected_id| selected_id == block_id)
        {
            return;
        }
        self.clear_page_document_selection(cx);
        self.page_block_selection
            .block_ids
            .push(block_id.to_string());
    }

    fn close_matching_page_block_menu(
        &mut self,
        block_id: &str,
        presentation: PageBlockContextMenuPresentation,
    ) -> bool {
        let is_matching = self
            .page_block_context_menu
            .as_ref()
            .is_some_and(|menu| menu.matches(block_id, presentation));
        if is_matching {
            self.page_block_context_menu = None;
        }
        is_matching
    }
}

fn new_page_block_context_menu_state(
    spec: PageBlockContextMenuOpenSpec,
    theme: Theme,
    actions: ViewActionSink<super::actions::PageBlockContextMenuAction>,
    cx: &mut Context<SurfaceState>,
) -> PageBlockContextMenuState {
    let list_state = gpui::ListState::new(
        PAGE_CODE_LANGUAGES.len(),
        gpui::ListAlignment::Top,
        gpui::px(PAGE_CODE_LANGUAGE_OPTION_STRIDE),
    );
    if let (PageBlockContextMenuPanel::CodeLanguage, Some(selected_index)) =
        (spec.panel, spec.selection.index())
    {
        list_state.scroll_to(gpui::ListOffset {
            item_ix: selected_index.saturating_sub(10),
            offset_in_item: gpui::px(0.0),
        });
    }
    let state = PageBlockContextMenuState {
        block_id: spec.block_id,
        presentation: spec.presentation,
        panel: spec.panel,
        query: String::new(),
        selection: spec.selection,
        focus_handle: cx.focus_handle().tab_stop(true),
        search_input: new_page_block_context_menu_search_input(theme, actions.clone(), cx),
        code_language_query: String::new(),
        code_language_search_input: new_page_code_language_search_input(theme, actions, cx),
        code_language_list_state: list_state,
        code_language_indices: (0..PAGE_CODE_LANGUAGES.len()).collect::<Vec<_>>().into(),
        focus_target: PageBlockMenuFocusTarget::for_panel(spec.panel),
    };
    state.request_active_input_focus(cx);
    state
}
