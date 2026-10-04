use gpui::Context;

use crate::ui::SurfaceState;

mod actions;
mod controller;
mod mutation;
mod state;
mod view;

pub(crate) use actions::StatusPropertyPickerAction;
use controller::{StatusPropertyPickerContext, StatusPropertyPickerEffect};
use mutation::StatusPropertyBackend;

const PICKER_WIDTH: f32 = 240.0;
const PICKER_MAX_HEIGHT: f32 = 280.0;
const PICKER_MARGIN: f32 = 12.0;
const PICKER_ROW_HEIGHT: f32 = 32.0;

impl SurfaceState {
    pub(crate) fn dispatch_status_property_picker_action(
        &mut self,
        action: StatusPropertyPickerAction,
        cx: &mut Context<Self>,
    ) {
        let workspace_api = self.notion_startup.workspace_api();
        let board_url = self.notion_startup.board_url();
        let context = StatusPropertyPickerContext {
            board: &self.board,
            current_surface: cx.entity().downgrade(),
            page_host: self.presentation.page_host.clone(),
            backend: StatusPropertyBackend {
                workspace_api,
                board_url,
            },
        };
        let effects = self.notion_chrome.reduce_status_property_picker_action(
            &mut self.database_search,
            action,
            context,
            cx,
        );
        for effect in effects {
            match effect {
                StatusPropertyPickerEffect::ForwardToParent { parent, picker } => {
                    cx.defer(move |cx| {
                        parent
                            .update(cx, move |parent, cx| {
                                parent.dispatch_status_property_picker_action(
                                    StatusPropertyPickerAction::InstallHosted(picker),
                                    cx,
                                );
                            })
                            .expect("inline status picker requires its parent surface");
                    });
                }
                StatusPropertyPickerEffect::SpawnMutation(job) => (*job).spawn(cx),
                StatusPropertyPickerEffect::Error(error) => self.print_notion_error(error),
                StatusPropertyPickerEffect::Notify => cx.notify(),
            }
        }
    }
}
