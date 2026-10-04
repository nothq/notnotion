pub(super) use super::super::{
    alpha, div, img, px, relative, rgb, rgba, AnyElement, Context, Div, FluentBuilder, FontWeight,
    IconAsset, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, SurfaceState, ToolbarDialogKind, ViewTab,
    ViewTabKind, VIEW_TAB_ICON_GAP, VIEW_TAB_ICON_SIZE,
};
use crate::ui::surface::InlineDatabaseView;
pub(super) use crate::ui::surface::{
    AiAutofillDialogState, InlineToolbarDialogAnchors, InlineToolbarDialogState,
};
pub(super) use gpui::{App, AppContext, Role};
pub(super) use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};
pub(super) use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct InlineDatabaseToolbarTarget {
    pub(crate) parent_surface: gpui::WeakEntity<SurfaceState>,
    pub(crate) inline_view: gpui::WeakEntity<InlineDatabaseView>,
    pub(crate) database_block_id: String,
    pub(crate) board_url: String,
    pub(crate) database_title: String,
}

mod buttons;
mod inline_controls;
mod primary_new;
mod render;
mod search;
mod undated;
mod views;

mod actions;
mod host;

use crate::ui::{surface::DatabaseSearchState, view_actions::ViewActionSink, IconSet, Theme};
use actions::{BoardToolbarAction, BoardToolbarMeasurement};

pub(in crate::ui) struct BoardToolbarRenderer<'a> {
    theme: Theme,
    icons: &'a IconSet,
    database_search: &'a DatabaseSearchState,
    inline_toolbar_minimized: bool,
    undated_count: Option<usize>,
    undated_expanded: bool,
    actions: ViewActionSink<BoardToolbarAction>,
}
