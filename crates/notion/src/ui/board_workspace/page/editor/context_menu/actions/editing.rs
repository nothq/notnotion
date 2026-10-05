use gpui::App;

use crate::ui::board_workspace::PageEditSession;
use crate::ui::{CardPageBlockColor, CardPageBlockKind, CardPageQuoteSize};

use super::super::catalog::{page_block_type_label, PageBlockMenuAction};
use super::super::state::PageBlockContextMenuTarget;

pub(super) enum PageBlockContextEditCommand {
    TurnInto {
        block_id: String,
        kind: CardPageBlockKind,
    },
    SetColor {
        block_id: String,
        color: CardPageBlockColor,
    },
    SetQuoteSize {
        block_id: String,
        size: CardPageQuoteSize,
    },
    SetCodeLanguage {
        block_id: String,
        language: crate::model::CardPageCodeLanguage,
    },
}

pub(super) enum PageBlockRootEditCommand {
    CodeWrap,
    CopyCode,
    CopyLink,
    Duplicate,
    Delete,
}

pub(super) enum PageBlockRootCommand {
    Edit(PageBlockRootEditCommand),
    EditIcon,
    AskAi,
    Comment,
    Unsupported,
}

impl PageBlockMenuAction {
    pub(super) fn root_command(self) -> PageBlockRootCommand {
        match self {
            Self::CodeWrap => PageBlockRootCommand::Edit(PageBlockRootEditCommand::CodeWrap),
            Self::CopyCode => PageBlockRootCommand::Edit(PageBlockRootEditCommand::CopyCode),
            Self::CopyLink => PageBlockRootCommand::Edit(PageBlockRootEditCommand::CopyLink),
            Self::Duplicate => PageBlockRootCommand::Edit(PageBlockRootEditCommand::Duplicate),
            Self::Delete => PageBlockRootCommand::Edit(PageBlockRootEditCommand::Delete),
            Self::EditIcon => PageBlockRootCommand::EditIcon,
            Self::AskAi => PageBlockRootCommand::AskAi,
            Self::Comment => PageBlockRootCommand::Comment,
            Self::CodeLanguage
            | Self::TurnInto
            | Self::Color
            | Self::QuoteSize
            | Self::MoveTo
            | Self::SuggestEdits
            | Self::Present
            | Self::Skills => PageBlockRootCommand::Unsupported,
        }
    }
}

impl PageEditSession<'_> {
    pub(super) fn apply_block_context_command(
        &mut self,
        command: PageBlockContextEditCommand,
        cx: &mut App,
    ) {
        match command {
            PageBlockContextEditCommand::TurnInto { block_id, kind } => {
                self.turn_page_block_into(&block_id, kind, cx)
            }
            PageBlockContextEditCommand::SetColor { block_id, color } => {
                self.set_page_block_color(&block_id, color, cx)
            }
            PageBlockContextEditCommand::SetQuoteSize { block_id, size } => {
                self.set_page_quote_size(&block_id, size, cx)
            }
            PageBlockContextEditCommand::SetCodeLanguage { block_id, language } => {
                self.set_page_code_language(&block_id, language, cx)
            }
        }
    }

    pub(super) fn apply_block_root_command(
        &mut self,
        block_id: &str,
        command: PageBlockRootEditCommand,
        board_url: Option<String>,
        cx: &mut App,
    ) {
        match command {
            PageBlockRootEditCommand::CodeWrap => self.toggle_page_code_wrap(block_id, cx),
            PageBlockRootEditCommand::CopyCode => self.copy_page_code_text(block_id),
            PageBlockRootEditCommand::CopyLink => self.copy_page_block_link(block_id, board_url),
            PageBlockRootEditCommand::Duplicate => self.duplicate_page_block(block_id, cx),
            PageBlockRootEditCommand::Delete => self.delete_page_block(block_id, cx),
        }
    }
}

impl PageBlockContextMenuTarget {
    pub(super) fn ai_block_context(&self) -> Option<String> {
        self.block.editable_content().map(|editable| {
            if editable.text.trim().is_empty() {
                page_block_type_label(&self.block).to_string()
            } else {
                editable.text.clone()
            }
        })
    }
}
