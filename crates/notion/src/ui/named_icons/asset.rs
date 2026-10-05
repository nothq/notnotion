use crate::ui::AppearanceMode;

use super::catalog::{notion_named_icon_catalog, notion_named_icon_label};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct NotionNamedIconSlug(&'static str);

impl NotionNamedIconSlug {
    pub(super) const fn from_catalog(slug: &'static str) -> Self {
        Self(slug)
    }

    pub(crate) const fn as_str(self) -> &'static str {
        self.0
    }

    pub(crate) fn label(self) -> String {
        notion_named_icon_label(self)
    }
}

impl std::fmt::Display for NotionNamedIconSlug {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) enum NotionNamedIconColor {
    #[default]
    Gray,
    LightGray,
    Brown,
    Yellow,
    Orange,
    Green,
    Blue,
    Purple,
    Pink,
    Red,
}

impl NotionNamedIconColor {
    pub(crate) const ALL: [Self; 10] = [
        Self::Gray,
        Self::LightGray,
        Self::Brown,
        Self::Yellow,
        Self::Orange,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Pink,
        Self::Red,
    ];

    pub(crate) const fn suffix(self) -> &'static str {
        match self {
            Self::Gray => "gray",
            Self::LightGray => "lightgray",
            Self::Brown => "brown",
            Self::Yellow => "yellow",
            Self::Orange => "orange",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
            Self::Red => "red",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Gray => "Default",
            Self::LightGray => "Light Gray",
            Self::Brown => "Brown",
            Self::Yellow => "Yellow",
            Self::Orange => "Orange",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Pink => "Pink",
            Self::Red => "Red",
        }
    }

    pub(crate) const fn picker_color(self, appearance_mode: AppearanceMode) -> u32 {
        match (self, appearance_mode) {
            (Self::Gray, AppearanceMode::Light) => 0x55534e,
            (Self::Gray, AppearanceMode::Dark) => 0xd3d3d3,
            (Self::LightGray, AppearanceMode::Light) => 0xa6a299,
            (Self::LightGray, AppearanceMode::Dark) => 0x7f7f7f,
            (Self::Brown, AppearanceMode::Light) => 0x9f6b53,
            (Self::Brown, AppearanceMode::Dark) => 0xaa755f,
            (Self::Yellow, AppearanceMode::Light) => 0xcb912f,
            (Self::Yellow, AppearanceMode::Dark) => 0xca8e1b,
            (Self::Orange, AppearanceMode::Light) => 0xd9730d,
            (Self::Orange, AppearanceMode::Dark) => 0xd87620,
            (Self::Green, AppearanceMode::Light) => 0x448361,
            (Self::Green, AppearanceMode::Dark) => 0x2d9964,
            (Self::Blue, AppearanceMode::Light) => 0x337ea9,
            (Self::Blue, AppearanceMode::Dark) => 0x2e7cd1,
            (Self::Purple, AppearanceMode::Light) => 0x9065b0,
            (Self::Purple, AppearanceMode::Dark) => 0x8d5bc1,
            (Self::Pink, AppearanceMode::Light) => 0xc14c8a,
            (Self::Pink, AppearanceMode::Dark) => 0xc94079,
            (Self::Red, AppearanceMode::Light) => 0xd44c47,
            (Self::Red, AppearanceMode::Dark) => 0xcd4945,
        }
    }

    pub(super) fn from_suffix(suffix: &str) -> Option<Self> {
        match suffix {
            "gray" => Some(Self::Gray),
            "lightgray" => Some(Self::LightGray),
            "brown" => Some(Self::Brown),
            "yellow" => Some(Self::Yellow),
            "orange" => Some(Self::Orange),
            "green" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "purple" => Some(Self::Purple),
            "pink" => Some(Self::Pink),
            "red" => Some(Self::Red),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum NotionNamedIconMode {
    Light,
    Dark,
}

impl NotionNamedIconMode {
    const fn query_value(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl From<AppearanceMode> for NotionNamedIconMode {
    fn from(value: AppearanceMode) -> Self {
        match value {
            AppearanceMode::Light => Self::Light,
            AppearanceMode::Dark => Self::Dark,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NotionNamedIconAsset {
    slug: NotionNamedIconSlug,
    color: NotionNamedIconColor,
    mode: NotionNamedIconMode,
}

impl NotionNamedIconAsset {
    pub(crate) fn new(
        slug: NotionNamedIconSlug,
        color: NotionNamedIconColor,
        appearance_mode: AppearanceMode,
    ) -> Self {
        Self {
            slug,
            color,
            mode: appearance_mode.into(),
        }
    }

    pub(crate) fn parse(value: &str, appearance_mode: AppearanceMode) -> Option<Self> {
        let value = value.strip_prefix("/icons/").unwrap_or(value);
        let value = value.strip_suffix(".svg").unwrap_or(value);
        let (slug, color) = value
            .rsplit_once('_')
            .and_then(|(slug, suffix)| {
                NotionNamedIconColor::from_suffix(suffix).map(|color| (slug, color))
            })
            .unwrap_or((value, NotionNamedIconColor::Gray));
        let index = notion_named_icon_catalog()
            .binary_search_by(|catalog_slug| catalog_slug.as_str().cmp(slug))
            .ok()?;
        Some(Self::new(
            notion_named_icon_catalog()[index],
            color,
            appearance_mode,
        ))
    }

    pub(crate) const fn slug(self) -> NotionNamedIconSlug {
        self.slug
    }

    pub(crate) fn persisted_value(self) -> String {
        format!("{}_{}", self.slug, self.color.suffix())
    }

    pub(super) fn url(self) -> String {
        format!(
            "https://app.notion.com/icons/{}_{}.svg?mode={}",
            self.slug,
            self.color.suffix(),
            self.mode.query_value()
        )
    }
}
