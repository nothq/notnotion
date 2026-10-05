use super::actions::handle_board_toolbar_action;
use super::{
    AiAutofillDialogState, AnyElement, BoardToolbarRenderer, Context, InlineDatabaseToolbarTarget,
    InlineToolbarDialogState, IntoElement, SurfaceState, ToolbarDialogKind,
};
use crate::ui::view_actions::ViewActionSink;

impl SurfaceState {
    pub(in crate::ui) fn board_toolbar_renderer(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &Context<Self>,
    ) -> BoardToolbarRenderer<'_> {
        let undated_count = self
            .board
            .toolbar_undated_count(self.board.active_view_kind());
        BoardToolbarRenderer {
            theme: self.theme,
            icons: &self.icons,
            database_search: &self.database_search,
            inline_toolbar_minimized: self.notion_chrome.inline_toolbar_minimized,
            undated_count,
            undated_expanded: undated_count.is_some()
                && self.date_undated_dialog_open_for_source(inline_target, cx),
            actions: ViewActionSink::new(cx, handle_board_toolbar_action),
        }
    }

    pub(crate) fn render_board_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        self.schedule_date_view_queries(cx);
        self.board_toolbar_renderer(None, cx)
            .render_board_toolbar_content(40.0, None, cx)
            .into_any_element()
    }

    pub(crate) fn render_inline_board_toolbar(
        &self,
        target: InlineDatabaseToolbarTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.schedule_date_view_queries(cx);
        self.board_toolbar_renderer(Some(&target), cx)
            .render_board_toolbar_content(0.0, Some(target), cx)
            .into_any_element()
    }
    pub(in super::super) fn open_inline_toolbar_dialog(
        &mut self,
        dialog: ToolbarDialogKind,
        parent_surface: &gpui::WeakEntity<SurfaceState>,
        cx: &mut Context<Self>,
    ) {
        if dialog == ToolbarDialogKind::Filter {
            self.database_filter.open_property_picker();
        }
        let anchor = self
            .board_view
            .inline_toolbar_dialog_anchors
            .get(dialog)
            .expect("inline database toolbar dialog requires a measured button anchor");
        let surface = cx.entity().downgrade();
        parent_surface
            .update(cx, move |parent, cx| {
                parent.toggle_inline_toolbar_dialog(dialog, anchor, surface, cx);
            })
            .expect("inline database toolbar requires its parent surface");
    }

    pub(crate) fn toggle_inline_toolbar_dialog(
        &mut self,
        dialog: ToolbarDialogKind,
        anchor: gpui::Bounds<gpui::Pixels>,
        surface: gpui::WeakEntity<SurfaceState>,
        cx: &mut Context<Self>,
    ) {
        let already_open = self
            .notion_chrome
            .inline_toolbar_dialog
            .as_ref()
            .is_some_and(|state| state.dialog == dialog);
        self.notion_chrome.inline_toolbar_dialog =
            (!already_open).then_some(InlineToolbarDialogState {
                anchor,
                dialog,
                surface,
            });
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_chrome.toolbar_dialog = None;
        self.notion_chrome.ai_autofill_dialog = None;
        self.database_search.close();
        self.notion_chrome.notion_search_open = false;
        self.notion_chrome.notion_ai_open = false;
        cx.notify();
    }

    pub(super) fn toggle_notion_ai_autofill(
        &mut self,
        parent_surface: Option<&gpui::WeakEntity<SurfaceState>>,
        cx: &mut Context<Self>,
    ) {
        let anchor = self
            .board_view
            .toolbar_bounds
            .expect("AI Autofill toolbar must be laid out before it can be clicked");
        let properties = self.board.database_properties.clone().into();
        if let Some(parent_surface) = parent_surface {
            let _ = parent_surface.update(cx, move |surface, cx| {
                surface.toggle_notion_ai_autofill_dialog(anchor, true, properties, cx);
            });
            return;
        }
        self.toggle_notion_ai_autofill_dialog(anchor, false, properties, cx);
    }

    fn toggle_notion_ai_autofill_dialog(
        &mut self,
        anchor: gpui::Bounds<gpui::Pixels>,
        inline_database: bool,
        properties: crate::ui::Arc<[crate::model::DatabaseProperty]>,
        cx: &mut Context<Self>,
    ) {
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_chrome.toolbar_dialog = None;
        self.database_search.close();
        self.notion_chrome.notion_search_open = false;
        self.notion_chrome.notion_ai_open = false;
        self.notion_chrome.ai_autofill_dialog = self
            .notion_chrome
            .ai_autofill_dialog
            .is_none()
            .then_some(AiAutofillDialogState {
                anchor,
                inline_database,
                properties,
                property_picker_open: false,
                selected_property_index: None,
                trial_open: false,
                trial_consent_checked: false,
            });
        cx.notify();
    }
}
