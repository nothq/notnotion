use std::sync::Arc;

use super::super::super::{
    alpha, command_menu_close_row_body, command_menu_divider, command_menu_empty_state,
    command_menu_option_body, command_menu_panel_shell, command_menu_row_shell,
    command_menu_section_title, div, img, px, render_page_command_bulleted_list_icon,
    render_page_command_callout_icon, render_page_command_glyph_icon,
    render_page_command_numbered_list_icon, render_page_command_todo_list_icon,
    render_page_command_toggle_list_icon, rgba, AnyElement, CardPageBlockKind, Div, Element,
    FluentBuilder, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, PageCommand,
    ParentElement, Styled, Theme,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::IconSet;
use crate::ui::PageCommandTarget;
use gpui::App;

use super::super::editor::{PageComposerRenderAction, PageRenderAction};

#[derive(Clone)]
pub(crate) struct PageCommandRenderer {
    theme: Theme,
    icons: Arc<IconSet>,
    actions: ViewActionSink<PageRenderAction>,
    commands: Arc<[PageCommand]>,
    selected_row_index: usize,
}

impl PageCommandRenderer {
    pub(crate) fn new(
        theme: Theme,
        icons: Arc<IconSet>,
        actions: ViewActionSink<PageRenderAction>,
        commands: Arc<[PageCommand]>,
        selected_row_index: usize,
    ) -> Self {
        Self {
            theme,
            icons,
            actions,
            commands,
            selected_row_index,
        }
    }

    pub(crate) fn render(&self, cx: &mut App) -> Div {
        div().pt(px(6.0)).child(
            (div().flex().flex_col().child(
                (command_menu_panel_shell(self.theme)
                    .child(command_menu_section_title(self.theme, "Basic blocks"))
                    .when(self.commands.is_empty(), |this| {
                        this.child(
                            command_menu_empty_state(self.theme, "No matching commands")
                                .into_any_element(),
                        )
                    })
                    .children(self.commands.iter().enumerate().map(|(index, command)| {
                        self.render_option(index, command, index == self.selected_row_index, cx)
                    }))
                    .child(command_menu_divider(self.theme))
                    .child(self.render_close_row(self.selected_row_index == self.commands.len())))
                .into_any_element(),
            ))
            .into_any_element(),
        )
    }

    pub(crate) fn render_option(
        &self,
        index: usize,
        command: &PageCommand,
        is_selected: bool,
        cx: &mut App,
    ) -> AnyElement {
        let target = command.target;
        let click_actions = self.actions.clone();
        let hover_actions = self.actions.clone();
        let mut row = command_menu_row_shell(self.theme, is_selected)
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                click_actions.emit(
                    PageRenderAction::Composer(PageComposerRenderAction::Select(target)),
                    window,
                    cx,
                );
            })
            .child(command_menu_option_body(
                self.theme,
                render_page_command_icon(self.theme, &self.icons, command, cx),
                command.label,
                command.shortcut,
            ));
        row.interactivity()
            .on_hover(move |is_hovered: &bool, window, cx| {
                if *is_hovered {
                    hover_actions.emit(
                        PageRenderAction::Composer(PageComposerRenderAction::Hover(index)),
                        window,
                        cx,
                    );
                }
            });
        row.into_any_element()
    }

    pub(crate) fn render_close_row(&self, is_selected: bool) -> AnyElement {
        let click_actions = self.actions.clone();
        let hover_actions = self.actions.clone();
        let close_index = self.commands.len();
        let mut row = div()
            .mx(px(6.0))
            .h(px(28.0))
            .rounded(px(6.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .bg(if is_selected {
                rgba(self.theme.command_menu_active_bg)
            } else {
                alpha(self.theme.elevated_surface_bg, 0.0)
            })
            .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
                click_actions.emit(
                    PageRenderAction::Composer(PageComposerRenderAction::CloseMenu),
                    window,
                    cx,
                );
            })
            .child(command_menu_close_row_body(self.theme));
        row.interactivity()
            .on_hover(move |is_hovered: &bool, window, cx| {
                if *is_hovered {
                    hover_actions.emit(
                        PageRenderAction::Composer(PageComposerRenderAction::Hover(close_index)),
                        window,
                        cx,
                    );
                }
            });
        row.into_any_element()
    }
}

pub(in crate::ui::board_workspace::page) fn render_page_command_icon(
    theme: Theme,
    icons: &IconSet,
    command: &PageCommand,
    cx: &mut App,
) -> AnyElement {
    match command.target {
        PageCommandTarget::Editable(CardPageBlockKind::Text) => {
            render_page_command_glyph_icon(theme, "T").into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::SubHeader) => {
            render_page_command_glyph_icon(theme, "H1").into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::SubSubHeader) => {
            render_page_command_glyph_icon(theme, "H2").into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::Heading3) => {
            render_page_command_glyph_icon(theme, "H3").into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::Heading4) => {
            render_page_command_glyph_icon(theme, "H4").into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::BulletedList) => {
            render_page_command_bulleted_list_icon(theme).into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::NumberedList) => {
            render_page_command_numbered_list_icon(theme).into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::ToDoList) => {
            render_page_command_todo_list_icon(theme).into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::ToggleList) => {
            render_page_command_toggle_list_icon(theme, icons.page_toggle_collapsed.render(cx))
                .into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::PageLink) => {
            img(icons.page.render(cx)).size(px(14.0)).into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::Callout) => {
            render_page_command_callout_icon(theme).into_any()
        }
        PageCommandTarget::Editable(CardPageBlockKind::Quote) => {
            render_page_command_glyph_icon(theme, "\"").into_any()
        }
        PageCommandTarget::Code => render_page_command_glyph_icon(theme, "</>").into_any(),
        PageCommandTarget::Editable(CardPageBlockKind::Code) => {
            unreachable!("Code creation uses its typed page-command target")
        }
        PageCommandTarget::Divider => render_page_command_glyph_icon(theme, "—").into_any(),
    }
}
