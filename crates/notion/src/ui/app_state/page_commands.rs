use crate::ui::CardPageBlockKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageCommandTarget {
    Editable(CardPageBlockKind),
    Code,
    Divider,
}

impl PageCommandTarget {
    pub const fn editable_kind(self) -> Option<CardPageBlockKind> {
        match self {
            Self::Editable(kind) => Some(kind),
            Self::Code | Self::Divider => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PageCommand {
    pub target: PageCommandTarget,
    pub label: &'static str,
    pub shortcut: Option<&'static str>,
    pub keywords: &'static [&'static str],
}

pub const PAGE_COMMANDS: [PageCommand; 14] = [
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::Text),
        label: "Text",
        shortcut: None,
        keywords: &["plain text", "paragraph", "writing"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::SubHeader),
        label: "Heading 1",
        shortcut: Some("#"),
        keywords: &["heading", "header", "title", "h1"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::SubSubHeader),
        label: "Heading 2",
        shortcut: Some("##"),
        keywords: &["heading", "header", "subtitle", "h2"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::Heading3),
        label: "Heading 3",
        shortcut: Some("###"),
        keywords: &["heading", "header", "section", "h3"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::Heading4),
        label: "Heading 4",
        shortcut: Some("####"),
        keywords: &["heading", "header", "small heading", "h4"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::BulletedList),
        label: "Bulleted list",
        shortcut: Some("-"),
        keywords: &["bullet", "unordered", "list"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::NumberedList),
        label: "Numbered list",
        shortcut: Some("1."),
        keywords: &["numbered", "ordered", "list"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::ToDoList),
        label: "To-do list",
        shortcut: Some("[]"),
        keywords: &["todo", "checkbox", "task", "to-do"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::ToggleList),
        label: "Toggle list",
        shortcut: Some(">"),
        keywords: &["toggle", "disclosure", "expand"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::PageLink),
        label: "Page",
        shortcut: None,
        keywords: &["page", "subpage", "nested page"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::Callout),
        label: "Callout",
        shortcut: None,
        keywords: &["callout", "note", "info"],
    },
    PageCommand {
        target: PageCommandTarget::Editable(CardPageBlockKind::Quote),
        label: "Quote",
        shortcut: Some("\""),
        keywords: &["quote", "quotation", "citation"],
    },
    PageCommand {
        target: PageCommandTarget::Code,
        label: "Code",
        shortcut: Some("```"),
        keywords: &["code", "snippet", "programming", "syntax"],
    },
    PageCommand {
        target: PageCommandTarget::Divider,
        label: "Divider",
        shortcut: Some("---"),
        keywords: &["divider", "horizontal rule", "separator", "line"],
    },
];
