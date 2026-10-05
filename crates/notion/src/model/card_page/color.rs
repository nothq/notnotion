use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageBlockColorValue {
    #[default]
    Default,
    Gray,
    Brown,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Red,
}

impl CardPageBlockColorValue {
    pub const ALL: [Self; 10] = [
        Self::Default,
        Self::Gray,
        Self::Brown,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Pink,
        Self::Red,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Gray => "Gray",
            Self::Brown => "Brown",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Pink => "Pink",
            Self::Red => "Red",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Default => 0,
            Self::Gray => 1,
            Self::Brown => 2,
            Self::Orange => 3,
            Self::Yellow => 4,
            Self::Green => 5,
            Self::Blue => 6,
            Self::Purple => 7,
            Self::Pink => 8,
            Self::Red => 9,
        }
    }

    const fn api_value(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Gray => "gray",
            Self::Brown => "brown",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "teal",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
            Self::Red => "red",
        }
    }

    fn from_api_value(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "gray" => Some(Self::Gray),
            "brown" => Some(Self::Brown),
            "orange" => Some(Self::Orange),
            "yellow" => Some(Self::Yellow),
            "green" | "teal" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "purple" => Some(Self::Purple),
            "pink" => Some(Self::Pink),
            "red" => Some(Self::Red),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "color_mode", content = "color", rename_all = "snake_case")]
pub enum CardPageBlockColor {
    Text(CardPageBlockColorValue),
    Background(CardPageBlockColorValue),
}

impl Default for CardPageBlockColor {
    fn default() -> Self {
        Self::Text(CardPageBlockColorValue::Default)
    }
}

impl CardPageBlockColor {
    pub const fn text(value: CardPageBlockColorValue) -> Self {
        Self::Text(value)
    }

    pub const fn background(value: CardPageBlockColorValue) -> Self {
        Self::Background(value)
    }

    pub const fn value(self) -> CardPageBlockColorValue {
        match self {
            Self::Text(value) | Self::Background(value) => value,
        }
    }

    pub const fn is_background(self) -> bool {
        matches!(self, Self::Background(_))
    }

    pub const fn api_value(self) -> &'static str {
        match self {
            Self::Text(value) => value.api_value(),
            Self::Background(CardPageBlockColorValue::Default) => "default_background",
            Self::Background(CardPageBlockColorValue::Gray) => "gray_background",
            Self::Background(CardPageBlockColorValue::Brown) => "brown_background",
            Self::Background(CardPageBlockColorValue::Orange) => "orange_background",
            Self::Background(CardPageBlockColorValue::Yellow) => "yellow_background",
            Self::Background(CardPageBlockColorValue::Green) => "teal_background",
            Self::Background(CardPageBlockColorValue::Blue) => "blue_background",
            Self::Background(CardPageBlockColorValue::Purple) => "purple_background",
            Self::Background(CardPageBlockColorValue::Pink) => "pink_background",
            Self::Background(CardPageBlockColorValue::Red) => "red_background",
        }
    }

    pub(crate) fn from_api_value(value: Option<&str>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        if let Some(text) = CardPageBlockColorValue::from_api_value(value) {
            return Ok(Self::Text(text));
        }
        let Some(background) = value.strip_suffix("_background") else {
            return Err(format!("unsupported Notion block color {value}"));
        };
        CardPageBlockColorValue::from_api_value(background)
            .map(Self::Background)
            .ok_or_else(|| format!("unsupported Notion block color {value}"))
    }
}
