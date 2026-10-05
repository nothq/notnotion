use gpui::{App, Window};

use super::{
    commit::PreparedPageLinkIconSave,
    preview::PageLinkIconUploadTarget,
    selection::{PageLinkIconSetEffect, PageLinkNamedIconChoice},
    PageLinkIconAction, PageLinkIconControlAction, PageLinkIconController,
    PageLinkIconPickerAction, PageLinkIconSelectionAction, PageLinkIconUploadAction,
};
use crate::{model::PageShellIcon, ui::AppearanceMode};

pub(super) enum PageLinkIconControllerEffect {
    DismissPageBlockInteraction,
    SetIcon(PageLinkIconSetEffect),
    PasteUpload(PageLinkIconUploadTarget),
    PromptUpload(PageLinkIconUploadTarget),
    SaveUpload(PreparedPageLinkIconSave),
}

#[derive(Clone, Copy)]
pub(super) struct PageLinkIconActionContext {
    pub(super) appearance_mode: AppearanceMode,
    pub(super) page_mutation_idle: bool,
}

pub(super) struct PageLinkIconActionOutcome {
    pub(super) effect: Option<PageLinkIconControllerEffect>,
    pub(super) notify: bool,
}

impl PageLinkIconActionOutcome {
    fn ignored() -> Self {
        Self {
            effect: None,
            notify: false,
        }
    }

    fn changed(changed: bool) -> Self {
        Self {
            effect: None,
            notify: changed,
        }
    }

    fn effect(effect: PageLinkIconControllerEffect) -> Self {
        Self {
            effect: Some(effect),
            notify: false,
        }
    }

    fn effect_and_notify(effect: PageLinkIconControllerEffect) -> Self {
        Self {
            effect: Some(effect),
            notify: true,
        }
    }

    fn optional_effect(effect: Option<PageLinkIconControllerEffect>) -> Self {
        effect.map_or_else(Self::ignored, Self::effect)
    }
}

impl PageLinkIconController {
    pub(super) fn handle_action(
        &mut self,
        action: PageLinkIconAction,
        context: PageLinkIconActionContext,
        window: &mut Window,
        cx: &mut App,
    ) -> PageLinkIconActionOutcome {
        let PageLinkIconActionContext {
            appearance_mode,
            page_mutation_idle,
        } = context;
        match action {
            PageLinkIconAction::Picker(action) => self.handle_picker_action(action, cx),
            PageLinkIconAction::Controls(action) => self.handle_control_action(action),
            PageLinkIconAction::Selection(action) => {
                self.handle_selection_action(action, appearance_mode)
            }
            PageLinkIconAction::Upload(action) => {
                self.handle_upload_action(action, page_mutation_idle, window, cx)
            }
            PageLinkIconAction::DismissPageBlockInteraction => PageLinkIconActionOutcome::effect(
                PageLinkIconControllerEffect::DismissPageBlockInteraction,
            ),
        }
    }

    fn handle_picker_action(
        &mut self,
        action: PageLinkIconPickerAction,
        cx: &mut App,
    ) -> PageLinkIconActionOutcome {
        let changed = match action {
            PageLinkIconPickerAction::Dismiss => self.dismiss_picker(),
            PageLinkIconPickerAction::DismissLayer => self.dismiss_layer(),
            PageLinkIconPickerAction::SetQuery(query) => self.set_query(query),
            PageLinkIconPickerAction::SetTab(tab) => self.set_tab(tab, cx),
            PageLinkIconPickerAction::JumpCategory {
                section_index,
                category_index,
            } => self.jump_category(section_index, category_index),
            PageLinkIconPickerAction::SyncCategoryFromScroll(item_index) => {
                self.sync_category_from_scroll(item_index)
            }
        };
        PageLinkIconActionOutcome::changed(changed)
    }

    fn handle_control_action(
        &mut self,
        action: PageLinkIconControlAction,
    ) -> PageLinkIconActionOutcome {
        let changed = match action {
            PageLinkIconControlAction::ToggleSkinToneMenu => self.toggle_skin_tone_menu(),
            PageLinkIconControlAction::SetSkinTone(skin_tone) => self.set_skin_tone(skin_tone),
            PageLinkIconControlAction::ToggleNamedColorMenu => self.toggle_named_color_menu(),
            PageLinkIconControlAction::SetNamedColor(color) => self.set_named_color(color),
            PageLinkIconControlAction::ToggleAskForNamedColor => self.toggle_ask_for_named_color(),
        };
        PageLinkIconActionOutcome::changed(changed)
    }

    fn handle_selection_action(
        &mut self,
        action: PageLinkIconSelectionAction,
        appearance_mode: AppearanceMode,
    ) -> PageLinkIconActionOutcome {
        match action {
            PageLinkIconSelectionAction::ChooseNamedIcon(selection) => {
                self.select_named_icon(selection, appearance_mode)
            }
            PageLinkIconSelectionAction::ChooseNamedIconColor {
                target,
                slug,
                color,
            } => {
                PageLinkIconActionOutcome::effect_and_notify(PageLinkIconControllerEffect::SetIcon(
                    self.choose_named_icon_color(target, slug, color, appearance_mode),
                ))
            }
            PageLinkIconSelectionAction::SetRandomIcon => {
                PageLinkIconActionOutcome::optional_effect(
                    self.random_icon(appearance_mode)
                        .map(PageLinkIconControllerEffect::SetIcon),
                )
            }
            PageLinkIconSelectionAction::SetIcon { target, icon } => {
                PageLinkIconActionOutcome::effect(PageLinkIconControllerEffect::SetIcon(
                    PageLinkIconSetEffect::direct(target, icon),
                ))
            }
            PageLinkIconSelectionAction::SelectCustomEmoji {
                target,
                pointer,
                rendered,
            } => self.select_custom_emoji(target, pointer, rendered),
        }
    }

    fn handle_upload_action(
        &mut self,
        action: PageLinkIconUploadAction,
        page_mutation_idle: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> PageLinkIconActionOutcome {
        match action {
            PageLinkIconUploadAction::PasteUpload => {
                self.upload_target_effect(PageLinkIconControllerEffect::PasteUpload)
            }
            PageLinkIconUploadAction::PromptUpload => {
                self.upload_target_effect(PageLinkIconControllerEffect::PromptUpload)
            }
            PageLinkIconUploadAction::ClearUploadPreview => {
                PageLinkIconActionOutcome::changed(self.clear_upload_preview(cx))
            }
            PageLinkIconUploadAction::ToggleUploadLibrary => {
                PageLinkIconActionOutcome::changed(self.toggle_upload_library(window, cx))
            }
            PageLinkIconUploadAction::SetUploadName(value) => {
                PageLinkIconActionOutcome::changed(self.set_upload_name(value, cx))
            }
            PageLinkIconUploadAction::SaveUpload => PageLinkIconActionOutcome::optional_effect(
                self.prepare_save(page_mutation_idle)
                    .map(PageLinkIconControllerEffect::SaveUpload),
            ),
        }
    }

    fn select_named_icon(
        &mut self,
        selection: super::PageLinkNamedIconSelection,
        appearance_mode: AppearanceMode,
    ) -> PageLinkIconActionOutcome {
        match self.choose_named_icon(selection, appearance_mode) {
            PageLinkNamedIconChoice::Ignored => PageLinkIconActionOutcome::ignored(),
            PageLinkNamedIconChoice::PickerChanged => PageLinkIconActionOutcome::changed(true),
            PageLinkNamedIconChoice::SetIcon(effect) => {
                PageLinkIconActionOutcome::effect(PageLinkIconControllerEffect::SetIcon(effect))
            }
        }
    }

    fn select_custom_emoji(
        &self,
        target: super::PageLinkIconTarget,
        pointer: String,
        rendered: Option<std::sync::Arc<gpui::RenderImage>>,
    ) -> PageLinkIconActionOutcome {
        let icon = match PageShellIcon::custom(pointer) {
            Ok(icon) => icon,
            Err(error) => {
                println!("notnotion: cannot select custom emoji: {error}");
                return PageLinkIconActionOutcome::ignored();
            }
        };
        PageLinkIconActionOutcome::effect(PageLinkIconControllerEffect::SetIcon(
            PageLinkIconSetEffect {
                target,
                icon: Some(icon),
                rendered,
                keep_picker_open: false,
            },
        ))
    }

    fn upload_target_effect(
        &self,
        effect: fn(PageLinkIconUploadTarget) -> PageLinkIconControllerEffect,
    ) -> PageLinkIconActionOutcome {
        PageLinkIconActionOutcome::optional_effect(self.upload_prompt_target().map(effect))
    }
}
