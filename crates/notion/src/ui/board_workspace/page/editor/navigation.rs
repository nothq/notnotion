use gpui::{App, Context, Entity, Pixels, Window};
use gpui_components::text_input::{TextInput, TextInputSnapshot, TextInputVisualLine};

use super::editing::{PageEditEffect, PageEditSession, PageEditorEffect};
use super::selection::VisiblePageTextSelection;
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPageBlock, PageTextSelection};

#[derive(Clone, Copy)]
pub(super) enum PageBlockNavigation {
    Left,
    Right,
    Up,
    Down,
}

pub(super) struct PageBlockNavigationRequest<'a> {
    pub(super) block_id: &'a str,
    pub(super) navigation: PageBlockNavigation,
    pub(super) snapshot: TextInputSnapshot,
    pub(super) window_x: Option<Pixels>,
    pub(super) extend: bool,
}

struct PageTextSelectionExtension<'a> {
    source_block_id: &'a str,
    snapshot: &'a TextInputSnapshot,
    target: &'a CardPageBlock,
    target_offset: usize,
}

struct PreparedPageBlockNavigation {
    target: CardPageBlock,
    target_input: Entity<TextInput>,
    target_offset: usize,
}

type PageTextSelectionUpdates = Vec<(Entity<TextInput>, usize)>;

pub(super) struct PageTextSelectionCollapse {
    source_offset: usize,
    updates: PageTextSelectionUpdates,
    target: Entity<TextInput>,
    target_is_source: bool,
}

impl PageTextSelectionCollapse {
    pub(super) fn apply(
        self,
        source: &mut TextInput,
        window: &mut Window,
        cx: &mut Context<TextInput>,
    ) {
        source.set_selection(self.source_offset..self.source_offset, false, cx);
        for (input, offset) in self.updates {
            input.update(cx, |input, cx| {
                input.set_selection(offset..offset, false, cx);
            });
        }
        let target_focus = if self.target_is_source {
            source.focus_handle_clone()
        } else {
            self.target.read(cx).focus_handle_clone()
        };
        window.focus(&target_focus, cx);
    }
}

impl PageBlockNavigation {
    fn forward(self) -> bool {
        matches!(self, Self::Right | Self::Down)
    }

    fn vertical(self) -> bool {
        matches!(self, Self::Up | Self::Down)
    }

    fn target_visual_line(self) -> Option<TextInputVisualLine> {
        match self {
            Self::Up => Some(TextInputVisualLine::Last),
            Self::Down => Some(TextInputVisualLine::First),
            Self::Left | Self::Right => None,
        }
    }
}

impl PageEditSession<'_> {
    pub(super) fn collapse_cross_block_text_selection(
        &mut self,
        source_block_id: &str,
        source_selection_end: usize,
        navigation: PageBlockNavigation,
        cx: &mut App,
    ) -> Option<PageTextSelectionCollapse> {
        let selection = self.editor.page_text_selection.as_ref()?.clone();
        if selection.anchor_block_id == selection.focus_block_id
            || self.editor.active_page_block.as_deref() != Some(source_block_id)
        {
            return None;
        }
        let data = self
            .documents
            .page_data_containing_editable_block(&selection.anchor_block_id)?;
        let visible_selection = VisiblePageTextSelection::new(&data, &selection)?;
        let (block_id, offset) = page_text_selection_collapse_endpoint(
            visible_selection.anchor_index(),
            visible_selection.focus_index(),
            selection,
            navigation,
        )?;
        let inputs = self.editor.input.resource_state().block_inputs.borrow();
        let target = inputs.get(&block_id)?.clone();
        let target_is_source = block_id == source_block_id;
        let source_offset = if target_is_source {
            offset
        } else {
            source_selection_end
        };
        let updates = inputs
            .iter()
            .filter(|(candidate_id, _)| candidate_id.as_str() != source_block_id)
            .filter_map(|(candidate_id, input)| {
                data.editable_block_indices.get(candidate_id)?;
                let input_offset = if input == &target {
                    offset
                } else {
                    input.read(cx).selection_range().end
                };
                Some((input.clone(), input_offset))
            })
            .collect();
        drop(inputs);
        self.editor
            .finish_cross_block_text_selection_collapse(block_id);
        self.effects.push(PageEditEffect::Notify);
        Some(PageTextSelectionCollapse {
            source_offset,
            updates,
            target,
            target_is_source,
        })
    }

    pub(super) fn navigate_page_block_text(
        &mut self,
        request: PageBlockNavigationRequest<'_>,
        cx: &mut App,
    ) -> Option<Entity<TextInput>> {
        let prepared = self.prepare_page_block_navigation(&request, cx)?;
        let PageBlockNavigationRequest {
            block_id,
            navigation: _,
            snapshot,
            window_x,
            extend,
        } = request;
        let PreparedPageBlockNavigation {
            target,
            target_input,
            target_offset,
        } = prepared;

        if extend {
            self.extend_page_text_selection(
                PageTextSelectionExtension {
                    source_block_id: block_id,
                    snapshot: &snapshot,
                    target: &target,
                    target_offset,
                },
                cx,
            );
        } else {
            if let Some(reset) =
                self.editor
                    .clear_page_document_selection_from_input(block_id, snapshot.cursor, cx)
            {
                if let Some(input) = reset.input {
                    self.effects.push(PageEditEffect::Editor(
                        PageEditorEffect::DeferInputSelection {
                            input,
                            range: reset.cursor..reset.cursor,
                            reversed: false,
                        },
                    ));
                }
            }
            target_input.update(cx, |input, cx| {
                input.set_selection(target_offset..target_offset, false, cx);
            });
        }
        self.editor
            .finish_page_block_navigation(&target, &target_input, window_x, cx);
        self.effects.push(PageEditEffect::Notify);
        Some(target_input)
    }

    fn prepare_page_block_navigation(
        &self,
        request: &PageBlockNavigationRequest<'_>,
        cx: &App,
    ) -> Option<PreparedPageBlockNavigation> {
        if request.snapshot.is_composing {
            return None;
        }
        let data = self
            .documents
            .page_data_containing_editable_block(request.block_id)?;
        let source_index = *data.editable_block_indices.get(request.block_id)?;
        let target_index =
            adjacent_visible_editable_block_index(&data, source_index, request.navigation)?;
        let target = data.page.blocks[target_index].clone();
        let target_editable = target.editable_content()?;
        let target_input = self
            .editor
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(&target.block_id)
            .cloned()?;
        let target_offset = navigation_target_offset(
            request.navigation,
            &target_editable.text,
            &target_input,
            request.window_x,
            cx,
        )?;
        Some(PreparedPageBlockNavigation {
            target,
            target_input,
            target_offset,
        })
    }

    fn extend_page_text_selection(
        &mut self,
        extension: PageTextSelectionExtension<'_>,
        cx: &mut App,
    ) {
        let PageTextSelectionExtension {
            source_block_id,
            snapshot,
            target,
            target_offset,
        } = extension;
        let existing_selection_is_visible = self
            .editor
            .page_text_selection
            .as_ref()
            .and_then(|selection| {
                self.documents
                    .page_data_containing_editable_block(&selection.anchor_block_id)
                    .map(|data| VisiblePageTextSelection::new(&data, selection).is_some())
            })
            .unwrap_or(false);
        let (anchor_block_id, anchor_offset) = self
            .editor
            .page_text_selection
            .as_ref()
            .filter(|selection| {
                existing_selection_is_visible
                    && selection.focus_block_id == source_block_id
                    && selection.focus_offset == snapshot.cursor
            })
            .map_or_else(
                || snapshot_anchor(source_block_id, snapshot),
                |selection| (selection.anchor_block_id.clone(), selection.anchor_offset),
            );
        self.editor.page_text_selection = Some(PageTextSelection {
            anchor_block_id,
            anchor_offset,
            focus_block_id: target.block_id.clone(),
            focus_offset: target_offset,
            pointer_active: false,
        });
        self.editor.page_block_selection.block_ids.clear();
        self.editor.synchronize_page_text_input_selections(
            self.documents,
            Some(source_block_id),
            cx,
        );
        self.editor.drop_empty_page_text_selection();
    }
}

impl PageEditorState {
    fn finish_cross_block_text_selection_collapse(&mut self, block_id: String) {
        self.page_text_selection = None;
        self.page_block_selection.block_ids.clear();
        self.page_forced_text_annotations = None;
        self.page_rich_text_dialog = None;
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.active_page_block = Some(block_id);
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
    }

    fn finish_page_block_navigation(
        &mut self,
        target: &CardPageBlock,
        target_input: &Entity<TextInput>,
        window_x: Option<Pixels>,
        cx: &mut App,
    ) {
        if let Some(window_x) = window_x {
            target_input.update(cx, |input, _cx| {
                input.set_vertical_navigation_window_x(window_x);
            });
        }
        self.active_page_block = Some(target.block_id.clone());
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
    }
}

fn page_text_selection_collapse_endpoint(
    anchor_index: usize,
    focus_index: usize,
    selection: PageTextSelection,
    navigation: PageBlockNavigation,
) -> Option<(String, usize)> {
    match (navigation, anchor_index <= focus_index) {
        (PageBlockNavigation::Left, true) | (PageBlockNavigation::Right, false) => {
            Some((selection.anchor_block_id, selection.anchor_offset))
        }
        (PageBlockNavigation::Right, true) | (PageBlockNavigation::Left, false) => {
            Some((selection.focus_block_id, selection.focus_offset))
        }
        (PageBlockNavigation::Up | PageBlockNavigation::Down, _) => None,
    }
}

fn adjacent_visible_editable_block_index(
    data: &crate::ui::LoadedCardPageData,
    source_index: usize,
    navigation: PageBlockNavigation,
) -> Option<usize> {
    let source_row_index = data
        .visible_rows
        .binary_search_by_key(&source_index, |row| row.block_index)
        .ok()?;
    if navigation.forward() {
        data.visible_rows[source_row_index + 1..]
            .iter()
            .map(|row| row.block_index)
            .find(|index| data.block_has_text_input(&data.page.blocks[*index].block_id))
    } else {
        data.visible_rows[..source_row_index]
            .iter()
            .rev()
            .map(|row| row.block_index)
            .find(|index| data.block_has_text_input(&data.page.blocks[*index].block_id))
    }
}

fn navigation_target_offset(
    navigation: PageBlockNavigation,
    target_text: &str,
    target_input: &Entity<TextInput>,
    window_x: Option<Pixels>,
    cx: &App,
) -> Option<usize> {
    assert_eq!(
        navigation.vertical(),
        window_x.is_some(),
        "vertical page navigation requires a shaped window x"
    );
    let Some(window_x) = window_x else {
        return Some(if navigation.forward() {
            0
        } else {
            target_text.len()
        });
    };
    let line = navigation
        .target_visual_line()
        .expect("vertical page navigation must target an outer visual line");
    target_input
        .read(cx)
        .outer_visual_line_offset_for_window_x(line, window_x)
}

fn snapshot_anchor(block_id: &str, snapshot: &TextInputSnapshot) -> (String, usize) {
    let anchor = if snapshot.cursor == snapshot.selection.start {
        snapshot.selection.end
    } else {
        snapshot.selection.start
    };
    (block_id.to_string(), anchor)
}
