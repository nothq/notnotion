use super::{PageShellBodyRenderer, PageShellBodySnapshot};
use crate::ui::board_workspace::page::editor::PageDocumentLayout;
use crate::ui::{
    div, px, view_actions::ViewActionSink, AnyElement, FluentBuilder, InteractiveElement,
    IntoElement, LoadedCardPage, PageBlockDragScrollTarget, ParentElement, Styled, SurfaceState,
};
use gpui::{Context, ScrollWheelEvent, Window};

pub(super) enum PageShellBodyAction {
    TopControlsVisible(bool),
    Comments(String),
    OpenWorkspace { board_url: String, label: String },
}

impl SurfaceState {
    pub(crate) fn page_shell_body_renderer(&self, cx: &Context<Self>) -> PageShellBodyRenderer {
        PageShellBodyRenderer {
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            icons: self.icons.clone(),
            page_icons: self.page_shell_icon_renderer(cx),
            snapshot: PageShellBodySnapshot {
                title: self.board.page_title.clone(),
                header_icon: self
                    .board
                    .page_shell
                    .as_ref()
                    .filter(|shell| shell.page_icon_is_explicit)
                    .map(|shell| shell.page_icon.clone()),
                top_controls_visible: self.notion_chrome.notion_page_top_controls_visible,
                comment_page: self
                    .page_documents
                    .standalone
                    .as_ref()
                    .map(|page| page.data.page.block_id.clone()),
            },
            actions: ViewActionSink::new(cx, handle_page_shell_body_action),
        }
    }

    pub(crate) fn render_notion_standalone_page_content(
        &self,
        page: &LoadedCardPage,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let data = page.data.clone();
        let renderer = self.page_block_renderer(page, PageDocumentLayout::Standalone, cx);
        let container = renderer
            .observe_page_block_drag_container_bounds(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .relative()
                    .flex()
                    .flex_col(),
                PageBlockDragScrollTarget::Standalone,
            )
            .id("notion-page-scroll")
            .debug_selector(|| "notion-page-scroll".to_string())
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, _: &ScrollWheelEvent, _, cx| {
                this.start_notion_page_hydration(cx)
            }));
        (renderer
            .wire_page_block_drag_container(
                container,
                data.clone(),
                PageBlockDragScrollTarget::Standalone,
                cx,
            )
            .child(div().absolute().inset_0())
            .child(self.render_page_document(page, PageDocumentLayout::Standalone, cx))
            .when_some(
                cx.has_active_drag()
                    .then(|| {
                        self.page_editor.render_page_block_drop_overlay(
                            &data,
                            PageBlockDragScrollTarget::Standalone,
                        )
                    })
                    .flatten(),
                |page, overlay| page.child(overlay),
            ))
        .into_any_element()
    }
}

fn handle_page_shell_body_action(
    surface: &mut SurfaceState,
    action: PageShellBodyAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageShellBodyAction::TopControlsVisible(visible) => surface
            .notion_chrome
            .set_notion_page_top_controls_visible(visible, cx),
        PageShellBodyAction::Comments(page_id) => {
            surface
                .notion_chrome
                .set_notion_page_top_controls_visible(true, cx);
            surface.open_notion_page_comments(page_id.clone(), page_id, cx);
        }
        PageShellBodyAction::OpenWorkspace { board_url, label } => {
            surface.open_notion_workspace(board_url, label, cx)
        }
    }
}
