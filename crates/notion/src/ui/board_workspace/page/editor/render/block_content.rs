use super::super::input::PageBlockInputContent;
use super::super::{
    page_block_visual_color, AnyElement, App, CardPageBlockColor, CardPageBlockColorValue,
    CardPageBlockKind, CardPageEditableBlock, CardPageStructuralBlock, Div, IntoElement,
};
use super::content::{
    render_page_block_editable_content, render_page_block_structural_content,
    NumberedEditableBlock, PageBlockVisualStyle,
};
use super::row::PageBlockRenderContext;
use super::PageBlockRenderer;

impl PageBlockRenderer {
    pub(super) fn render_page_block_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        numbered_index: usize,
        cx: &mut App,
    ) -> Div {
        if let Some(editable) = context.block.editable_content() {
            return self.render_page_editable_block_content(context, editable, numbered_index, cx);
        }
        if let Some(alias) = context.block.alias_content() {
            return self.render_page_alias_block_content(context, alias, cx);
        }
        if let Some(resource) = context.block.resource_content() {
            return self.render_page_block_image(context, resource.image(), cx);
        }
        if let Some(unsupported) = context.block.unsupported_leaf_content() {
            return super::resource::render_unsupported_page_leaf(unsupported, self.theme);
        }
        let structural = context
            .block
            .structural_content()
            .expect("visible non-editable page block must be structural");
        if let CardPageStructuralBlock::CollectionView { .. } = structural {
            return self
                .inline_databases
                .render(&context.data.page.block_id, &context.block.block_id);
        }
        render_page_block_structural_content(structural, self.theme)
    }

    fn render_page_editable_block_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        numbered_index: usize,
        cx: &mut App,
    ) -> Div {
        let color = page_block_render_color(
            editable.kind,
            context.block.color,
            context.inherited_text_color,
        );
        match editable.kind {
            CardPageBlockKind::PageLink => {
                self.render_page_link_block_content(context, editable, color, cx)
            }
            CardPageBlockKind::Callout => {
                self.render_page_callout_block_content(context, editable, color, cx)
            }
            CardPageBlockKind::Code => self.render_page_code_block(context, editable, color, cx),
            _ => self.render_regular_page_editable_content(
                context,
                NumberedEditableBlock {
                    editable,
                    numbered_index,
                },
                color,
                cx,
            ),
        }
    }

    fn render_regular_page_editable_content(
        &self,
        context: &PageBlockRenderContext<'_>,
        block: NumberedEditableBlock<'_>,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> Div {
        let editable = block.editable;
        let input = self.page_block_input_entity(
            context.data,
            PageBlockInputContent {
                block_id: &context.block.block_id,
                editable,
                color,
            },
            cx,
        );
        let spacing = context
            .row_spacing
            .expect("editable page rows require precomputed spacing");
        let visual = PageBlockVisualStyle::new(self.theme, self.appearance_mode, color)
            .with_selection(
                self.interaction
                    .block_selected_without_drag(&context.block.block_id),
            );
        let marker = self.render_page_editable_block_marker(context, editable, color, cx);
        render_page_block_editable_content(block, visual, spacing, marker, input.into_any_element())
    }

    fn render_page_editable_block_marker(
        &self,
        context: &PageBlockRenderContext<'_>,
        editable: &CardPageEditableBlock,
        color: CardPageBlockColor,
        cx: &mut App,
    ) -> Option<AnyElement> {
        match editable.kind {
            CardPageBlockKind::ToggleList => Some(self.render_page_toggle_disclosure_button(
                &context.data.page.block_id,
                &context.block.block_id,
                color,
                cx,
            )),
            CardPageBlockKind::ToDoList => Some(
                self.render_page_to_do_checkbox(
                    &context.block.block_id,
                    editable
                        .to_do_state()
                        .expect("to-do block must contain typed checked state"),
                    color,
                    cx,
                ),
            ),
            CardPageBlockKind::PageLink => Some(
                super::super::render_page_link_marker(self.icons.page.render(cx))
                    .into_any_element(),
            ),
            _ => None,
        }
    }
}

fn page_block_render_color(
    kind: CardPageBlockKind,
    color: CardPageBlockColor,
    inherited_text_color: Option<CardPageBlockColorValue>,
) -> CardPageBlockColor {
    if kind == CardPageBlockKind::Code {
        return CardPageBlockColor::default();
    }
    let color = page_block_visual_color(kind, color);
    match (color, inherited_text_color) {
        (
            CardPageBlockColor::Text(CardPageBlockColorValue::Default),
            Some(inherited_text_color),
        ) => CardPageBlockColor::text(inherited_text_color),
        _ => color,
    }
}
