use std::sync::Arc;

use gpui::{anchored, deferred, Anchor, App, ClickEvent};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::{
    command_menu_empty_state, command_menu_option_body, command_menu_panel_shell,
    command_menu_row_shell, command_menu_section_title, div, px, AnyElement, CardPageBlock,
    ElementId, FluentBuilder, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    PageCommand, ParentElement, StatefulInteractiveElement, Styled, Theme,
};
use super::support::{clamped_slash_command_index, filtered_page_slash_commands};

use super::host::PageSlashAction;
use crate::ui::surface::PageInputResources;
use crate::ui::{view_actions::ViewActionSink, IconSet, PageSlashMenuState, Viewport};

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageSlashMenuRenderer {
    pub(super) menu: PageSlashMenuState,
    pub(super) theme: Theme,
    pub(super) icons: Arc<IconSet>,
    pub(super) viewport: Viewport,
    pub(super) input_resources: PageInputResources,
    pub(super) actions: ViewActionSink<PageSlashAction>,
}

const PAGE_NATIVE_SLASH_MENU_MAX_HEIGHT: f32 = 430.0;
const PAGE_NATIVE_SLASH_MENU_CHROME_HEIGHT: f32 = 36.0;
const PAGE_NATIVE_SLASH_MENU_ROW_HEIGHT: f32 = 28.0;
const PAGE_NATIVE_SLASH_MENU_GAP: f32 = 3.0;
const PAGE_NATIVE_SLASH_MENU_VIEWPORT_MARGIN: f32 = 8.0;

struct NativePageSlashCommandRow<'a> {
    block_id: &'a str,
    index: usize,
    command: &'a PageCommand,
    selected: bool,
}

impl PageSlashMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn render(
        &self,
        block: &CardPageBlock,
        format: crate::model::CardPageFormat,
        cx: &mut App,
    ) -> AnyElement {
        if self.menu.block_id != block.block_id {
            return div().into_any_element();
        }
        let menu = &self.menu;
        let Some(editable) = block.editable_content() else {
            return div().into_any_element();
        };
        let commands = filtered_page_slash_commands(&menu.query);
        let selected_index =
            clamped_slash_command_index(menu.selected_command_index, commands.len());
        let block_id = block.block_id.clone();
        let menu_height = (PAGE_NATIVE_SLASH_MENU_CHROME_HEIGHT
            + PAGE_NATIVE_SLASH_MENU_ROW_HEIGHT * commands.len().max(1) as f32)
            .min(PAGE_NATIVE_SLASH_MENU_MAX_HEIGHT);
        let open_upward =
            self.native_page_slash_menu_opens_upward(&block.block_id, menu_height, cx);
        let panel =
            self.render_native_page_slash_menu_panel(&block_id, &commands, selected_index, cx);
        let spec = super::super::support::page_block_editable_input_spec(editable, format);
        let anchor = if open_upward {
            Anchor::BottomLeft
        } else {
            Anchor::TopLeft
        };
        let anchor_y = if open_upward {
            -PAGE_NATIVE_SLASH_MENU_GAP
        } else {
            spec.top_padding
                + spec.line_height
                + spec.input_padding_y * 2.0
                + PAGE_NATIVE_SLASH_MENU_GAP
        };
        div()
            .absolute()
            .left(px(0.0))
            .top(px(anchor_y))
            .size_0()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(deferred(anchored().anchor(anchor).child(panel)).with_priority(100))
            .into_any_element()
    }

    fn native_page_slash_menu_opens_upward(
        &self,
        block_id: &str,
        menu_height: f32,
        cx: &mut App,
    ) -> bool {
        self.input_resources
            .state()
            .block_inputs
            .borrow()
            .get(block_id)
            .and_then(|input| input.read(cx).last_window_bounds())
            .is_some_and(|bounds| {
                let below = (self.viewport.app_height() - bounds.bottom().as_f32()).max(0.0);
                let above = (bounds.top().as_f32() - self.viewport.chrome_top_inset()).max(0.0);
                below
                    < menu_height
                        + PAGE_NATIVE_SLASH_MENU_GAP
                        + PAGE_NATIVE_SLASH_MENU_VIEWPORT_MARGIN
                    && above > below
            })
    }

    fn render_native_page_slash_menu_panel(
        &self,
        block_id: &str,
        commands: &[PageCommand],
        selected_index: usize,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        let panel = command_menu_panel_shell(self.theme)
            .id(ElementId::Name(
                format!("notion-native-slash-menu-{block_id}").into(),
            ))
            .max_h(px(430.0))
            .overflow_y_scroll()
            .child(command_menu_section_title(self.theme, "Basic blocks"))
            .when(commands.is_empty(), |panel| {
                panel.child(command_menu_empty_state(self.theme, "No matching commands"))
            })
            .children(commands.iter().enumerate().map(|(index, command)| {
                self.render_native_page_slash_command_row(
                    NativePageSlashCommandRow {
                        block_id,
                        index,
                        command,
                        selected: index == selected_index,
                    },
                    cx,
                )
            }));
        let actions = self.actions.clone();
        ClickAwayBoundary::new().dismissible_with_handler(
            div().child(panel),
            move |_, window, cx| actions.emit(PageSlashAction::Dismiss, window, cx),
            cx,
        )
    }

    fn render_native_page_slash_command_row(
        &self,
        row_spec: NativePageSlashCommandRow<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let block_id_for_click = row_spec.block_id.to_string();
        let block_id_for_hover = row_spec.block_id.to_string();
        let target = row_spec.command.target;
        let index = row_spec.index;
        let mut row = command_menu_row_shell(self.theme, row_spec.selected)
            .id(ElementId::Name(
                format!("notion-native-slash-{}-{index}", row_spec.block_id).into(),
            ))
            .on_click(self.actions.listener(move |_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                PageSlashAction::Apply {
                    block_id: block_id_for_click.clone(),
                    target,
                }
            }))
            .child(command_menu_option_body(
                self.theme,
                super::super::super::commands::render::render_page_command_icon(
                    self.theme,
                    &self.icons,
                    row_spec.command,
                    cx,
                ),
                row_spec.command.label,
                row_spec.command.shortcut,
            ));
        let actions = self.actions.clone();
        row.interactivity()
            .on_hover(move |hovered: &bool, window, cx| {
                if *hovered {
                    actions.emit(
                        PageSlashAction::Hover {
                            block_id: block_id_for_hover.clone(),
                            index,
                        },
                        window,
                        cx,
                    );
                }
            });
        row.into_any_element()
    }
}
