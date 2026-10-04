use gpui::Entity;
use gpui_components::text_input::TextInput;

use super::catalog::PageBlockMenuAction;
use super::state::{PageBlockContextMenuPanel, PageBlockMenuMutation};

mod editing;
mod host;
use editing::PageBlockContextEditCommand;
mod reducer;

pub(super) use host::{
    handle_page_block_context_menu_action, resolve_page_block_context_menu_target,
};

pub(super) enum PageBlockContextMenuAction {
    Dismiss,
    DismissInteraction,
    SetRootQuery(String),
    SetCodeLanguageQuery(String),
    Submit,
    MoveSelection(isize),
    CloseSubmenu,
    OpenSelectedSubmenu,
    DeleteTarget,
    ActivateRoot {
        block_id: String,
        action: PageBlockMenuAction,
    },
    ActivateSubmenuRow {
        block_id: String,
        panel: PageBlockContextMenuPanel,
        index: usize,
    },
    HoverRoot {
        block_id: String,
        action: PageBlockMenuAction,
        index: usize,
    },
    HoverSubmenu {
        block_id: String,
        panel: PageBlockContextMenuPanel,
        index: usize,
    },
}

impl PageBlockContextMenuAction {
    fn explicit_block_id(&self) -> Option<&str> {
        match self {
            Self::ActivateRoot { block_id, .. }
            | Self::ActivateSubmenuRow { block_id, .. }
            | Self::HoverRoot { block_id, .. }
            | Self::HoverSubmenu { block_id, .. } => Some(block_id),
            Self::Dismiss
            | Self::DismissInteraction
            | Self::SetRootQuery(_)
            | Self::SetCodeLanguageQuery(_)
            | Self::Submit
            | Self::MoveSelection(_)
            | Self::CloseSubmenu
            | Self::OpenSelectedSubmenu
            | Self::DeleteTarget => None,
        }
    }
}

enum PageBlockContextMenuCommand {
    DismissInteraction,
    Root {
        block_id: String,
        action: PageBlockMenuAction,
        origin: PageBlockRootActionOrigin,
    },
    Edit(PageBlockContextEditCommand),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageBlockRootActionOrigin {
    Menu,
    Shortcut,
}

#[derive(Default)]
struct PageBlockContextMenuTransition {
    command: Option<PageBlockContextMenuCommand>,
    focus: Option<Entity<TextInput>>,
    notify: bool,
}

impl PageBlockContextMenuTransition {
    fn command(command: PageBlockContextMenuCommand) -> Self {
        Self {
            command: Some(command),
            ..Self::default()
        }
    }

    fn notified() -> Self {
        Self {
            notify: true,
            ..Self::default()
        }
    }
}

impl From<PageBlockMenuMutation> for PageBlockContextMenuTransition {
    fn from(mutation: PageBlockMenuMutation) -> Self {
        Self {
            command: None,
            focus: mutation.focus,
            notify: mutation.notify,
        }
    }
}
