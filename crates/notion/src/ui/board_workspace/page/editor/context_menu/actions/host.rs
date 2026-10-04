use super::super::super::block_ops::PageBlockContextTargets;
use gpui::{Context, Window};

use super::super::super::SurfaceState;
use super::super::catalog::PageBlockMenuAction;
use super::super::state::{PageBlockContextMenuTarget, SupportedPageBlockAction};
use super::editing::PageBlockRootCommand;
use super::{PageBlockContextMenuAction, PageBlockContextMenuCommand, PageBlockRootActionOrigin};
use crate::ui::board_workspace::PageEditSession;

pub(in super::super) fn handle_page_block_context_menu_action(
    surface: &mut SurfaceState,
    action: PageBlockContextMenuAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let block_id = action.explicit_block_id().map(str::to_string).or_else(|| {
        surface
            .page_editor
            .page_block_context_menu
            .as_ref()
            .map(|menu| menu.block_id.clone())
    });
    let target = block_id
        .as_deref()
        .and_then(|block_id| resolve_page_block_context_menu_target(surface, block_id));
    let transition = surface
        .page_editor
        .reduce_page_block_context_menu(action, target.as_ref());
    if let Some(input) = transition.focus {
        input.update(cx, |input, cx| input.request_focus(cx));
    }
    if let Some(command) = transition.command {
        dispatch_page_block_context_menu_command(surface, command, cx);
    }
    if transition.notify {
        cx.notify();
    }
}

pub(in super::super) fn resolve_page_block_context_menu_target(
    surface: &SurfaceState,
    block_id: &str,
) -> Option<PageBlockContextMenuTarget> {
    let page = surface.page_documents.page_containing_block(block_id)?;
    let page_id = page.block_id.clone();
    let block = page
        .blocks
        .iter()
        .find(|block| block.block_id == block_id)?
        .clone();
    let targets = PageBlockContextTargets::new(
        &page,
        surface
            .page_editor
            .page_block_context_menu_root_indices(&page, block_id),
    );
    let code = targets.code_block();
    Some(PageBlockContextMenuTarget {
        block,
        page_id,
        is_code: code.is_some(),
        code_wrap: code.and_then(|editable| editable.code_wrap()),
        code_language: code.and_then(|editable| editable.code_language()).cloned(),
        can_turn_into: targets.can_turn_into(),
        can_color: targets.can_color(),
        current_color: targets.current_color(),
        targets_are_quotes: targets.are_quotes(),
        current_quote_size: targets.current_quote_size(),
        can_copy_link: surface.notion_startup.board_url().is_some(),
        can_duplicate: targets.can_duplicate(),
        can_delete: targets.can_delete(),
        writable_comment_target: surface
            .page_documents
            .loaded_writable_comment_target(block_id),
    })
}

fn dispatch_page_block_context_menu_command(
    surface: &mut SurfaceState,
    command: PageBlockContextMenuCommand,
    cx: &mut Context<SurfaceState>,
) -> bool {
    match command {
        PageBlockContextMenuCommand::DismissInteraction => {
            surface.page_editor.dismiss_page_block_interaction(cx);
        }
        PageBlockContextMenuCommand::Root {
            block_id,
            action,
            origin,
        } => {
            return dispatch_page_block_root_action(surface, &block_id, action, origin, cx);
        }
        PageBlockContextMenuCommand::Edit(command) => {
            let transition = {
                let mut edit =
                    PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                edit.apply_block_context_command(command, cx);
                edit.finish(())
            };
            surface.apply_page_edit_transition(transition, cx);
        }
    }
    true
}

impl SurfaceState {
    pub(crate) fn activate_supported_page_block_action(
        &mut self,
        block_id: &str,
        action: SupportedPageBlockAction,
        cx: &mut Context<Self>,
    ) -> bool {
        dispatch_page_block_context_menu_command(
            self,
            PageBlockContextMenuCommand::Root {
                block_id: block_id.to_string(),
                action: supported_page_block_menu_action(action),
                origin: PageBlockRootActionOrigin::Shortcut,
            },
            cx,
        )
    }
}

fn supported_page_block_menu_action(action: SupportedPageBlockAction) -> PageBlockMenuAction {
    match action {
        SupportedPageBlockAction::CopyLink => PageBlockMenuAction::CopyLink,
        SupportedPageBlockAction::Duplicate => PageBlockMenuAction::Duplicate,
        SupportedPageBlockAction::Delete => PageBlockMenuAction::Delete,
        SupportedPageBlockAction::AskAi => PageBlockMenuAction::AskAi,
    }
}

fn dispatch_page_block_root_action(
    surface: &mut SurfaceState,
    block_id: &str,
    action: PageBlockMenuAction,
    origin: PageBlockRootActionOrigin,
    cx: &mut Context<SurfaceState>,
) -> bool {
    let Some(target) = resolve_page_block_context_menu_target(surface, block_id) else {
        return false;
    };
    if (origin == PageBlockRootActionOrigin::Menu && !target.action_visible(action))
        || !target.action_enabled(action)
    {
        return false;
    }
    let block_id = target.block.block_id.clone();
    match action.root_command() {
        PageBlockRootCommand::Edit(command) => {
            let board_url = surface.notion_startup.board_url();
            let transition = {
                let mut edit =
                    PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                edit.apply_block_root_command(&block_id, command, board_url, cx);
                edit.finish(())
            };
            surface.apply_page_edit_transition(transition, cx);
        }
        PageBlockRootCommand::EditIcon => {
            surface.toggle_page_link_icon_picker(target.page_id.clone(), block_id.clone(), cx)
        }
        PageBlockRootCommand::AskAi => {
            surface.notion_chrome.notion_ai_block_context = target.ai_block_context();
            surface.page_editor.page_block_context_menu = None;
            surface.notion_chrome.notion_ai_open = true;
            surface.notion_chrome.notion_ai_input_active = true;
            cx.notify();
        }
        PageBlockRootCommand::Comment => {
            let Some((page_id, target_id)) = target.writable_comment_target else {
                return false;
            };
            surface.open_notion_page_comments(page_id, target_id, cx);
        }
        PageBlockRootCommand::Unsupported => return false,
    }
    true
}
