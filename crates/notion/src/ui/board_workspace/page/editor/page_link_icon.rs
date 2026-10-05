use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, OnceLock},
};

use gpui::{App, Pixels, Point, RenderImage, Window};
use gpui_components::text_input::TextInput;

use crate::{
    model::{NotionCustomEmoji, PageShellIcon, PreparedPageIconUpload},
    ui::{
        surface::NotionSurfaceResources,
        view_actions::{ViewActionSink, ViewNotifier},
        AppearanceMode, IconSet, NotionNamedIconColor, NotionNamedIconSlug, Theme,
    },
};

mod actions;
mod commit;
mod host;
mod persistence;
mod picker;
mod preview;
mod render;
mod selection;
mod session;

/// Picker instance and generation of each in-flight commit, keyed by page and block id.
type PageLinkIconCommitsInFlight = HashMap<(String, String), (u64, u64)>;

#[derive(Default)]
pub(crate) struct PageLinkIconController {
    pub(super) picker: Option<PageLinkIconPickerState>,
    next_picker_instance_id: u64,
    commits_in_flight: PageLinkIconCommitsInFlight,
    recent_named_icons: VecDeque<NotionNamedIconSlug>,
    named_icon_preference: PageLinkNamedIconPreference,
}

impl PageLinkIconController {
    pub(crate) fn inherit_session_state(&mut self, previous: &Self) {
        self.next_picker_instance_id = previous.next_picker_instance_id;
        self.commits_in_flight = previous.commits_in_flight.clone();
        self.recent_named_icons = previous.recent_named_icons.clone();
        self.named_icon_preference = previous.named_icon_preference;
    }

    pub(crate) fn take_cached_state_from(&mut self, previous: &mut Self) {
        self.picker = previous.picker.take();
    }

    pub(crate) fn reset_for_route(&mut self) {
        self.picker = None;
    }

    pub(crate) fn picker_is_open(&self) -> bool {
        self.picker.is_some()
    }

    pub(crate) fn picker(&self) -> Option<&PageLinkIconPickerState> {
        self.picker.as_ref()
    }

    pub(crate) fn dismiss_picker(&mut self) -> bool {
        if self
            .picker
            .as_ref()
            .is_some_and(|picker| picker.upload_committed)
        {
            return false;
        }
        self.picker.take().is_some()
    }

    pub(crate) fn clear_picker(&mut self) {
        self.picker = None;
    }

    pub(crate) fn commit_in_flight(&self, page_id: &str, block_id: &str) -> bool {
        self.commits_in_flight
            .keys()
            .any(|(pending_page_id, pending_block_id)| {
                pending_page_id == page_id && pending_block_id == block_id
            })
    }

    pub(crate) fn commit_in_flight_for_page(&self, page_id: &str) -> bool {
        self.commits_in_flight
            .keys()
            .any(|(pending_page_id, _)| pending_page_id == page_id)
    }

    fn begin_commit(
        &mut self,
        page_id: &str,
        block_id: &str,
        instance_id: u64,
        generation: u64,
    ) -> bool {
        if self.commit_in_flight(page_id, block_id) {
            return false;
        }
        self.commits_in_flight.insert(
            (page_id.to_owned(), block_id.to_owned()),
            (instance_id, generation),
        );
        true
    }

    fn finish_commit(
        &mut self,
        page_id: &str,
        block_id: &str,
        instance_id: u64,
        generation: u64,
    ) -> bool {
        let target = (page_id.to_owned(), block_id.to_owned());
        if self.commits_in_flight.get(&target) != Some(&(instance_id, generation)) {
            return false;
        }
        self.commits_in_flight.remove(&target);
        true
    }

    fn remember_named_icon(&mut self, slug: NotionNamedIconSlug) {
        self.recent_named_icons.retain(|recent| recent != &slug);
        self.recent_named_icons.push_front(slug);
        self.recent_named_icons.truncate(12);
    }
}

pub(super) enum PageLinkIconAction {
    Picker(PageLinkIconPickerAction),
    Controls(PageLinkIconControlAction),
    Selection(PageLinkIconSelectionAction),
    Upload(PageLinkIconUploadAction),
    DismissPageBlockInteraction,
}

pub(super) enum PageLinkIconPickerAction {
    Dismiss,
    DismissLayer,
    SetQuery(String),
    SetTab(PageLinkIconPickerTab),
    JumpCategory {
        section_index: usize,
        category_index: usize,
    },
    SyncCategoryFromScroll(usize),
}

pub(super) enum PageLinkIconControlAction {
    ToggleSkinToneMenu,
    SetSkinTone(emojis::SkinTone),
    ToggleNamedColorMenu,
    SetNamedColor(NotionNamedIconColor),
    ToggleAskForNamedColor,
}

pub(super) enum PageLinkIconSelectionAction {
    ChooseNamedIcon(PageLinkNamedIconSelection),
    ChooseNamedIconColor {
        target: PageLinkIconTarget,
        slug: NotionNamedIconSlug,
        color: NotionNamedIconColor,
    },
    SetRandomIcon,
    SetIcon {
        target: PageLinkIconTarget,
        icon: Option<PageShellIcon>,
    },
    SelectCustomEmoji {
        target: PageLinkIconTarget,
        pointer: String,
        rendered: Option<Arc<RenderImage>>,
    },
}

pub(super) enum PageLinkIconUploadAction {
    PasteUpload,
    PromptUpload,
    ClearUploadPreview,
    ToggleUploadLibrary,
    SetUploadName(String),
    SaveUpload,
}

#[derive(Clone)]
pub(super) struct PageLinkIconView {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) resources: NotionSurfaceResources,
    pub(super) notifier: ViewNotifier,
    pub(super) actions: ViewActionSink<PageLinkIconAction>,
    pub(super) page_mutation_idle: bool,
}

impl PageLinkIconView {
    fn listener<Event: ?Sized>(
        &self,
        handler: impl Fn(&Self, &Event, &mut Window, &mut App) + 'static,
    ) -> impl Fn(&Event, &mut Window, &mut App) + 'static {
        let view = self.clone();
        move |event, window, cx| handler(&view, event, window, cx)
    }

    fn emit(&self, action: PageLinkIconAction, window: &mut Window, cx: &mut App) {
        self.actions.emit(action, window, cx);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum PageLinkIconPickerTab {
    #[default]
    Emoji,
    Icons,
    Upload,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PageLinkIconPickerControlMenu {
    SkinTone,
    NamedIconColor,
    NamedIconChoice {
        slug: NotionNamedIconSlug,
        cell_key: usize,
        anchor: Point<Pixels>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ui::board_workspace::page::editor) struct PageLinkIconTarget {
    pub(in crate::ui::board_workspace::page::editor) page_id: String,
    pub(in crate::ui::board_workspace::page::editor) block_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ui::board_workspace::page::editor) struct PageLinkNamedIconSelection {
    pub(in crate::ui::board_workspace::page::editor) target: PageLinkIconTarget,
    pub(in crate::ui::board_workspace::page::editor) slug: NotionNamedIconSlug,
    pub(in crate::ui::board_workspace::page::editor) cell_key: usize,
    pub(in crate::ui::board_workspace::page::editor) list_item_index: usize,
    pub(in crate::ui::board_workspace::page::editor) column_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageLinkNamedIconPreference {
    AskEveryTime,
    Color(NotionNamedIconColor),
}

impl Default for PageLinkNamedIconPreference {
    fn default() -> Self {
        Self::Color(NotionNamedIconColor::Gray)
    }
}

impl PageLinkNamedIconPreference {
    pub(in crate::ui::board_workspace::page::editor) const fn preview_color(
        self,
    ) -> NotionNamedIconColor {
        match self {
            Self::AskEveryTime => NotionNamedIconColor::Gray,
            Self::Color(color) => color,
        }
    }
}

#[derive(Clone)]
enum PageLinkIconUploadSource {
    ExternalUrl(String),
    LocalFile(PreparedPageIconUpload),
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageLinkIconUploadPreview {
    source: PageLinkIconUploadSource,
    pub(in crate::ui::board_workspace::page::editor) rendered: Arc<RenderImage>,
}

#[derive(Clone)]
pub(crate) struct PageLinkIconPickerState {
    pub(crate) instance_id: u64,
    pub(crate) page_id: String,
    pub(crate) block_id: String,
    pub(crate) tab: PageLinkIconPickerTab,
    pub(crate) query: String,
    pub(crate) search_input: gpui::Entity<TextInput>,
    pub(crate) emoji_list_state: gpui::ListState,
    pub(crate) named_icon_list_state: gpui::ListState,
    pub(crate) named_icon_matches: Arc<[NotionNamedIconSlug]>,
    pub(crate) recent_named_icons: Arc<[NotionNamedIconSlug]>,
    pub(crate) category_index: usize,
    pub(crate) skin_tone: emojis::SkinTone,
    pub(crate) named_icon_preference: PageLinkNamedIconPreference,
    pub(crate) control_menu: Option<PageLinkIconPickerControlMenu>,
    pub(in crate::ui::board_workspace::page::editor) upload_preview:
        Option<PageLinkIconUploadPreview>,
    pub(crate) upload_generation: u64,
    pub(crate) upload_pending: bool,
    pub(crate) upload_committed: bool,
    pub(crate) upload_add_to_library: bool,
    pub(crate) upload_name: String,
    pub(crate) upload_name_input: gpui::Entity<TextInput>,
    pub(crate) custom_emojis: Arc<[NotionCustomEmoji]>,
    pub(crate) custom_emoji_names: Arc<HashSet<String>>,
    pub(crate) custom_emoji_library_generation: u64,
    pub(crate) custom_emoji_library_pending: bool,
    pub(crate) custom_emoji_creation_allowed: bool,
    pub(crate) custom_emoji_total_count: usize,
    pub(crate) custom_emoji_limit: Option<usize>,
}

impl PageLinkIconPickerState {
    fn matches_instance(&self, instance_id: u64, page_id: &str, block_id: &str) -> bool {
        self.instance_id == instance_id && self.page_id == page_id && self.block_id == block_id
    }

    pub(in crate::ui::board_workspace::page::editor) fn custom_emoji_limit_reached(&self) -> bool {
        self.custom_emoji_limit
            .is_some_and(|limit| self.custom_emoji_total_count >= limit)
    }

    pub(in crate::ui::board_workspace::page::editor) fn custom_emoji_name_is_taken(
        &self,
        name: &str,
    ) -> bool {
        self.custom_emoji_names.contains(name) || notion_builtin_emoji_name_exists(name)
    }
}

fn notion_builtin_emoji_name_exists(name: &str) -> bool {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            include_str!("../../../assets/notion_emoji_aliases.txt")
                .lines()
                .collect()
        })
        .binary_search(&name)
        .is_ok()
}

mod presentation;
pub(super) use presentation::PageLinkIconPresentation;
