use std::sync::Arc;

use gpui::{
    anchored, deferred, point, Anchor, AnchoredPositionMode, AnyElement, App, Bounds, Div,
    ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels,
    Role, Stateful, StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::ClickAwayBoundary;

use super::actions::PageMentionAction;
use super::state::{PageMentionController, PageMentionMenuIdentity};
use super::{
    PageMentionClock, PageMentionMenuSection, PAGE_MENTION_MENU_ANCHOR_X_INSET,
    PAGE_MENTION_MENU_ANCHOR_Y_GAP, PAGE_MENTION_MENU_MAX_HEIGHT_RATIO,
    PAGE_MENTION_MENU_PAGE_ROW_HEIGHT, PAGE_MENTION_MENU_ROW_HEIGHT,
    PAGE_MENTION_MENU_VIEWPORT_MARGIN,
};
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::surface::NotionSurfaceResources;
use crate::ui::view_actions::{ViewActionSink, ViewNotifier};
use crate::ui::{
    alpha, command_menu_divider, command_menu_panel_shell, command_menu_section_header, div, px,
    rgb, AppearanceMode, IconSet, PageMentionMenuState, Theme, COMMAND_MENU_ROW_PADDING_X,
    COMMAND_MENU_SUBLABEL_SIZE,
};

mod row;

const PAGE_MENTION_MENU_CHROME_HEIGHT: f32 = 4.0 + 6.0 + 4.0 + 14.4 + 8.0 + 4.0;
const PAGE_MENTION_MENU_DIVIDER_HEIGHT: f32 = 9.0 + 1.0;
const PAGE_MENTION_MENU_FOOTER_HEIGHT: f32 = 30.0;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageMentionRendererResources {
    pub(in crate::ui::board_workspace::page::editor) theme: Theme,
    pub(in crate::ui::board_workspace::page::editor) appearance_mode: AppearanceMode,
    pub(in crate::ui::board_workspace::page::editor) icons: Arc<IconSet>,
    pub(in crate::ui::board_workspace::page::editor) notion_resources: NotionSurfaceResources,
    pub(in crate::ui::board_workspace::page::editor) notifier: ViewNotifier,
    pub(in crate::ui::board_workspace::page::editor) viewport_height: f32,
    pub(in crate::ui::board_workspace::page::editor) chrome_top_inset: f32,
    pub(in crate::ui::board_workspace::page::editor) clock: PageMentionClock,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageMentionRenderer {
    pub(super) resources: PageMentionRendererResources,
    pub(super) actions: ViewActionSink<PageMentionAction>,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageMentionMenuPresentation {
    menu: PageMentionMenuState,
    sections: Vec<PageMentionMenuSection>,
    trigger: Bounds<Pixels>,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageMentionMenuPresentationSeed {
    menu: PageMentionMenuState,
    sections: Vec<PageMentionMenuSection>,
}

impl PageMentionMenuPresentationSeed {
    pub(in crate::ui::board_workspace::page::editor) fn with_trigger(
        self,
        trigger: Bounds<Pixels>,
    ) -> PageMentionMenuPresentation {
        PageMentionMenuPresentation {
            menu: self.menu,
            sections: self.sections,
            trigger,
        }
    }
}

impl PageMentionController {
    pub(in crate::ui::board_workspace::page::editor) fn presentation_seed_for_block(
        &self,
        block_id: &str,
        clock: &PageMentionClock,
    ) -> Option<PageMentionMenuPresentationSeed> {
        let menu = self.menu_for_block(block_id)?.clone();
        Some(PageMentionMenuPresentationSeed {
            sections: self.sections(&menu, clock),
            menu,
        })
    }
}

impl PageMentionRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        resources: PageMentionRendererResources,
        actions: ViewActionSink<PageMentionAction>,
    ) -> Self {
        Self { resources, actions }
    }

    pub(in crate::ui::board_workspace::page::editor) fn render_menu(
        &self,
        presentation: &PageMentionMenuPresentation,
        cx: &mut App,
    ) -> AnyElement {
        let max_height = self.resources.viewport_height * PAGE_MENTION_MENU_MAX_HEIGHT_RATIO;
        let menu_height =
            estimated_menu_height(&presentation.sections, !presentation.menu.query.is_empty())
                .min(max_height);
        let x = presentation.trigger.left().as_f32() - PAGE_MENTION_MENU_ANCHOR_X_INSET;
        let below = presentation.trigger.bottom().as_f32() + PAGE_MENTION_MENU_ANCHOR_Y_GAP;
        let above = presentation.trigger.top().as_f32() - PAGE_MENTION_MENU_ANCHOR_Y_GAP;
        let space_below = (self.resources.viewport_height - below).max(0.0);
        let space_above = (above - self.resources.chrome_top_inset).max(0.0);
        let open_upward = space_below < menu_height + PAGE_MENTION_MENU_VIEWPORT_MARGIN
            && space_above > space_below;
        let (anchor, position) = if open_upward {
            (Anchor::BottomLeft, point(px(x), px(above)))
        } else {
            (Anchor::TopLeft, point(px(x), px(below)))
        };
        let panel = self.render_panel(presentation, max_height, cx);
        div()
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .size_0()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
            .child(
                deferred(
                    anchored()
                        .position(position)
                        .position_mode(AnchoredPositionMode::Window)
                        .anchor(anchor)
                        .child(panel),
                )
                .with_priority(100),
            )
            .into_any_element()
    }

    fn render_panel(
        &self,
        presentation: &PageMentionMenuPresentation,
        max_height: f32,
        cx: &mut App,
    ) -> Stateful<Div> {
        let mut row_index = 0usize;
        let mut panel = command_menu_panel_shell(self.resources.theme)
            .id(ElementId::Name(
                format!("notion-native-mention-menu-{}", presentation.menu.block_id).into(),
            ))
            .role(Role::ListBox)
            .aria_label("Mention a date, person or page")
            .max_h(px(max_height))
            .overflow_y_scroll();
        for (section_index, section) in presentation.sections.iter().enumerate() {
            if section_index > 0 {
                panel = panel.child(command_menu_divider(self.resources.theme));
            }
            panel = panel.child(command_menu_section_header(
                self.resources.theme,
                section.title,
            ));
            for row in &section.rows {
                panel = panel.child(self.render_row(
                    row::PageMentionRowSpec {
                        identity: PageMentionMenuIdentity::new(&presentation.menu),
                        index: row_index,
                        row,
                        selected: row_index == presentation.menu.selected_index,
                    },
                    cx,
                ));
                row_index += 1;
            }
        }
        if !presentation.menu.query.trim().is_empty() {
            panel = panel.child(self.render_footer());
        }
        let click_away = ClickAwayBoundary::new();
        let actions = self.actions.clone();
        let identity = PageMentionMenuIdentity::new(&presentation.menu);
        click_away.dismissible_with_handler(
            div().child(panel),
            move |_, window, cx| {
                actions.emit(PageMentionAction::DismissMenu(identity.clone()), window, cx);
            },
            cx,
        )
    }

    fn render_footer(&self) -> Div {
        div()
            .mt(px(4.0))
            .mx(px(-4.0))
            .mb(px(-4.0))
            .h(px(PAGE_MENTION_MENU_FOOTER_HEIGHT))
            .border_t_1()
            .border_color(alpha(self.resources.theme.text_primary, 0.07))
            .flex()
            .items_center()
            .pl(px(COMMAND_MENU_ROW_PADDING_X + 4.0))
            .gap(px(8.0))
            .text_size(px(COMMAND_MENU_SUBLABEL_SIZE))
            .text_color(rgb(self.resources.theme.text_hint))
            .child("👍")
            .child("👎")
            .child("Give search feedback")
    }

    pub(super) fn page_icons(&self) -> PageShellIconRenderer {
        PageShellIconRenderer::new(
            self.resources.appearance_mode,
            Arc::clone(&self.resources.icons),
            self.resources.notion_resources.clone(),
            self.resources.notifier.clone(),
        )
    }
}

fn estimated_menu_height(sections: &[PageMentionMenuSection], has_query: bool) -> f32 {
    let mut height = PAGE_MENTION_MENU_CHROME_HEIGHT;
    for (index, section) in sections.iter().enumerate() {
        if index > 0 {
            height += PAGE_MENTION_MENU_DIVIDER_HEIGHT + PAGE_MENTION_MENU_CHROME_HEIGHT - 8.0;
        }
        for row in &section.rows {
            height += if row.is_two_line() {
                PAGE_MENTION_MENU_PAGE_ROW_HEIGHT
            } else {
                PAGE_MENTION_MENU_ROW_HEIGHT
            } + 1.0;
        }
    }
    if has_query {
        height += PAGE_MENTION_MENU_FOOTER_HEIGHT;
    }
    height
}
