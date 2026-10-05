use crate::ui::view_actions::ViewActionSink;

use super::{AppearanceMode, InlineToolbarAction, SharedString, Theme};

pub(super) struct InlineToolbarRenderer {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) database_title: SharedString,
    pub(super) actions: ViewActionSink<InlineToolbarAction>,
}
