use std::sync::Arc;

use super::{
    div, px, rgb, rgba, AnyElement, AppearanceMode, CardPeekState, Context, Div, FluentBuilder,
    FontWeight, IconAsset, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Styled, SurfaceState, Theme, ToolbarDialogKind, CARD_PEEK_WIDTH,
};
use crate::ui::board_workspace::page::editor::{
    handle_page_render_action, PageDocumentLayout, PageDocumentRenderAction, PageRenderAction,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::PageBlockDragScrollTarget;
use gpui::App;

mod content;
mod intro;

use content::SelectedPageBodyRenderer;
use intro::{SelectedPageIntroRenderer, SelectedPageIntroResources};

#[derive(Clone, Copy)]
pub(crate) struct SelectedPageOverlayLayout {
    panel_width: f32,
    panel_height: f32,
    right_inset: f32,
}

impl SelectedPageOverlayLayout {
    pub(crate) fn panel_width(main_pane_width: f32) -> f32 {
        CARD_PEEK_WIDTH.min((main_pane_width - 360.0).max(0.0))
    }

    fn new(main_pane_width: f32, panel_height: f32, right_inset: f32) -> Self {
        Self {
            panel_width: Self::panel_width(main_pane_width),
            panel_height,
            right_inset,
        }
    }
}

struct SelectedPageOverlayRenderer {
    frame: SelectedPageOverlayFrame,
    header: SelectedPageHeaderRenderer,
    intro: SelectedPageIntroRenderer,
    body: SelectedPageBodyRenderer,
}

struct SelectedPageOverlayFrame {
    layout: SelectedPageOverlayLayout,
    theme: Theme,
    appearance_mode: AppearanceMode,
}

struct SelectedPageHeaderRenderer {
    theme: Theme,
    actions: ViewActionSink<PageRenderAction>,
    toolbar_actions: Div,
}

impl SurfaceState {
    pub(crate) fn render_selected_page_overlay(&self, cx: &mut Context<Self>) -> Div {
        let selected_page = self
            .page_documents
            .selected_page
            .as_ref()
            .expect("selected page overlay requires selected page state")
            .clone();
        let actions = ViewActionSink::new(cx, handle_page_render_action);
        let page_layout = self.page_layout();
        let layout = SelectedPageOverlayLayout::new(
            page_layout.main_pane_width(),
            self.viewport.app_height(),
            page_layout.consuming_ai_width(),
        );
        let frame = SelectedPageOverlayFrame::new(layout, self.theme, self.appearance_mode);
        let top_bar = self.top_bar_renderer(cx);
        let header = SelectedPageHeaderRenderer::capture(
            frame.theme,
            actions.clone(),
            &top_bar,
            &self.icons.topbar_actions,
            cx,
        );
        let intro = SelectedPageIntroRenderer::capture(
            SelectedPageIntroResources::new(
                frame.theme,
                frame.appearance_mode,
                Arc::clone(&self.icons),
                !self.board.is_locked && self.notion_startup.workspace_api().is_some(),
                self.page_editor.input.resources(),
            ),
            &selected_page,
            actions,
            cx,
        );
        let body = match &selected_page {
            CardPeekState::Loaded(page) => {
                let data = page.data.clone();
                let blocks = self.page_block_renderer(page, PageDocumentLayout::SelectedPage, cx);
                let document =
                    self.render_page_document(page, PageDocumentLayout::SelectedPage, cx);
                let drop_overlay = cx
                    .has_active_drag()
                    .then(|| {
                        self.page_editor.render_page_block_drop_overlay(
                            &data,
                            PageBlockDragScrollTarget::SelectedPage,
                        )
                    })
                    .flatten();
                SelectedPageBodyRenderer::loaded(blocks, data, document, drop_overlay)
            }
            CardPeekState::Loading { .. } | CardPeekState::Error { .. } => {
                SelectedPageBodyRenderer::empty()
            }
        };
        SelectedPageOverlayRenderer::new(frame, header, intro, body).render(cx)
    }
}

impl SelectedPageOverlayFrame {
    fn new(
        layout: SelectedPageOverlayLayout,
        theme: Theme,
        appearance_mode: AppearanceMode,
    ) -> Self {
        Self {
            layout,
            theme,
            appearance_mode,
        }
    }
}

impl SelectedPageOverlayRenderer {
    fn new(
        frame: SelectedPageOverlayFrame,
        header: SelectedPageHeaderRenderer,
        intro: SelectedPageIntroRenderer,
        body: SelectedPageBodyRenderer,
    ) -> Self {
        Self {
            frame,
            header,
            intro,
            body,
        }
    }

    fn render(self, cx: &mut App) -> Div {
        let show_action_button = self.intro.has_loaded_page();
        let background = if self.frame.appearance_mode == AppearanceMode::Light {
            0xffffff
        } else {
            0x202020
        };
        let body = div()
            .w_full()
            .h_full()
            .relative()
            .flex()
            .flex_col()
            .child(self.header.render())
            .child(self.intro.render(self.frame.theme))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(self.body.render(cx)),
            );
        div()
            .absolute()
            .right(px(self.frame.layout.right_inset))
            .top(px(0.0))
            .child(
                div()
                    .w(px(self.frame.layout.panel_width))
                    .h(px(self.frame.layout.panel_height))
                    .border_l_1()
                    .border_color(rgba(self.frame.theme.surface_border))
                    .bg(rgb(background))
                    .flex()
                    .child(body)
                    .when(show_action_button, |panel| {
                        panel.child(Self::render_action_button(self.frame.appearance_mode))
                    })
                    .into_any_element(),
            )
    }

    fn render_action_button(appearance_mode: AppearanceMode) -> Div {
        let (bg, fg) = if appearance_mode == AppearanceMode::Light {
            (0xf0efed, 0x37352f)
        } else {
            (0xd3d3d3, 0x4a4a4a)
        };
        div()
            .absolute()
            .right(px(18.0))
            .bottom(px(14.0))
            .size(px(36.0))
            .rounded_full()
            .bg(rgb(bg))
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(10.0)).rounded_full().bg(rgb(fg)))
    }
}

impl SelectedPageHeaderRenderer {
    fn capture(
        theme: Theme,
        actions: ViewActionSink<PageRenderAction>,
        top_bar: &super::top_bar::TopBarRenderer<'_>,
        actions_icon: &IconAsset,
        cx: &mut App,
    ) -> Self {
        let toolbar_actions = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(top_bar.render_top_bar_share_button(cx))
            .child(top_bar.render_top_bar_favorite_button(cx))
            .child(top_bar.render_top_bar_icon_button(
                actions_icon,
                cx,
                false,
                Some(ToolbarDialogKind::Actions),
            ));
        Self {
            theme,
            actions,
            toolbar_actions,
        }
    }

    fn render(self) -> AnyElement {
        div()
            .h(px(44.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .child(self.leading_controls())
            .child(self.toolbar_actions)
            .into_any_element()
    }

    fn leading_controls(&self) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.button("Back", Some("←"), || {
                PageRenderAction::Document(PageDocumentRenderAction::NavigateSelectedPageBack)
            }))
            .child(self.button("Close", None, || {
                PageRenderAction::Document(PageDocumentRenderAction::CloseSelectedPage)
            }))
    }

    fn button(
        &self,
        label: &'static str,
        glyph: Option<&'static str>,
        action: fn() -> PageRenderAction,
    ) -> AnyElement {
        div()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(move |_: &MouseDownEvent, _, _| action()),
            )
            .when_some(glyph, |button, glyph| {
                button.child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(self.theme.text_secondary))
                        .child(glyph),
                )
            })
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_secondary))
                    .child(label),
            )
            .into_any_element()
    }
}
