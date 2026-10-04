use gpui_components::text_input::TextInputWrapMode;

use crate::model::{CardPageBlockKind, CardPageCodeWrap, CardPageEditableBlock};

#[derive(Clone, Copy)]
pub(super) enum PageBlockInputBehavior {
    Document,
    PageLink,
    Code { wrap_mode: TextInputWrapMode },
}

impl PageBlockInputBehavior {
    pub(super) fn from_editable(editable: &CardPageEditableBlock) -> Self {
        match editable.kind {
            CardPageBlockKind::Code => Self::Code {
                wrap_mode: code_text_input_wrap_mode(
                    editable
                        .code_wrap()
                        .expect("Code block input behavior requires Code block details"),
                ),
            },
            CardPageBlockKind::PageLink => Self::PageLink,
            CardPageBlockKind::Text
            | CardPageBlockKind::SubHeader
            | CardPageBlockKind::SubSubHeader
            | CardPageBlockKind::Heading3
            | CardPageBlockKind::Heading4
            | CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList
            | CardPageBlockKind::Callout
            | CardPageBlockKind::Quote => Self::Document,
        }
    }
}

const fn code_text_input_wrap_mode(wrap: CardPageCodeWrap) -> TextInputWrapMode {
    match wrap {
        CardPageCodeWrap::NoWrap => TextInputWrapMode::NoWrap,
        CardPageCodeWrap::Wrap => TextInputWrapMode::SoftWrap,
    }
}
