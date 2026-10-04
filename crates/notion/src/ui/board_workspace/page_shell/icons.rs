use super::sidebar::notion_sidebar_panel_muted;
use super::{
    div, img, notion_sidebar_muted, px, relative, rgb, AnyElement, IntoElement, ParentElement,
    Styled,
};
use crate::ui::PageShellIcon;
use crate::ui::{notion_named_icon_value_has_color, NotionNamedIconAsset};
use gpui::{App, StyledImage};
use std::sync::Arc;

use crate::ui::{
    surface::NotionSurfaceResources, view_actions::ViewNotifier, AppearanceMode, IconSet,
};

mod host;

#[derive(Clone)]
pub(crate) struct PageShellIconRenderer {
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    resources: NotionSurfaceResources,
    notifier: ViewNotifier,
}

impl PageShellIconRenderer {
    pub(crate) fn new(
        appearance_mode: AppearanceMode,
        icons: Arc<IconSet>,
        resources: NotionSurfaceResources,
        notifier: ViewNotifier,
    ) -> Self {
        Self {
            appearance_mode,
            icons,
            resources,
            notifier,
        }
    }

    pub(crate) fn render(&self, icon: &PageShellIcon, size: f32, cx: &mut App) -> AnyElement {
        match icon.kind.as_str() {
            "emoji" => div()
                .w(px(size.max(14.0)))
                .text_size(px(size.max(12.0)))
                .line_height(relative(1.0))
                .child(icon.value.clone())
                .into_any_element(),
            "named" => self.render_named(&icon.value, size, cx),
            "external" | "custom" => {
                let mut icon_container = div().size(px(size)).overflow_hidden();
                if let Some(rendered) =
                    self.resources
                        .external_icon_image(icon.render_value(), &self.notifier, cx)
                {
                    icon_container = icon_container.child(
                        img(rendered)
                            .size(px(size))
                            .object_fit(gpui::ObjectFit::Cover),
                    );
                }
                icon_container.into_any_element()
            }
            _ => img(self.icons.page.render(cx))
                .size(px(size))
                .into_any_element(),
        }
    }

    fn render_named(&self, value: &str, size: f32, cx: &mut App) -> AnyElement {
        let explicit_catalog_icon = notion_named_icon_value_has_color(value);
        let base_name = named_icon_base_name(value);
        if !explicit_catalog_icon {
            if let Some(icon_name) = notion_page_shell_builtin_icon_name(base_name) {
                return self.builtin(icon_name, size, cx);
            }
        }
        if let Some(asset) = NotionNamedIconAsset::parse(value, self.appearance_mode) {
            return self.render_named_asset(asset, size, cx);
        }
        match base_name {
            "calendar-event-current" => div()
                .size(px(size))
                .rounded(px(1.5))
                .bg(rgb(0x5e9fe8))
                .into_any_element(),
            "calendar-event-upcoming" => div()
                .size(px(size))
                .rounded(px(1.5))
                .border_1()
                .border_dashed()
                .border_color(rgb(0x5e9fe8))
                .into_any_element(),
            "panel-add" | "panel-open" => self.render_panel(base_name, size),
            "more" => img(self.icons.sidebar.more.render(cx))
                .size(px(size))
                .into_any_element(),
            "page" => img(self.icons.page.render(cx))
                .size(px(size))
                .into_any_element(),
            "database" => img(self.icons.view_table_inactive.render(cx))
                .size(px(size + 2.0))
                .into_any_element(),
            "service-counter" => img(self.icons.sidebar.service_counter.render(cx))
                .size(px(size))
                .into_any_element(),
            "government" => img(self.icons.sidebar.government.render(cx))
                .size(px(size))
                .into_any_element(),
            "archive" => img(self.icons.folder.render(cx))
                .size(px(size))
                .into_any_element(),
            _ => self.glyph(base_name, size),
        }
    }

    fn render_named_asset(
        &self,
        asset: NotionNamedIconAsset,
        size: f32,
        cx: &mut App,
    ) -> AnyElement {
        let mut icon = div().size(px(size)).flex().items_center().justify_center();
        if let Some(rendered) = self.resources.named_icon_image(asset, &self.notifier, cx) {
            icon = icon.child(img(rendered).size(px(size)));
        }
        icon.into_any_element()
    }

    fn render_panel(&self, value: &str, size: f32) -> AnyElement {
        div()
            .w(px(size.max(14.0)))
            .text_size(px((size - 1.0).max(12.0)))
            .line_height(relative(1.0))
            .text_color(rgb(notion_sidebar_panel_muted(self.appearance_mode)))
            .child(if value == "panel-add" { "+" } else { "↗" })
            .into_any_element()
    }

    pub(crate) fn sidebar_builtin(&self, icon_name: &str, cx: &mut App) -> AnyElement {
        self.builtin(icon_name, 14.0, cx)
    }

    pub(crate) fn builtin(&self, icon_name: &str, size: f32, cx: &mut App) -> AnyElement {
        let cached_icon = match icon_name {
            "search" => Some(&self.icons.sidebar.search),
            "home" => Some(&self.icons.sidebar.home),
            "chat" => Some(&self.icons.sidebar.chat),
            "inbox" => Some(&self.icons.sidebar.inbox),
            "new-chat" => Some(&self.icons.sidebar.new_chat),
            "new-page" => Some(&self.icons.sidebar.new_page),
            "notion-ai" => Some(&self.icons.sidebar.notion_ai),
            "add" => Some(&self.icons.sidebar.add),
            "more" => Some(&self.icons.sidebar.more),
            "page" => Some(&self.icons.page),
            _ => None,
        };
        if let Some(icon) = cached_icon {
            return img(icon.render(cx)).size(px(size)).into_any_element();
        }
        match icon_name {
            "meetings" => img(self.icons.property_date.render(cx))
                .size(px(size))
                .into_any_element(),
            "library" => img(self.icons.folder.render(cx))
                .size(px(size))
                .into_any_element(),
            _ => self.glyph(icon_name, size),
        }
    }

    pub(crate) fn glyph(&self, value: &str, size: f32) -> AnyElement {
        render_named_glyph(value, size, self.appearance_mode)
    }

    pub(crate) fn sidebar_chevron(&self, expanded: bool, size: f32, cx: &mut App) -> AnyElement {
        let icon = if expanded {
            &self.icons.sidebar.chevron_expanded
        } else {
            &self.icons.sidebar.chevron_collapsed
        };
        img(icon.render(cx)).size(px(size)).into_any_element()
    }
}

fn render_named_glyph(value: &str, size: f32, appearance_mode: AppearanceMode) -> AnyElement {
    let glyph = match value {
        "goal" | "sprint-goals" => "✦",
        "shopping-bag" => "◪",
        "wallet" => "$",
        "library" => "▥",
        "gavel" => "⚖",
        "building" | "government" => "▣",
        "runner" => "↗",
        _ => "•",
    };
    div()
        .w(px(size.max(14.0)))
        .text_size(px((size - 1.0).max(12.0)))
        .line_height(relative(1.0))
        .text_color(rgb(notion_sidebar_muted(appearance_mode)))
        .child(glyph)
        .into_any_element()
}

fn named_icon_base_name(value: &str) -> &str {
    let value = value.strip_prefix("/icons/").unwrap_or(value);
    let value = value.strip_suffix(".svg").unwrap_or(value);
    let Some((base, color)) = value.rsplit_once('_') else {
        return value;
    };
    if matches!(
        color,
        "gray"
            | "lightgray"
            | "brown"
            | "orange"
            | "yellow"
            | "green"
            | "blue"
            | "purple"
            | "pink"
            | "red"
    ) {
        base
    } else {
        value
    }
}

fn notion_page_shell_builtin_icon_name(value: &str) -> Option<&'static str> {
    match value {
        "search" => Some("search"),
        "home" => Some("home"),
        "chat" => Some("chat"),
        "meetings" => Some("meetings"),
        "notion-ai" => Some("notion-ai"),
        "inbox" => Some("inbox"),
        "library" => Some("library"),
        _ => None,
    }
}
