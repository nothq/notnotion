mod feed;
mod header;
mod icons;
mod item;
mod item_actions;
mod menus;
mod states;
mod time;

use crate::ui::ScrollWheelEvent;

use super::{
    alpha, div, px, AnyElement, InteractiveElement, IntoElement, NotionSidebarInboxState,
    ParentElement, SidebarAction, SidebarInboxAction, SidebarRenderer, SidebarView,
    StatefulInteractiveElement, Styled,
};
use crate::ui::surface::NotionSidebarState;
use crate::ui::{Context, SurfaceState, Viewport};
use gpui::App;
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

use super::host::apply_sidebar_update;
use crate::ui::surface::NotionSidebarUpdate;

impl SidebarView<'_> {
    pub(crate) fn render_inbox_menu_backdrop(
        viewport: Viewport,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        dismissible_backdrop(
            div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .w(px(viewport.app_width()))
                .h(px(viewport.app_height()))
                .bg(alpha(0x000000, 0.001)),
            BackdropDismissal::new(|surface: &mut SurfaceState, _, _, cx| {
                apply_sidebar_update(surface, NotionSidebarUpdate::DismissInboxMenus, cx);
            }),
            cx,
        )
        .into_any_element()
    }
}

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_inbox(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> AnyElement {
        let content = match &state.inbox {
            NotionSidebarInboxState::Idle | NotionSidebarInboxState::Loading => {
                self.render_notion_inbox_skeleton()
            }
            NotionSidebarInboxState::Loaded { items, .. } if items.is_empty() => {
                self.render_notion_inbox_empty(state.inbox_filter, cx)
            }
            NotionSidebarInboxState::Loaded { items, .. } => {
                self.render_notion_inbox_items(state, items, cx)
            }
            NotionSidebarInboxState::Failed => div().into_any_element(),
        };
        div()
            .id("notion-sidebar-inbox-body")
            .size_full()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&state.inbox_scroll_handle)
            .on_scroll_wheel(self.actions.listener(|_: &ScrollWheelEvent, _, _| {
                SidebarAction::Inbox(SidebarInboxAction::LoadMore)
            }))
            .pt(px(6.0))
            .px(px(8.0))
            .pb(px(16.0))
            .child(content)
            .into_any_element()
    }
}
