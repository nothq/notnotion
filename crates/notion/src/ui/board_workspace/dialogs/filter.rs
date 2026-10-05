mod action_menu;
mod advanced_actions;
mod advanced_editor;
mod advanced_menus;
mod advanced_row;
mod advanced_tree;
mod apply;
mod bar;
mod calendar;
mod chips;
mod choices;
mod controller;
mod date_actions;
mod date_editor;
mod date_helpers;
mod date_pickers;
mod dialog;
mod draft;
mod editor;
mod helpers;
mod inputs;
mod key;
mod labels;
mod picker_header;
mod prelude;
mod property_actions;
mod renderer;
mod save;
mod types;
mod visual_helpers;

pub(super) use types::DatabaseFilterHost;
pub(crate) use types::{DatabaseFilterDialogPlacement, DATABASE_FILTER_BAR_HEIGHT};

use std::sync::Arc;

use gpui::{Context, Window};

use crate::ui::board_workspace::ToolbarDialogKind;
use crate::ui::surface::{InlineToolbarDialogState, SurfaceState};
use crate::ui::view_actions::ViewActionSink;

use apply::schedule_database_filter_query;
use controller::DatabaseFilterController;
use property_actions::execute_database_filter_relation_search;
use renderer::{DatabaseFilterRenderResources, DatabaseFilterRenderer};
use save::execute_database_filter_save;
use types::{DatabaseFilterAction, DatabaseFilterEffect, DatabaseFilterPresentationEffect};

pub(super) fn database_filter_renderer<'a>(
    surface: &'a SurfaceState,
    host: DatabaseFilterHost,
    cx: &Context<SurfaceState>,
) -> DatabaseFilterRenderer<'a> {
    DatabaseFilterRenderer::new(
        &surface.database_filter,
        &surface.board,
        DatabaseFilterRenderResources {
            theme: surface.theme,
            appearance_mode: surface.appearance_mode,
            page_icons: surface.page_shell_icon_renderer(cx),
            workspace_editable: surface.notion_startup.workspace_api().is_some(),
            host,
            actions: ViewActionSink::new(cx, handle_database_filter_action),
        },
    )
}

fn handle_database_filter_action(
    surface: &mut SurfaceState,
    action: DatabaseFilterAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let effects =
        DatabaseFilterController::new(&mut surface.database_filter, &surface.board).apply(action);
    for effect in effects {
        execute_database_filter_effect(surface, effect, cx);
    }
    cx.notify();
}

fn execute_database_filter_effect(
    surface: &mut SurfaceState,
    effect: DatabaseFilterEffect,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        DatabaseFilterEffect::QueryProjection {
            filter_state,
            revision,
        } => schedule_database_filter_query(surface, filter_state, revision, cx),
        DatabaseFilterEffect::LoadUsers => execute_database_filter_user_load(surface, cx),
        DatabaseFilterEffect::ScheduleRelationSearch => {
            execute_database_filter_relation_search(surface, cx)
        }
        DatabaseFilterEffect::Save(host) => execute_database_filter_save(surface, host, cx),
        DatabaseFilterEffect::Show(host) => execute_database_filter_presentation_effect(
            surface,
            DatabaseFilterPresentationEffect::Show(host),
            cx,
        ),
        DatabaseFilterEffect::Dismiss { host, close_local } => {
            execute_database_filter_presentation_effect(
                surface,
                DatabaseFilterPresentationEffect::Dismiss { host, close_local },
                cx,
            )
        }
        DatabaseFilterEffect::Report(error) => surface.print_notion_error(error),
    }
}

fn execute_database_filter_user_load(surface: &mut SurfaceState, cx: &mut Context<SurfaceState>) {
    if surface.database_filter.users.is_some() || surface.database_filter.users_in_flight {
        return;
    }
    let Some(workspace_api) = surface.notion_startup.workspace_api() else {
        return;
    };
    let session = surface.database_filter.session.clone();
    surface.database_filter.users_in_flight = true;
    surface.spawn_background_task(
        (),
        cx,
        move |()| workspace_api.load_database_filter_users(),
        move |surface, result, cx| {
            if !Arc::ptr_eq(&surface.database_filter.session, &session) {
                return;
            }
            surface.database_filter.users_in_flight = false;
            match result {
                Ok(result) => surface.database_filter.users = Some(result.users.into()),
                Err(error) => {
                    if surface.handle_notion_workspace_failure(
                        "database filter user loading failed",
                        error,
                        cx,
                    ) {
                        return;
                    }
                }
            }
            cx.notify();
        },
    );
}

fn execute_database_filter_presentation_effect(
    surface: &mut SurfaceState,
    effect: DatabaseFilterPresentationEffect,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        DatabaseFilterPresentationEffect::Show(DatabaseFilterHost::FullPage) => {
            surface.notion_chrome.toolbar_dialog = Some(ToolbarDialogKind::Filter);
            cx.notify();
        }
        DatabaseFilterPresentationEffect::Show(DatabaseFilterHost::Inline) => {
            let anchor = surface
                .database_filter
                .editor_anchor
                .or(surface.board_view.inline_toolbar_dialog_anchors.filter)
                .expect("inline filter dialog requires a measured filter control");
            let child_surface = cx.entity().downgrade();
            let parent_surface = surface
                .presentation
                .page_host
                .clone()
                .expect("inline filter controls require their parent surface");
            parent_surface
                .update(cx, move |parent, cx| {
                    parent.notion_chrome.inline_toolbar_dialog = Some(InlineToolbarDialogState {
                        anchor,
                        dialog: ToolbarDialogKind::Filter,
                        surface: child_surface,
                    });
                    cx.notify();
                })
                .expect("inline filter controls require their parent surface");
        }
        DatabaseFilterPresentationEffect::Dismiss { host, close_local } => {
            if close_local || host == DatabaseFilterHost::FullPage {
                surface.notion_chrome.close_toolbar_dialog(cx);
            }
            if host == DatabaseFilterHost::Inline {
                if let Some(parent) = surface.presentation.page_host.as_ref() {
                    let _ = parent.update(cx, |parent, cx| {
                        parent.notion_chrome.dismiss_inline_toolbar_dialog(cx);
                    });
                }
            }
        }
    }
}
