use crate::ui::surface::NotionShareDialogState;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::Theme;

mod actions;
mod dialog;
mod host;
mod lifecycle;
mod member_search;
mod operations;
mod role_picker;
mod rows;

pub(crate) use actions::ShareEvent;
use actions::{ShareMutation, ShareUpdate};

struct ShareDialogView<'a> {
    dialog: &'a NotionShareDialogState,
    theme: Theme,
    right_inset: f32,
    actions: ViewActionSink<ShareEvent>,
}

const SHARE_DIALOG_WIDTH: f32 = 380.0;
const SHARE_DIALOG_HEIGHT: f32 = 440.0;
const SHARE_DIALOG_ROW_HEIGHT: f32 = 48.0;
const SHARE_DIALOG_HEADER_HEIGHT: f32 = 62.0;
const SHARE_ROLE_PICKER_WIDTH: f32 = 188.0;
const SHARE_ROLE_PICKER_ROW_HEIGHT: f32 = 32.0;
