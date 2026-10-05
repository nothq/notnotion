use crate::model::{ConvertPageBlockRequest, NotionPageBlockKind};
use crate::ui::{CardPageBlock, CardPageBlockKind, CardPageStructuralBlock};

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page::editor) enum VerifiedPageTextBlockKind {
    Text,
    SubHeader,
    SubSubHeader,
    Heading3,
    Heading4,
    BulletedList,
    NumberedList,
    ToDoList,
    ToggleList,
    Callout,
    Quote,
}

impl VerifiedPageTextBlockKind {
    pub(in crate::ui::board_workspace::page::editor) const fn parse(
        kind: CardPageBlockKind,
    ) -> Option<Self> {
        match kind {
            CardPageBlockKind::Text => Some(Self::Text),
            CardPageBlockKind::SubHeader => Some(Self::SubHeader),
            CardPageBlockKind::SubSubHeader => Some(Self::SubSubHeader),
            CardPageBlockKind::Heading3 => Some(Self::Heading3),
            CardPageBlockKind::Heading4 => Some(Self::Heading4),
            CardPageBlockKind::BulletedList => Some(Self::BulletedList),
            CardPageBlockKind::NumberedList => Some(Self::NumberedList),
            CardPageBlockKind::ToDoList => Some(Self::ToDoList),
            CardPageBlockKind::ToggleList => Some(Self::ToggleList),
            CardPageBlockKind::Callout => Some(Self::Callout),
            CardPageBlockKind::Quote => Some(Self::Quote),
            CardPageBlockKind::PageLink | CardPageBlockKind::Code => None,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) const fn text() -> Self {
        Self::Text
    }

    pub(in crate::ui::board_workspace::page::editor) const fn card_kind(self) -> CardPageBlockKind {
        match self {
            Self::Text => CardPageBlockKind::Text,
            Self::SubHeader => CardPageBlockKind::SubHeader,
            Self::SubSubHeader => CardPageBlockKind::SubSubHeader,
            Self::Heading3 => CardPageBlockKind::Heading3,
            Self::Heading4 => CardPageBlockKind::Heading4,
            Self::BulletedList => CardPageBlockKind::BulletedList,
            Self::NumberedList => CardPageBlockKind::NumberedList,
            Self::ToDoList => CardPageBlockKind::ToDoList,
            Self::ToggleList => CardPageBlockKind::ToggleList,
            Self::Callout => CardPageBlockKind::Callout,
            Self::Quote => CardPageBlockKind::Quote,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) const fn notion_kind(
        self,
    ) -> NotionPageBlockKind {
        match self {
            Self::Text => NotionPageBlockKind::Text,
            Self::SubHeader => NotionPageBlockKind::Header,
            Self::SubSubHeader => NotionPageBlockKind::SubHeader,
            Self::Heading3 => NotionPageBlockKind::SubSubHeader,
            Self::Heading4 => NotionPageBlockKind::Header4,
            Self::BulletedList => NotionPageBlockKind::BulletedList,
            Self::NumberedList => NotionPageBlockKind::NumberedList,
            Self::ToDoList => NotionPageBlockKind::ToDo,
            Self::ToggleList => NotionPageBlockKind::Toggle,
            Self::Callout => NotionPageBlockKind::Callout,
            Self::Quote => NotionPageBlockKind::Quote,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) const fn split_kind(self) -> Self {
        match self {
            Self::BulletedList => Self::BulletedList,
            Self::NumberedList => Self::NumberedList,
            Self::ToDoList => Self::ToDoList,
            Self::ToggleList => Self::ToggleList,
            Self::Text
            | Self::SubHeader
            | Self::SubSubHeader
            | Self::Heading3
            | Self::Heading4
            | Self::Callout
            | Self::Quote => Self::Text,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn conversion_request(
        self,
        block_id: String,
    ) -> ConvertPageBlockRequest {
        ConvertPageBlockRequest {
            block_id,
            kind: self.notion_kind(),
        }
    }
}

pub(in crate::ui::board_workspace::page::editor) const fn notion_page_block_kind(
    kind: CardPageBlockKind,
) -> NotionPageBlockKind {
    match kind {
        CardPageBlockKind::Text => NotionPageBlockKind::Text,
        CardPageBlockKind::SubHeader => NotionPageBlockKind::Header,
        CardPageBlockKind::SubSubHeader => NotionPageBlockKind::SubHeader,
        CardPageBlockKind::Heading3 => NotionPageBlockKind::SubSubHeader,
        CardPageBlockKind::Heading4 => NotionPageBlockKind::Header4,
        CardPageBlockKind::BulletedList => NotionPageBlockKind::BulletedList,
        CardPageBlockKind::NumberedList => NotionPageBlockKind::NumberedList,
        CardPageBlockKind::ToDoList => NotionPageBlockKind::ToDo,
        CardPageBlockKind::ToggleList => NotionPageBlockKind::Toggle,
        CardPageBlockKind::PageLink => NotionPageBlockKind::Page,
        CardPageBlockKind::Callout => NotionPageBlockKind::Callout,
        CardPageBlockKind::Quote => NotionPageBlockKind::Quote,
        CardPageBlockKind::Code => NotionPageBlockKind::Code,
    }
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_creation(
    block: &CardPageBlock,
) -> Option<(NotionPageBlockKind, &str)> {
    if let Some(editable) = block.editable_content() {
        if editable.kind == CardPageBlockKind::Code {
            return None;
        }
        return Some((
            notion_page_block_kind(editable.kind),
            editable.text.as_str(),
        ));
    }
    match block.structural_content()? {
        CardPageStructuralBlock::Divider => Some((NotionPageBlockKind::Divider, "")),
        CardPageStructuralBlock::CollectionView { .. }
        | CardPageStructuralBlock::CollectionViewPage { .. } => None,
    }
}
