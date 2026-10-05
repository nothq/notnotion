use std::sync::Arc;

use gpui::Role;

use crate::ui::board_workspace::page::editor::{
    PageDocumentRenderAction, PageRenderAction, PageTitleRenderer,
};
use crate::ui::board_workspace::page::properties::PagePropertiesRenderer;
use crate::ui::surface::PageInputResources;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{
    alpha, div, px, relative, rgb, rgba, AnyElement, AppearanceMode, CardPage, CardPeekState, Div,
    FluentBuilder, FontWeight, IconSet, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, SurfaceState, Theme,
};

pub(super) struct SelectedPageIntroResources {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    editing_available: bool,
    inputs: PageInputResources,
}

pub(super) struct SelectedPageIntroRenderer {
    page_title: String,
    page_title_display: String,
    loaded_page: Option<CardPage>,
    editable_title: Option<AnyElement>,
    properties: Option<Div>,
    actions: ViewActionSink<PageRenderAction>,
}

impl SelectedPageIntroResources {
    pub(super) fn new(
        theme: Theme,
        appearance_mode: AppearanceMode,
        icons: Arc<IconSet>,
        editing_available: bool,
        inputs: PageInputResources,
    ) -> Self {
        Self {
            theme,
            appearance_mode,
            icons,
            editing_available,
            inputs,
        }
    }
}

impl SelectedPageIntroRenderer {
    pub(super) fn capture(
        resources: SelectedPageIntroResources,
        selected_page: &CardPeekState,
        actions: ViewActionSink<PageRenderAction>,
        cx: &mut gpui::Context<SurfaceState>,
    ) -> Self {
        let page_title = selected_page.title().to_string();
        let page_title_display = if page_title.trim().is_empty() {
            "New page".to_string()
        } else {
            page_title.clone()
        };
        let loaded_page = match selected_page {
            CardPeekState::Loaded(page) => Some(page.data.page.clone()),
            CardPeekState::Loading { .. } | CardPeekState::Error { .. } => None,
        };
        let editable_title = loaded_page.as_ref().map(|page| {
            PageTitleRenderer::new(
                resources.theme,
                resources.inputs.clone(),
                page.block_id.clone(),
                cx,
            )
            .render(page, 32.0, cx)
        });
        let properties = loaded_page
            .as_ref()
            .filter(|page| !page.properties.is_empty())
            .map(|page| {
                PagePropertiesRenderer::new(
                    resources.theme,
                    resources.appearance_mode,
                    Arc::clone(&resources.icons),
                    resources.editing_available,
                    actions.clone(),
                )
                .render(page, cx)
            });
        Self {
            page_title,
            page_title_display,
            loaded_page,
            editable_title,
            properties,
            actions,
        }
    }

    pub(super) fn has_loaded_page(&self) -> bool {
        self.loaded_page.is_some()
    }

    pub(super) fn render(self, theme: Theme) -> Div {
        let title = self.editable_title.unwrap_or_else(|| {
            Self::render_title(theme, &self.page_title, self.page_title_display)
        });
        let mut intro = div()
            .pl(px(56.0))
            .pr(px(64.0))
            .pt(px(32.0))
            .pb(px(16.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(title)
            .when_some(self.properties, |intro, properties| intro.child(properties));
        if let Some(page) = self.loaded_page {
            intro = intro
                .child(Self::render_comments(theme, &page, &self.actions))
                .child(div().h(px(1.0)).bg(rgba(theme.surface_border)));
        }
        intro
    }

    fn render_title(theme: Theme, page_title: &str, display: String) -> AnyElement {
        div()
            .text_size(px(32.0))
            .line_height(relative(1.2))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(if page_title.trim().is_empty() {
                theme.text_muted
            } else {
                theme.text_primary
            }))
            .child(display)
            .into_any_element()
    }

    fn render_comments(
        theme: Theme,
        page: &CardPage,
        actions: &ViewActionSink<PageRenderAction>,
    ) -> AnyElement {
        let page_id = page.block_id.clone();
        let summary = Self::comment_summary(page);
        div()
            .id(format!("notion-page-comments-section-{}", page.block_id))
            .role(Role::Button)
            .aria_label("Open page comments")
            .pb(px(4.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    PageRenderAction::Document(PageDocumentRenderAction::OpenComments(
                        page_id.clone(),
                    ))
                }),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(theme.text_secondary))
                    .child("Comments"),
            )
            .child(
                div()
                    .h(px(28.0))
                    .px(px(4.0))
                    .rounded(px(5.0))
                    .hover(|style| style.bg(alpha(theme.text_primary, 0.06)))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(20.0))
                            .rounded_full()
                            .bg(rgba(theme.page_icon_bg)),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(theme.text_muted))
                            .child(summary),
                    ),
            )
            .into_any_element()
    }

    fn comment_summary(page: &CardPage) -> String {
        let count = page
            .discussions
            .iter()
            .filter(|discussion| discussion.target_block_id.as_str() == page.block_id)
            .map(|discussion| discussion.comments.len())
            .sum::<usize>();
        match count {
            0 if page.comments_writable => "Add a comment…".to_string(),
            0 => "View comments".to_string(),
            1 => "1 comment".to_string(),
            _ => format!("{count} comments"),
        }
    }
}
