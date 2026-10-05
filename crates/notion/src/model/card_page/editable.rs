use serde::{Deserialize, Serialize};

use super::CardPageTextAnnotationSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageBlockKind {
    Text,
    SubHeader,
    SubSubHeader,
    Heading3,
    Heading4,
    BulletedList,
    NumberedList,
    ToDoList,
    ToggleList,
    PageLink,
    Callout,
    Quote,
    Code,
}

const NOTION_DEFAULT_CODE_LANGUAGE: &str = "JavaScript";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageCodeLanguage(String);

impl CardPageCodeLanguage {
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn from_api_value(value: &str) -> Result<Self, String> {
        Self::try_from(value.to_string())
    }
}

impl Default for CardPageCodeLanguage {
    fn default() -> Self {
        Self(NOTION_DEFAULT_CODE_LANGUAGE.to_string())
    }
}

impl TryFrom<String> for CardPageCodeLanguage {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("Notion code language must not be empty or whitespace".to_string());
        }
        Ok(Self(value))
    }
}

impl From<CardPageCodeLanguage> for String {
    fn from(language: CardPageCodeLanguage) -> Self {
        language.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageCodeWrap {
    #[default]
    NoWrap,
    Wrap,
}

impl CardPageCodeWrap {
    pub(crate) const fn from_api_value(value: bool) -> Self {
        if value {
            Self::Wrap
        } else {
            Self::NoWrap
        }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Wrap)
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::NoWrap => Self::Wrap,
            Self::Wrap => Self::NoWrap,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPageCodeSettings {
    language: CardPageCodeLanguage,
    wrap: CardPageCodeWrap,
}

impl CardPageCodeSettings {
    pub const fn new(language: CardPageCodeLanguage, wrap: CardPageCodeWrap) -> Self {
        Self { language, wrap }
    }

    pub const fn language(&self) -> &CardPageCodeLanguage {
        &self.language
    }

    pub const fn wrap(&self) -> CardPageCodeWrap {
        self.wrap
    }

    pub fn with_language(mut self, language: CardPageCodeLanguage) -> Self {
        self.language = language;
        self
    }

    pub const fn with_wrap(mut self, wrap: CardPageCodeWrap) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Default for CardPageCodeSettings {
    fn default() -> Self {
        Self::new(CardPageCodeLanguage::default(), CardPageCodeWrap::default())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageToDoState {
    #[default]
    Unchecked,
    Checked,
}

impl CardPageToDoState {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Unchecked => Self::Checked,
            Self::Checked => Self::Unchecked,
        }
    }

    pub const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageQuoteSize {
    #[default]
    Default,
    Large,
}

impl CardPageQuoteSize {
    pub const fn api_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Large => Some("large"),
        }
    }

    pub(crate) fn from_api_value(value: Option<&str>) -> Result<Self, String> {
        match value {
            None => Ok(Self::Default),
            Some("large") => Ok(Self::Large),
            Some(value) => Err(format!("unsupported Notion quote size {value}")),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "block_details", rename_all = "snake_case")]
enum CardPageEditableBlockDetails {
    #[default]
    Standard,
    ToDo {
        state: CardPageToDoState,
    },
    Quote {
        size: CardPageQuoteSize,
    },
    Code {
        language: CardPageCodeLanguage,
        wrap: CardPageCodeWrap,
    },
}

/// Why an editable block refuses text input in notnotion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageEditableReadOnlyReason {
    /// The title carries an inline token (a mention kind or an equation) that
    /// notnotion cannot round-trip yet; editing would rewrite it as plain text.
    UnsupportedInlineToken,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageEditableBlock {
    pub kind: CardPageBlockKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<CardPageTextAnnotationSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<CardPageEditableReadOnlyReason>,
    #[serde(default, flatten)]
    details: CardPageEditableBlockDetails,
}

impl CardPageEditableBlock {
    pub fn new(
        kind: CardPageBlockKind,
        text: impl Into<String>,
        annotations: Vec<CardPageTextAnnotationSpan>,
    ) -> Self {
        let details = default_details(kind);
        Self {
            kind,
            text: text.into(),
            annotations,
            read_only: None,
            details,
        }
    }

    pub fn to_do(
        text: impl Into<String>,
        annotations: Vec<CardPageTextAnnotationSpan>,
        state: CardPageToDoState,
    ) -> Self {
        Self {
            kind: CardPageBlockKind::ToDoList,
            text: text.into(),
            annotations,
            read_only: None,
            details: CardPageEditableBlockDetails::ToDo { state },
        }
    }

    pub(crate) fn quote(
        text: impl Into<String>,
        annotations: Vec<CardPageTextAnnotationSpan>,
        size: CardPageQuoteSize,
    ) -> Self {
        Self {
            kind: CardPageBlockKind::Quote,
            text: text.into(),
            annotations,
            read_only: None,
            details: CardPageEditableBlockDetails::Quote { size },
        }
    }

    pub(crate) fn code(
        text: impl Into<String>,
        annotations: Vec<CardPageTextAnnotationSpan>,
        language: CardPageCodeLanguage,
        wrap: CardPageCodeWrap,
    ) -> Self {
        Self {
            kind: CardPageBlockKind::Code,
            text: text.into(),
            annotations,
            read_only: None,
            details: CardPageEditableBlockDetails::Code { language, wrap },
        }
    }

    pub(crate) fn code_with_settings(
        text: impl Into<String>,
        annotations: Vec<CardPageTextAnnotationSpan>,
        settings: CardPageCodeSettings,
    ) -> Self {
        Self::code(text, annotations, settings.language, settings.wrap)
    }

    pub const fn to_do_state(&self) -> Option<CardPageToDoState> {
        match &self.details {
            CardPageEditableBlockDetails::ToDo { state } => Some(*state),
            CardPageEditableBlockDetails::Standard
            | CardPageEditableBlockDetails::Quote { .. }
            | CardPageEditableBlockDetails::Code { .. } => None,
        }
    }

    pub const fn quote_size(&self) -> Option<CardPageQuoteSize> {
        match &self.details {
            CardPageEditableBlockDetails::Quote { size } => Some(*size),
            CardPageEditableBlockDetails::Standard
            | CardPageEditableBlockDetails::ToDo { .. }
            | CardPageEditableBlockDetails::Code { .. } => None,
        }
    }

    pub const fn code_language(&self) -> Option<&CardPageCodeLanguage> {
        match &self.details {
            CardPageEditableBlockDetails::Code { language, .. } => Some(language),
            CardPageEditableBlockDetails::Standard
            | CardPageEditableBlockDetails::ToDo { .. }
            | CardPageEditableBlockDetails::Quote { .. } => None,
        }
    }

    pub const fn code_wrap(&self) -> Option<CardPageCodeWrap> {
        match &self.details {
            CardPageEditableBlockDetails::Code { wrap, .. } => Some(*wrap),
            CardPageEditableBlockDetails::Standard
            | CardPageEditableBlockDetails::ToDo { .. }
            | CardPageEditableBlockDetails::Quote { .. } => None,
        }
    }

    pub fn code_settings(&self) -> Option<CardPageCodeSettings> {
        Some(CardPageCodeSettings::new(
            self.code_language()?.clone(),
            self.code_wrap()?,
        ))
    }

    pub fn set_to_do_state(&mut self, state: CardPageToDoState) -> bool {
        let CardPageEditableBlockDetails::ToDo {
            state: current_state,
        } = &mut self.details
        else {
            return false;
        };
        *current_state = state;
        true
    }

    pub fn set_quote_size(&mut self, size: CardPageQuoteSize) -> bool {
        let CardPageEditableBlockDetails::Quote { size: current_size } = &mut self.details else {
            return false;
        };
        *current_size = size;
        true
    }

    pub fn set_code_language(&mut self, language: CardPageCodeLanguage) -> bool {
        let CardPageEditableBlockDetails::Code {
            language: current_language,
            ..
        } = &mut self.details
        else {
            return false;
        };
        *current_language = language;
        true
    }

    pub fn set_code_wrap(&mut self, wrap: CardPageCodeWrap) -> bool {
        let CardPageEditableBlockDetails::Code {
            wrap: current_wrap, ..
        } = &mut self.details
        else {
            return false;
        };
        *current_wrap = wrap;
        true
    }

    pub fn set_kind(&mut self, kind: CardPageBlockKind) {
        if self.kind == kind {
            return;
        }
        self.kind = kind;
        self.details = default_details(kind);
    }
}

fn default_details(kind: CardPageBlockKind) -> CardPageEditableBlockDetails {
    match kind {
        CardPageBlockKind::ToDoList => CardPageEditableBlockDetails::ToDo {
            state: CardPageToDoState::Unchecked,
        },
        CardPageBlockKind::Quote => CardPageEditableBlockDetails::Quote {
            size: CardPageQuoteSize::Default,
        },
        CardPageBlockKind::Code => CardPageEditableBlockDetails::Code {
            language: CardPageCodeLanguage::default(),
            wrap: CardPageCodeWrap::default(),
        },
        _ => CardPageEditableBlockDetails::Standard,
    }
}
