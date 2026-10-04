use std::rc::Rc;

use super::callbacks::{
    page_block_on_backspace, page_block_on_change, page_block_on_delete, page_block_on_submit,
};
use super::mention::{
    page_mention_input_pill_highlights, page_text_input_atoms, PageMentionAction,
};
use super::render::PageBlockRenderer;
use super::rich_text::page_text_input_highlights;
use super::support::{page_block_editable_input_spec, page_block_text_input_style};
use super::{
    page_command_placeholder, App, AppContext, CardPageBlockColor, CardPageBlockKind,
    CardPageEditableBlock, ElementId, TextInput, TextInputEnterBehavior, TextInputMode,
    TextInputProps,
};
use crate::model::CardPageFormat;
use crate::ui::PageBlockInputPropsKey;
use gpui::Hsla;
use gpui_components::text_input::{
    TextInputAtomClick, TextInputGhost, TextInputHighlight, TextInputWrapMode,
};

mod actions;
mod behavior;
mod bindings;
mod menu;
mod mounted_focus;
mod navigation_props;

pub(super) use actions::{
    PageInputAction, PageInputHostAction, PageNavigationInputAction, PageTextInputAction,
};
pub(super) use bindings::PageInputBindings;

use behavior::PageBlockInputBehavior;

#[derive(Clone, Copy)]
pub(super) struct PageBlockInputContent<'a> {
    pub(super) block_id: &'a str,
    pub(super) editable: &'a CardPageEditableBlock,
    pub(super) color: CardPageBlockColor,
}

impl PageBlockRenderer {
    pub(super) fn page_block_input_entity(
        &self,
        page_data: &std::sync::Arc<crate::ui::LoadedCardPageData>,
        content: PageBlockInputContent<'_>,
        cx: &mut App,
    ) -> gpui::Entity<TextInput> {
        let PageBlockInputContent {
            block_id, editable, ..
        } = content;
        let focus_request = (!page_data.has_column_structure())
            .then(|| self.input_resources.take_page_block_focus_request(block_id))
            .flatten();
        let props_key = self.page_block_input_props_key(
            content,
            page_data.page.format,
            focus_request.is_some(),
        );
        let input = self.page_block_input_for_props(block_id, props_key, cx);
        if let Some(request) = focus_request {
            let text = editable.text.clone();
            input.update(cx, move |input, cx| {
                if let Some(marked_range) = request.marked_range {
                    input.set_text_and_mark_range(text, marked_range, cx);
                } else {
                    input.set_text_and_move_cursor(text, request.cursor, cx);
                }
            });
        }
        if let Some((range, reversed)) = self.page_text_input_selection(page_data, block_id) {
            input.update(cx, |input, cx| input.set_selection(range, reversed, cx));
        }
        input
    }

    fn page_block_input_props_key(
        &self,
        content: PageBlockInputContent<'_>,
        format: CardPageFormat,
        request_focus: bool,
    ) -> PageBlockInputPropsKey {
        let PageBlockInputContent {
            block_id,
            editable,
            color,
        } = content;
        PageBlockInputPropsKey {
            editable: editable.clone(),
            color,
            format,
            appearance_mode: self.appearance_mode,
            slash_menu_active: self.interaction.slash_menu_is_open(block_id),
            mention_menu_active: self.interaction.mention_menu_is_open(block_id),
            request_focus,
            // Relative labels ("Today") are rebuilt when the day rolls over.
            today: self.mention_clock.today,
            // The ghost follows the selected row, which the arrow keys move
            // without touching the block text.
            mention_ghost: self
                .mention_ghost
                .as_ref()
                .filter(|(id, _)| id == block_id)
                .map(|(_, ghost)| ghost.text.to_string()),
            show_placeholder: matches!(
                editable.kind,
                CardPageBlockKind::PageLink | CardPageBlockKind::ToggleList
            ) || self.interaction.active_block_is(block_id),
        }
    }

    fn page_block_input_for_props(
        &self,
        block_id: &str,
        props_key: PageBlockInputPropsKey,
        cx: &mut App,
    ) -> gpui::Entity<TextInput> {
        let existing = self
            .input_resources
            .state()
            .block_inputs
            .borrow()
            .get(block_id)
            .cloned();
        if let Some(input) = existing {
            let props_changed = self
                .input_resources
                .state()
                .block_input_props
                .borrow()
                .get(block_id)
                != Some(&props_key);
            if props_changed {
                let props = self.page_block_input_props(block_id, &props_key, cx);
                input.update(cx, |input, cx| input.apply_props(props, cx));
                self.store_page_block_input_props(block_id, props_key);
            }
            input
        } else {
            let props = self.page_block_input_props(block_id, &props_key, cx);
            let input = cx.new(|cx| TextInput::new(props, cx));
            self.input_resources
                .state()
                .block_inputs
                .borrow_mut()
                .insert(block_id.to_string(), input.clone());
            self.store_page_block_input_props(block_id, props_key);
            input
        }
    }

    fn store_page_block_input_props(&self, block_id: &str, props: PageBlockInputPropsKey) {
        self.input_resources
            .state()
            .block_input_props
            .borrow_mut()
            .insert(block_id.to_string(), props);
    }

    fn page_block_input_props(
        &self,
        block_id: &str,
        key: &PageBlockInputPropsKey,
        cx: &mut App,
    ) -> TextInputProps {
        let block_id = block_id.to_string();
        let behavior = PageBlockInputBehavior::from_editable(&key.editable);
        let props = self.page_block_base_input_props(&block_id, key, cx);
        let props = self.page_block_navigation_input_props(props, &block_id, behavior, cx);
        self.page_block_menu_input_props(props, &block_id)
    }

    fn page_block_base_input_props(
        &self,
        block_id: &str,
        key: &PageBlockInputPropsKey,
        cx: &mut App,
    ) -> TextInputProps {
        let (editable, color) = (&key.editable, key.color);
        let spec = page_block_editable_input_spec(editable, key.format);
        let mut style = page_block_text_input_style(spec, self.theme, color, self.appearance_mode);
        if editable.kind == CardPageBlockKind::PageLink {
            style.placeholder = style.text;
        }
        let content = PageBlockInputContent {
            block_id,
            editable,
            color,
        };
        let highlights = self.page_block_input_highlights(content, style.text, cx);
        let props = TextInputProps::multiline(editable.text.clone())
            .placeholder(if key.show_placeholder {
                page_command_placeholder(editable.kind)
            } else {
                ""
            })
            .mode(TextInputMode::Multiline {
                max_visible_lines: None,
            })
            .style(style)
            .font_weight(spec.font_weight)
            .highlights(highlights)
            .atoms(page_text_input_atoms(
                editable,
                self.appearance_mode,
                self.mention_clock.today,
            ))
            .on_atom_click(page_block_on_mention_atom_click(
                block_id.to_string(),
                self.input_bindings.mention_actions.clone(),
            ))
            .ghost(self.page_block_input_ghost(block_id))
            // Titles with inline tokens notnotion cannot round-trip stay read-only so
            // a keystroke can never rewrite the token as plain text.
            .disabled(editable.read_only.is_some())
            .bordered(false)
            .request_focus(key.request_focus)
            .accessibility(
                ElementId::Name(format!("notion-page-block-{block_id}").into()),
                "Notion block",
            )
            .on_change_with_state(page_block_on_change(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ));
        self.page_block_behavior_input_props(
            props,
            block_id,
            PageBlockInputBehavior::from_editable(editable),
            cx,
        )
    }

    fn page_block_input_ghost(&self, block_id: &str) -> Option<TextInputGhost> {
        self.mention_ghost
            .as_ref()
            .filter(|(id, _)| id == block_id)
            .map(|(_, ghost)| ghost.clone())
    }

    fn page_block_input_highlights(
        &self,
        content: PageBlockInputContent<'_>,
        text_color: Hsla,
        cx: &mut App,
    ) -> Vec<TextInputHighlight> {
        let PageBlockInputContent {
            block_id,
            editable,
            color,
        } = content;
        let highlights = if editable.kind == CardPageBlockKind::Code {
            self.code_syntax.highlights(block_id, editable, cx)
        } else {
            page_text_input_highlights(editable, self.appearance_mode, color)
        };
        let Some(range) = self
            .mention_pill_range
            .as_ref()
            .filter(|(id, _)| id == block_id)
            .map(|(_, range)| range.clone())
        else {
            return highlights;
        };
        page_mention_input_pill_highlights(highlights, range, text_color, self.appearance_mode)
    }

    fn page_block_behavior_input_props(
        &self,
        props: TextInputProps,
        block_id: &str,
        behavior: PageBlockInputBehavior,
        _cx: &mut App,
    ) -> TextInputProps {
        match behavior {
            PageBlockInputBehavior::Document => props
                .wrap_mode(TextInputWrapMode::SoftWrap)
                .wrap_at_hyphens(true)
                .enter_behavior(TextInputEnterBehavior::SubmitOnEnter)
                .on_pre_mutation_action(self.input_bindings.pre_mutation(block_id))
                .on_enter_before_default(self.input_bindings.cross_selection_line_break(block_id))
                .on_submit_with_state(page_block_on_submit(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                ))
                .on_backspace_at_start(page_block_on_backspace(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                ))
                .on_delete_at_end(page_block_on_delete(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                )),
            PageBlockInputBehavior::PageLink => props
                .wrap_mode(TextInputWrapMode::SoftWrap)
                .wrap_at_hyphens(true)
                .enter_behavior(TextInputEnterBehavior::SubmitOnEnter)
                .on_pre_mutation_action(self.input_bindings.pre_mutation(block_id))
                .on_submit_with_state(page_block_on_submit(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                )),
            PageBlockInputBehavior::Code { wrap_mode } => props
                .wrap_mode(wrap_mode)
                .wrap_at_hyphens(false)
                .tab_text("  ")
                .enter_behavior(TextInputEnterBehavior::ModeDefault),
        }
    }
}

fn page_block_on_mention_atom_click(
    block_id: String,
    actions: crate::ui::view_actions::ViewActionSink<PageMentionAction>,
) -> TextInputAtomClick {
    Rc::new(move |atom_index, anchor, window, cx| {
        actions.emit(
            PageMentionAction::OpenPickerAtAtom {
                block_id: block_id.clone(),
                atom_index,
                anchor,
            },
            window,
            cx,
        );
    })
}
