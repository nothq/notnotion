use gpui::{
    div, px, InteractiveElement, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};

use crate::ui::view_actions::ViewActionSink;
use crate::ui::{rgb, rgba, Theme};

use super::{
    position_inline_database_control_dialog, DatabaseViewControlAction, DatabaseViewControlHost,
    DatabaseViewControlPlacement, DatabaseViewControlsResources, DatabaseViewControlsSnapshot,
    CONTROL_DIALOG_HEADER_HEIGHT, CONTROL_DIALOG_MAX_HEIGHT, CONTROL_DIALOG_ROW_HEIGHT,
    CONTROL_DIALOG_WIDTH,
};

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct DatabaseViewControlsRenderer {
    pub(super) snapshot: DatabaseViewControlsSnapshot,
    pub(super) theme: Theme,
    pub(super) actions: ViewActionSink<DatabaseViewControlAction>,
}

impl DatabaseViewControlsRenderer {
    pub(super) fn new(
        snapshot: DatabaseViewControlsSnapshot,
        theme: Theme,
        actions: ViewActionSink<DatabaseViewControlAction>,
    ) -> Self {
        Self {
            snapshot,
            theme,
            actions,
        }
    }

    pub(super) fn resources(
        &self,
        placement: DatabaseViewControlPlacement,
    ) -> DatabaseViewControlsResources {
        DatabaseViewControlsResources {
            theme: self.theme,
            host: match placement {
                DatabaseViewControlPlacement::FullPage { .. } => DatabaseViewControlHost::FullPage,
                DatabaseViewControlPlacement::Inline { .. } => DatabaseViewControlHost::Inline,
            },
        }
    }

    pub(super) fn dialog(
        &self,
        placement: DatabaseViewControlPlacement,
        label: &'static str,
        row_count: usize,
    ) -> gpui::Stateful<gpui::Div> {
        let desired_height =
            (CONTROL_DIALOG_HEADER_HEIGHT + row_count as f32 * CONTROL_DIALOG_ROW_HEIGHT + 8.0)
                .min(CONTROL_DIALOG_MAX_HEIGHT);
        let mut dialog = div()
            .id(format!("notion-database-{}-dialog", label.to_lowercase()))
            .role(Role::Dialog)
            .aria_label(label)
            .absolute()
            .w(px(CONTROL_DIALOG_WIDTH))
            .h(px(desired_height))
            .min_h(px(CONTROL_DIALOG_HEADER_HEIGHT + CONTROL_DIALOG_ROW_HEIGHT))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
            .flex()
            .flex_col();
        dialog = match placement {
            DatabaseViewControlPlacement::FullPage { top, right } => {
                dialog.top(px(top)).right(px(right))
            }
            DatabaseViewControlPlacement::Inline {
                anchor,
                viewport_width,
                viewport_height,
            } => position_inline_database_control_dialog(
                dialog,
                anchor,
                viewport_width,
                viewport_height,
                desired_height,
            ),
        };
        dialog
    }

    pub(super) fn header(&self, title: &'static str, detail: &'static str) -> gpui::Div {
        div()
            .h(px(CONTROL_DIALOG_HEADER_HEIGHT))
            .flex_none()
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(detail),
            )
    }
}
