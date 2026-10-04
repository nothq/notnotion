use gpui::{AnyElement, InteractiveElement, IntoElement, Styled};

use crate::ui::surface::NotionStartup;
use crate::ui::{div, rgb, Theme};

impl NotionStartup {
    pub(crate) fn render_startup(&self, theme: Theme) -> Option<AnyElement> {
        match self {
            Self::Loading {
                cached: Some(_), ..
            }
            | Self::Error {
                cached: Some(_), ..
            } => None,
            Self::Loading { cached: None, .. } | Self::Error { cached: None, .. } => Some(
                div()
                    .id("notion-startup")
                    .size_full()
                    .bg(rgb(theme.app_bg))
                    .into_any_element(),
            ),
            Self::Ready(_) => None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => None,
        }
    }
}
