use gpui::Pixels;
use gpui_components::text_input::{TextInputPointerSelection, TextInputSnapshot};

use super::super::navigation::PageBlockNavigation;

pub(in crate::ui::board_workspace::page::editor) enum PageInputAction {
    Text(PageTextInputAction),
    Navigation(PageNavigationInputAction),
    Host(PageInputHostAction),
}

pub(in crate::ui::board_workspace::page::editor) enum PageTextInputAction {
    BlockChanged {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    MergeBackward {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    MergeForward {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    Indent {
        block_id: String,
        outdent: bool,
        snapshot: TextInputSnapshot,
    },
    History {
        block_id: String,
        redo: bool,
        cursor: usize,
    },
}

pub(in crate::ui::board_workspace::page::editor) enum PageNavigationInputAction {
    Navigate {
        block_id: String,
        navigation: PageBlockNavigation,
        snapshot: TextInputSnapshot,
        window_x: Option<Pixels>,
        extend: bool,
    },
    SelectAll {
        block_id: String,
        cursor: usize,
    },
    SelectionChanged {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    PointerSelectionChanged {
        block_id: String,
        selection: TextInputPointerSelection,
    },
    Focused(String),
    LayoutChanged(String),
}

pub(in crate::ui::board_workspace::page::editor) enum PageInputHostAction {
    Submit {
        block_id: String,
        snapshot: TextInputSnapshot,
    },
    MoveSlash {
        block_id: String,
        delta: isize,
    },
    CloseSlash(String),
    MoveBlock {
        block_id: String,
        delta: isize,
        cursor: usize,
    },
}
