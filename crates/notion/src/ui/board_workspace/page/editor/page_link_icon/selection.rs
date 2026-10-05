use std::sync::Arc;

use gpui::RenderImage;

use super::{
    render::{
        page_link_picker_named_icon_item_count, PAGE_LINK_NAMED_ICON_CELL_SIZE,
        PAGE_LINK_NAMED_ICON_GRID_TOP,
    },
    PageLinkIconController, PageLinkIconPickerControlMenu, PageLinkIconPickerState,
    PageLinkIconPickerTab, PageLinkIconTarget, PageLinkNamedIconPreference,
    PageLinkNamedIconSelection,
};
use crate::{
    model::PageShellIcon,
    ui::{
        notion_named_icon_catalog, AppearanceMode, NotionNamedIconAsset, NotionNamedIconColor,
        NotionNamedIconSlug,
    },
};

pub(super) struct PageLinkIconSetEffect {
    pub(super) target: PageLinkIconTarget,
    pub(super) icon: Option<PageShellIcon>,
    pub(super) rendered: Option<Arc<RenderImage>>,
    pub(super) keep_picker_open: bool,
}

pub(super) enum PageLinkNamedIconChoice {
    Ignored,
    PickerChanged,
    SetIcon(PageLinkIconSetEffect),
}

impl PageLinkIconSetEffect {
    pub(super) fn direct(target: PageLinkIconTarget, icon: Option<PageShellIcon>) -> Self {
        Self {
            target,
            icon,
            rendered: None,
            keep_picker_open: false,
        }
    }
}

impl PageLinkIconController {
    pub(super) fn toggle_skin_tone_menu(&mut self) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Emoji {
            return false;
        }
        picker.control_menu = match picker.control_menu {
            Some(PageLinkIconPickerControlMenu::SkinTone) => None,
            _ => Some(PageLinkIconPickerControlMenu::SkinTone),
        };
        true
    }

    pub(super) fn set_skin_tone(&mut self, skin_tone: emojis::SkinTone) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        picker.skin_tone = skin_tone;
        picker.control_menu = None;
        true
    }

    pub(super) fn toggle_named_color_menu(&mut self) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Icons {
            return false;
        }
        picker.control_menu = match picker.control_menu {
            Some(PageLinkIconPickerControlMenu::NamedIconColor) => None,
            _ => Some(PageLinkIconPickerControlMenu::NamedIconColor),
        };
        true
    }

    pub(super) fn set_named_color(&mut self, color: NotionNamedIconColor) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        let preference = PageLinkNamedIconPreference::Color(color);
        picker.named_icon_preference = preference;
        picker.control_menu = None;
        self.named_icon_preference = preference;
        true
    }

    pub(super) fn toggle_ask_for_named_color(&mut self) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        let preference = match picker.named_icon_preference {
            PageLinkNamedIconPreference::AskEveryTime => {
                PageLinkNamedIconPreference::Color(NotionNamedIconColor::Gray)
            }
            PageLinkNamedIconPreference::Color(_) => PageLinkNamedIconPreference::AskEveryTime,
        };
        picker.named_icon_preference = preference;
        self.named_icon_preference = preference;
        true
    }

    pub(super) fn choose_named_icon(
        &mut self,
        selection: PageLinkNamedIconSelection,
        appearance_mode: AppearanceMode,
    ) -> PageLinkNamedIconChoice {
        let Some(picker) = self.picker.as_mut() else {
            return PageLinkNamedIconChoice::Ignored;
        };
        match picker.named_icon_preference {
            PageLinkNamedIconPreference::AskEveryTime => {
                let Some(row_bounds) = picker
                    .named_icon_list_state
                    .bounds_for_item(selection.list_item_index)
                else {
                    return PageLinkNamedIconChoice::Ignored;
                };
                let viewport_bounds = picker.named_icon_list_state.viewport_bounds();
                if viewport_bounds.size.width <= gpui::px(0.0)
                    || viewport_bounds.size.height <= gpui::px(0.0)
                {
                    return PageLinkNamedIconChoice::Ignored;
                }
                let anchor = gpui::point(
                    row_bounds.left() - viewport_bounds.left()
                        + gpui::px(
                            16.0 + selection.column_index as f32 * PAGE_LINK_NAMED_ICON_CELL_SIZE,
                        ),
                    row_bounds.bottom() - viewport_bounds.top()
                        + gpui::px(PAGE_LINK_NAMED_ICON_GRID_TOP),
                );
                picker.control_menu = Some(PageLinkIconPickerControlMenu::NamedIconChoice {
                    slug: selection.slug,
                    cell_key: selection.cell_key,
                    anchor,
                });
                PageLinkNamedIconChoice::PickerChanged
            }
            PageLinkNamedIconPreference::Color(color) => {
                PageLinkNamedIconChoice::SetIcon(named_icon_effect(
                    selection.target,
                    selection.slug,
                    color,
                    appearance_mode,
                    false,
                ))
            }
        }
    }

    pub(super) fn choose_named_icon_color(
        &self,
        target: PageLinkIconTarget,
        slug: NotionNamedIconSlug,
        color: NotionNamedIconColor,
        appearance_mode: AppearanceMode,
    ) -> PageLinkIconSetEffect {
        named_icon_effect(target, slug, color, appearance_mode, false)
    }

    pub(super) fn random_icon(
        &self,
        appearance_mode: AppearanceMode,
    ) -> Option<PageLinkIconSetEffect> {
        let picker = self.picker.as_ref()?;
        match picker.tab {
            PageLinkIconPickerTab::Emoji => {
                let catalog = emojis::iter().collect::<Vec<_>>();
                let emoji = catalog.get(random_page_icon_index(catalog.len()))?;
                let emoji = emoji.with_skin_tone(picker.skin_tone).unwrap_or(emoji);
                Some(PageLinkIconSetEffect {
                    target: PageLinkIconTarget {
                        page_id: picker.page_id.clone(),
                        block_id: picker.block_id.clone(),
                    },
                    icon: Some(PageShellIcon::emoji(emoji.as_str())),
                    rendered: None,
                    keep_picker_open: true,
                })
            }
            PageLinkIconPickerTab::Icons => {
                let catalog = notion_named_icon_catalog();
                let slug = catalog
                    .get(random_page_icon_index(catalog.len()))
                    .copied()?;
                Some(named_icon_effect(
                    PageLinkIconTarget {
                        page_id: picker.page_id.clone(),
                        block_id: picker.block_id.clone(),
                    },
                    slug,
                    picker.named_icon_preference.preview_color(),
                    appearance_mode,
                    true,
                ))
            }
            PageLinkIconPickerTab::Upload => None,
        }
    }

    pub(super) fn finish_applied_set_icon(
        &mut self,
        previous_picker: Option<PageLinkIconPickerState>,
        icon: Option<&PageShellIcon>,
        keep_picker_open: bool,
        appearance_mode: AppearanceMode,
    ) {
        self.picker = None;
        if let Some(slug) = icon
            .filter(|icon| icon.kind == "named")
            .and_then(|icon| NotionNamedIconAsset::parse(&icon.value, appearance_mode))
            .map(NotionNamedIconAsset::slug)
        {
            self.remember_named_icon(slug);
        }
        if !keep_picker_open {
            return;
        }
        let Some(mut picker) = previous_picker else {
            return;
        };
        picker.recent_named_icons = self
            .recent_named_icons
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .into();
        picker
            .named_icon_list_state
            .reset(page_link_picker_named_icon_item_count(
                picker.named_icon_matches.len(),
                picker.recent_named_icons.len(),
                picker.query.trim().is_empty(),
            ));
        self.picker = Some(picker);
    }
}

fn named_icon_effect(
    target: PageLinkIconTarget,
    slug: NotionNamedIconSlug,
    color: NotionNamedIconColor,
    appearance_mode: AppearanceMode,
    keep_picker_open: bool,
) -> PageLinkIconSetEffect {
    let icon = NotionNamedIconAsset::new(slug, color, appearance_mode);
    PageLinkIconSetEffect {
        target,
        icon: Some(PageShellIcon::named(icon.persisted_value())),
        rendered: None,
        keep_picker_open,
    }
}

/// Pick a stable bounded index without adding a dependency just for the picker.
fn random_page_icon_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    (nanos as usize) % len
}
