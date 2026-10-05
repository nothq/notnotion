use super::super::{CardPageBlock, CardPageBlockKind, CardPageStructuralBlock};

pub(super) const PAGE_BLOCK_MENU_WIDTH: f32 = 265.0;
pub(super) const PAGE_BLOCK_SUBMENU_WIDTH: f32 = 220.0;
pub(super) const PAGE_CODE_LANGUAGE_MENU_WIDTH: f32 = 240.0;
pub(super) const PAGE_BLOCK_MENU_ROW_HEIGHT: f32 = 28.0;
pub(super) const PAGE_BLOCK_MENU_PANEL_INSET: f32 = 4.0;
pub(super) const PAGE_BLOCK_MENU_SUBMENU_OVERLAP: f32 = 4.0;
pub(super) const PAGE_BLOCK_COLOR_MENU_ROW_COUNT: usize = 21;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageBlockMenuAction {
    CodeLanguage,
    CodeWrap,
    CopyCode,
    TurnInto,
    Color,
    EditIcon,
    QuoteSize,
    CopyLink,
    Duplicate,
    MoveTo,
    Delete,
    Comment,
    SuggestEdits,
    Present,
    AskAi,
    Skills,
}

#[derive(Clone, Copy)]
pub(super) struct PageBlockMenuActionSpec {
    pub(super) action: PageBlockMenuAction,
    pub(super) label: &'static str,
    pub(super) shortcut: Option<&'static str>,
    pub(super) icon: &'static str,
    pub(super) group: u8,
}

pub(super) const PAGE_BLOCK_MENU_ACTIONS: [PageBlockMenuActionSpec; 16] = [
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::TurnInto,
        label: "Turn into",
        shortcut: None,
        icon: "↪",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::CopyCode,
        label: "Copy code",
        shortcut: None,
        icon: "",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::CodeWrap,
        label: "Wrap code",
        shortcut: None,
        icon: "",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::CodeLanguage,
        label: "Language",
        shortcut: None,
        icon: "",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Color,
        label: "Color",
        shortcut: None,
        icon: "▣",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::EditIcon,
        label: "Edit icon",
        shortcut: None,
        icon: "✎",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::QuoteSize,
        label: "Quote size",
        shortcut: None,
        icon: "≡",
        group: 0,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::CopyLink,
        label: "Copy link to block",
        shortcut: Some("⌘⌃L"),
        icon: "↗",
        group: 1,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Duplicate,
        label: "Duplicate",
        shortcut: Some("⌘D"),
        icon: "▢",
        group: 1,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::MoveTo,
        label: "Move to",
        shortcut: Some("⌘⇧P"),
        icon: "↱",
        group: 1,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Delete,
        label: "Delete",
        shortcut: Some("Del"),
        icon: "⌫",
        group: 1,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Comment,
        label: "Comment",
        shortcut: Some("⌘⇧M"),
        icon: "□",
        group: 2,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::SuggestEdits,
        label: "Suggest edits",
        shortcut: Some("⌘⇧⌥X"),
        icon: "✓",
        group: 2,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Present,
        label: "Present from here",
        shortcut: Some("⌘⌥P"),
        icon: "▻",
        group: 3,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::AskAi,
        label: "Ask AI",
        shortcut: Some("⌘J"),
        icon: "⌁",
        group: 4,
    },
    PageBlockMenuActionSpec {
        action: PageBlockMenuAction::Skills,
        label: "Skills",
        shortcut: None,
        icon: "⌘",
        group: 4,
    },
];

#[derive(Clone, Copy)]
pub(super) struct PageBlockTurnIntoSpec {
    pub(super) label: &'static str,
    pub(super) kind: Option<CardPageBlockKind>,
}

pub(super) const PAGE_BLOCK_TURN_INTO_ACTIONS: [PageBlockTurnIntoSpec; 24] = [
    PageBlockTurnIntoSpec {
        label: "Text",
        kind: Some(CardPageBlockKind::Text),
    },
    PageBlockTurnIntoSpec {
        label: "Heading 1",
        kind: Some(CardPageBlockKind::SubHeader),
    },
    PageBlockTurnIntoSpec {
        label: "Heading 2",
        kind: Some(CardPageBlockKind::SubSubHeader),
    },
    PageBlockTurnIntoSpec {
        label: "Heading 3",
        kind: Some(CardPageBlockKind::Heading3),
    },
    PageBlockTurnIntoSpec {
        label: "Heading 4",
        kind: Some(CardPageBlockKind::Heading4),
    },
    PageBlockTurnIntoSpec {
        label: "Page",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Page in",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Bulleted list",
        kind: Some(CardPageBlockKind::BulletedList),
    },
    PageBlockTurnIntoSpec {
        label: "Numbered list",
        kind: Some(CardPageBlockKind::NumberedList),
    },
    PageBlockTurnIntoSpec {
        label: "To-do list",
        kind: Some(CardPageBlockKind::ToDoList),
    },
    PageBlockTurnIntoSpec {
        label: "Toggle list",
        kind: Some(CardPageBlockKind::ToggleList),
    },
    PageBlockTurnIntoSpec {
        label: "Code",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Quote",
        kind: Some(CardPageBlockKind::Quote),
    },
    PageBlockTurnIntoSpec {
        label: "Callout",
        kind: Some(CardPageBlockKind::Callout),
    },
    PageBlockTurnIntoSpec {
        label: "Block equation",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Synced block",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Toggle heading 1",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Toggle heading 2",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Toggle heading 3",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "Toggle heading 4",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "2 columns",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "3 columns",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "4 columns",
        kind: None,
    },
    PageBlockTurnIntoSpec {
        label: "5 columns",
        kind: None,
    },
];

pub(super) fn page_block_type_label(block: &CardPageBlock) -> &'static str {
    if block.alias_content().is_some() {
        return "Link to page";
    }
    if block.resource_content().is_some() {
        return "Image";
    }
    if block.simple_table_content().is_some() {
        return "Table";
    }
    if block.unsupported_leaf_content().is_some() {
        return "Block";
    }
    match block.editable_content().map(|editable| editable.kind) {
        Some(CardPageBlockKind::Text) => "Text",
        Some(CardPageBlockKind::SubHeader) => "Heading 1",
        Some(CardPageBlockKind::SubSubHeader) => "Heading 2",
        Some(CardPageBlockKind::Heading3) => "Heading 3",
        Some(CardPageBlockKind::Heading4) => "Heading 4",
        Some(CardPageBlockKind::BulletedList) => "Bulleted list",
        Some(CardPageBlockKind::NumberedList) => "Numbered list",
        Some(CardPageBlockKind::ToDoList) => "To-do list",
        Some(CardPageBlockKind::ToggleList) => "Toggle list",
        Some(CardPageBlockKind::PageLink) => "Page",
        Some(CardPageBlockKind::Callout) => "Callout",
        Some(CardPageBlockKind::Quote) => "Quote",
        Some(CardPageBlockKind::Code) => "Code",
        None if matches!(
            block.structural_content(),
            Some(CardPageStructuralBlock::Divider)
        ) =>
        {
            "Divider"
        }
        None => "Block",
    }
}

pub(super) fn page_block_turn_into_glyph(label: &str) -> &'static str {
    match label {
        "Page" | "Page in" => "▣",
        "Code" => "</>",
        "Callout" => "▰",
        "Block equation" => "∑",
        "Synced block" => "↻",
        "Toggle heading 1" | "Toggle heading 2" | "Toggle heading 3" | "Toggle heading 4" => "▸",
        "2 columns" | "3 columns" | "4 columns" | "5 columns" => "▦",
        _ => "·",
    }
}
