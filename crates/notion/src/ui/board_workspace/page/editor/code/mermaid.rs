use std::{cell::RefCell, collections::HashMap, sync::Arc};

use gpui::{div, img, px, AnyElement, App, IntoElement, ParentElement, RenderImage, Styled};

use super::super::render::PageBlockRenderer;
use crate::model::CardPageEditableBlock;
use crate::ui::Theme;

const PAGE_MERMAID_MAX_WIDTH: f32 = 640.0;

/// Each mermaid code block's last drawn source and diagram, by block id.
pub(crate) type PageMermaidDiagrams =
    RefCell<HashMap<String, (String, Option<PageMermaidDiagram>)>>;

/// A mermaid code block drawn as its diagram, at the diagram's own size.
#[derive(Clone)]
pub(crate) struct PageMermaidDiagram {
    pub(crate) image: Arc<RenderImage>,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl PageBlockRenderer {
    /// A code block's editor, with its diagram below when the block is
    /// `mermaid`. The diagram is redrawn only when the source changes.
    pub(in crate::ui::board_workspace::page::editor) fn with_mermaid_diagram(
        &self,
        input: impl IntoElement,
        block_id: &str,
        editable: &CardPageEditableBlock,
        cx: &App,
    ) -> AnyElement {
        let is_mermaid = editable.code_language().is_some_and(|language| {
            let language = language.as_str().split_whitespace().next();
            language.is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
        });
        if !is_mermaid {
            return input.into_any_element();
        }
        let state = self.input_resources.state();
        let mut diagrams = state.mermaid_diagrams.borrow_mut();
        let diagram = match diagrams.get(block_id) {
            Some((source, diagram)) if *source == editable.text => diagram.clone(),
            _ => {
                let diagram = render_mermaid_diagram(&editable.text, self.theme, cx);
                diagrams.insert(
                    block_id.to_string(),
                    (editable.text.clone(), diagram.clone()),
                );
                diagram
            }
        };
        let Some(diagram) = diagram else {
            return input.into_any_element();
        };
        let width = diagram.width.min(PAGE_MERMAID_MAX_WIDTH);
        let height = diagram.height * width / diagram.width;
        div()
            .flex()
            .flex_col()
            .child(input)
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .pb(px(12.0))
                    .child(img(diagram.image).w(px(width)).h(px(height))),
            )
            .into_any_element()
    }
}

/// Draws mermaid source in the page's colors. Source that does not parse, as
/// while someone is still typing it, has no diagram and shows its code alone.
fn render_mermaid_diagram(source: &str, theme: Theme, cx: &App) -> Option<PageMermaidDiagram> {
    let svg = mermaid_rs_renderer::render_with_options(
        source,
        mermaid_rs_renderer::RenderOptions {
            theme: mermaid_theme(theme),
            ..Default::default()
        },
    )
    .ok()?;
    let view_box = svg.split_once("viewBox=\"")?.1.split_once('"')?.0;
    let mut values = view_box.split_ascii_whitespace().skip(2);
    let width = values.next()?.parse::<f32>().ok()?;
    let height = values.next()?.parse::<f32>().ok()?;
    let image = cx
        .svg_renderer()
        .render_single_frame(svg.as_bytes(), 1.0)
        .ok()?;
    Some(PageMermaidDiagram {
        image,
        width,
        height,
    })
}

fn mermaid_theme(theme: Theme) -> mermaid_rs_renderer::Theme {
    let hex = |color: u32| format!("#{color:06x}");
    let background = hex(theme.app_bg);
    let surface = hex(theme.card_fill);
    let border = hex(theme.surface_border.hex);
    let text = hex(theme.text_primary);
    let mut mermaid = mermaid_rs_renderer::Theme::modern();
    mermaid.background = background.clone();
    mermaid.primary_color = hex(theme.elevated_surface_bg);
    mermaid.secondary_color = surface.clone();
    mermaid.tertiary_color = surface.clone();
    mermaid.primary_text_color = text.clone();
    mermaid.text_color = text.clone();
    mermaid.line_color = hex(theme.text_secondary);
    mermaid.primary_border_color = border.clone();
    mermaid.edge_label_background = background;
    mermaid.cluster_background = surface.clone();
    mermaid.cluster_border = border.clone();
    mermaid.sequence_actor_fill = hex(theme.elevated_surface_bg);
    mermaid.sequence_actor_border = border.clone();
    mermaid.sequence_actor_line = hex(theme.text_muted);
    mermaid.sequence_note_fill = surface.clone();
    mermaid.sequence_note_border = border.clone();
    mermaid.sequence_activation_fill = surface;
    mermaid.sequence_activation_border = border.clone();
    mermaid.pie_title_text_color = text.clone();
    mermaid.pie_section_text_color = text;
    mermaid.pie_legend_text_color = hex(theme.text_secondary);
    mermaid.pie_stroke_color = border.clone();
    mermaid.pie_outer_stroke_color = border;
    mermaid
}
