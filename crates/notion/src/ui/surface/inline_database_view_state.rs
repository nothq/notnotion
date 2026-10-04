use gpui::{Bounds, Entity, Font, Pixels, SharedString, WeakEntity};
use gpui_components::text_input::TextInput;

use crate::model::{NotionCollectionViewId, ViewTab};

use super::InlineDatabaseView;

/// The inline database whose views a view menu lists.
pub(crate) struct InlineDatabaseViewMenuOwner {
    pub(crate) database_block_id: String,
    pub(crate) inline_view: WeakEntity<InlineDatabaseView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InlineDatabaseViewMenuSelection {
    View(NotionCollectionViewId),
    NewView,
    NewDataSource,
}

#[derive(Clone)]
pub(crate) struct InlineDatabaseViewMenuState {
    pub(crate) anchor: Bounds<Pixels>,
    pub(crate) database_block_id: String,
    pub(crate) inline_view: WeakEntity<InlineDatabaseView>,
    pub(crate) view_tabs: std::sync::Arc<[ViewTab]>,
    pub(crate) query: SharedString,
    pub(crate) highlighted: InlineDatabaseViewMenuSelection,
    pub(crate) input: Entity<TextInput>,
    pub(crate) new_view_picker_open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InlineDatabaseViewTabLayoutSource {
    pub(crate) provider_view_id: NotionCollectionViewId,
    pub(crate) label: String,
    pub(crate) active: bool,
}

#[derive(Default)]
pub(crate) struct InlineDatabaseViewTabsLayout {
    pub(crate) bounds: Option<Bounds<Pixels>>,
    pub(crate) source: Vec<InlineDatabaseViewTabLayoutSource>,
    pub(crate) font: Option<Font>,
    pub(crate) scale_factor_bits: u32,
    pub(crate) tab_widths: Vec<f32>,
    pub(crate) visible_indices: Vec<usize>,
}
