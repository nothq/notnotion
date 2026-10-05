use std::rc::Rc;

use gpui::{px, rgb, App, AppContext, ElementId, FontWeight, Hsla};
use gpui_components::text_input::{
    TextInput, TextInputEnterBehavior, TextInputKeyDownPreAction, TextInputProps, TextInputStyle,
    TextInputWrapMode,
};

use crate::model::{CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex};
use crate::ui::surface::{PageSimpleTableCellGeneration, PageSimpleTableRuntime};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, page_block_foreground, AppearanceMode, CardPageBlockColor};

use super::super::callbacks::{
    simple_table_on_change, simple_table_on_history, simple_table_on_layout_change,
    simple_table_on_navigation, simple_table_on_pre_mutation, simple_table_on_selection_change,
    simple_table_on_tab, simple_table_on_vertical_navigation, PageSimpleTableInputContext,
};
use super::super::render::{PageBlockRenderer, PageRenderAction, PageTableRenderAction};
use super::super::rich_text::page_simple_table_text_input_highlights;
use super::navigation::PageSimpleTableCellNavigation;
use super::PageSimpleTableShortcut;

#[derive(Clone, Copy)]
struct PageSimpleTableCellInputDescriptor<'a> {
    page_id: &'a str,
    address: &'a CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    cell: &'a crate::model::CardPageSimpleTableCell,
    color: CardPageBlockColor,
    header: bool,
    request_focus: bool,
}

/// The table cell whose input to mount, with its block color and header state.
pub(in crate::ui::board_workspace::page::editor) struct PageSimpleTableCellInputRequest<'a> {
    pub(in crate::ui::board_workspace::page::editor) page_id: &'a str,
    pub(in crate::ui::board_workspace::page::editor) address: &'a CardPageSimpleTableCellAddress,
    pub(in crate::ui::board_workspace::page::editor) color: CardPageBlockColor,
    pub(in crate::ui::board_workspace::page::editor) header: bool,
}

impl PageBlockRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn page_simple_table_cell_input_entity(
        &self,
        request: PageSimpleTableCellInputRequest<'_>,
        cx: &mut App,
    ) -> Option<gpui::Entity<TextInput>> {
        let PageSimpleTableCellInputRequest {
            page_id,
            address,
            color,
            header,
        } = request;
        if self.data.page.block_id != page_id {
            return None;
        }
        let recursive_flow = self.data.has_column_structure();
        let (generation, existing, request_focus, composition_cell, retain_local) = {
            let active = self.tables.editor().borrow();
            let active = active
                .as_ref()
                .filter(|active| active.page_id == page_id && active.address == *address)?;
            (
                active.generation,
                active.input.clone(),
                active.pending_focus.is_some() && !recursive_flow,
                active.optimistic_composition_cell.clone(),
                active.composition_active || active.composition_dirty,
            )
        };
        let rendered_cell =
            self.page_simple_table_rendered_cell(address, retain_local, composition_cell)?;
        let descriptor = PageSimpleTableCellInputDescriptor {
            page_id,
            address,
            generation,
            cell: &rendered_cell,
            color,
            header,
            request_focus,
        };
        let props = self.page_simple_table_cell_input_props(descriptor);
        let input = if let Some(input) = existing {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            input
        } else {
            cx.new(|cx| TextInput::new(props, cx))
        };
        let mut active = self.tables.editor().borrow_mut();
        let active = active
            .as_mut()
            .filter(|active| active.address == *address && active.generation == generation)?;
        active.input = Some(input.clone());
        Some(input)
    }

    fn page_simple_table_rendered_cell(
        &self,
        address: &CardPageSimpleTableCellAddress,
        retain_local: bool,
        composition_cell: Option<crate::model::CardPageWritableSimpleTableCell>,
    ) -> Option<crate::model::CardPageSimpleTableCell> {
        if retain_local {
            return composition_cell.map(|cell| cell.into_cell());
        }
        let index = CardPageSimpleTableCellIndex::new(&self.data.page).ok()?;
        let cell = index.cell(&self.data.page, address).ok()?.clone();
        crate::model::CardPageWritableSimpleTableCell::try_from(cell.clone()).ok()?;
        let authority = self.authority_page.as_ref()?;
        let authority_index = CardPageSimpleTableCellIndex::new(authority).ok()?;
        crate::model::CardPageWritableSimpleTableCell::try_from(
            authority_index.cell(authority, address).ok()?.clone(),
        )
        .ok()?;
        Some(cell)
    }

    fn page_simple_table_cell_input_props(
        &self,
        descriptor: PageSimpleTableCellInputDescriptor<'_>,
    ) -> TextInputProps {
        let PageSimpleTableCellInputDescriptor {
            page_id,
            address,
            generation,
            ..
        } = descriptor;
        let input_context = PageSimpleTableInputContext::new(
            self.data.clone(),
            self.tables.clone(),
            self.actions.clone(),
        );
        let props = self
            .page_simple_table_cell_base_input_props(descriptor)
            .on_key_down_before_default(self.simple_table_key_down.clone())
            .on_change_with_state(simple_table_on_change(
                page_id.to_owned(),
                address.clone(),
                generation,
                self.actions.clone(),
            ))
            .on_pre_mutation_action(simple_table_on_pre_mutation(
                address.clone(),
                generation,
                self.tables.clone(),
                self.notifier.clone(),
            ))
            .on_selection_change(simple_table_on_selection_change(
                address.clone(),
                generation,
                self.actions.clone(),
            ))
            .on_submit_with_state(simple_table_on_navigation(
                input_context.clone(),
                address.clone(),
                generation,
                PageSimpleTableCellNavigation::Down,
            ))
            .on_tab_with_state(simple_table_on_tab(
                input_context.clone(),
                address.clone(),
                generation,
            ))
            .on_left_at_start(simple_table_on_navigation(
                input_context.clone(),
                address.clone(),
                generation,
                PageSimpleTableCellNavigation::Left,
            ))
            .on_right_at_end(simple_table_on_navigation(
                input_context.clone(),
                address.clone(),
                generation,
                PageSimpleTableCellNavigation::Right,
            ));
        simple_table_vertical_and_history_props(props, input_context, address.clone(), generation)
    }

    fn page_simple_table_cell_base_input_props(
        &self,
        descriptor: PageSimpleTableCellInputDescriptor<'_>,
    ) -> TextInputProps {
        TextInputProps::multiline(descriptor.cell.text().to_owned())
            .style(simple_table_cell_input_style(
                self.theme,
                descriptor.color,
                self.appearance_mode,
            ))
            .font_weight(if descriptor.header {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .highlights(page_simple_table_text_input_highlights(
                descriptor.cell,
                self.appearance_mode,
                descriptor.color,
            ))
            .fill_width(true)
            .bordered(false)
            .wrap_mode(TextInputWrapMode::SoftWrap)
            .wrap_at_hyphens(true)
            .enter_behavior(TextInputEnterBehavior::SubmitOnEnter)
            .request_focus(descriptor.request_focus)
            .accessibility(
                simple_table_cell_accessibility_id(descriptor.address),
                "Notion table cell",
            )
    }
}

fn simple_table_cell_accessibility_id(address: &CardPageSimpleTableCellAddress) -> ElementId {
    ElementId::Name(
        format!(
            "notion-simple-table-cell-input-{}-{}-{}",
            address.table_block_id(),
            address.row_block_id(),
            address.column_id().as_str()
        )
        .into(),
    )
}

fn simple_table_vertical_and_history_props(
    props: TextInputProps,
    context: PageSimpleTableInputContext,
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
) -> TextInputProps {
    props
        .on_up_at_first_visual_line(simple_table_on_vertical_navigation(
            context.clone(),
            address.clone(),
            generation,
            PageSimpleTableCellNavigation::Up,
        ))
        .on_down_at_last_visual_line(simple_table_on_vertical_navigation(
            context.clone(),
            address.clone(),
            generation,
            PageSimpleTableCellNavigation::Down,
        ))
        .on_undo_with_state(simple_table_on_history(
            context.clone(),
            address.clone(),
            generation,
            false,
        ))
        .on_redo_with_state(simple_table_on_history(
            context.clone(),
            address.clone(),
            generation,
            true,
        ))
        .on_layout_change(simple_table_on_layout_change(
            context.page_id().to_string(),
            address,
            context.actions(),
        ))
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_on_key_down(
    tables: PageSimpleTableRuntime,
    actions: ViewActionSink<PageRenderAction>,
) -> TextInputKeyDownPreAction {
    Rc::new(move |_, event, window, cx| {
        let Some(shortcut) = PageSimpleTableShortcut::capture(&tables, event, window, cx) else {
            return false;
        };
        if let PageSimpleTableShortcut::Format(action) = shortcut {
            actions.emit(
                PageRenderAction::Table(PageTableRenderAction::Edit(
                    super::PageSimpleTableEditAction::Format(*action),
                )),
                window,
                cx,
            );
        }
        true
    })
}

fn simple_table_cell_input_style(
    theme: crate::ui::Theme,
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> TextInputStyle {
    let transparent = Hsla::from(rgb(0x000000)).opacity(0.0);
    TextInputStyle {
        height: px(35.0),
        min_height: px(35.0),
        padding_x: px(9.0),
        padding_y: px(7.0),
        radius: px(0.0),
        background: transparent,
        border: transparent,
        focused_border: transparent,
        text: page_block_foreground(color, appearance_mode),
        placeholder: rgb(theme.text_hint).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(14.0),
        line_height: px(20.0),
        font_family: None,
    }
}
