use super::super::{
    column_style, AppearanceMode, BoardSnapshot, Card, CardPage, ColumnState, HashMap, Image,
    ImageFormat, SurfaceState,
};
use crate::model::PagePresenceSnapshot;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};

impl SurfaceState {
    pub(crate) fn replace_notion_page_presence(&mut self, presence: PagePresenceSnapshot) {
        self.board.presence = Some(presence);
        self.presence_images = board_presence_images(&self.board);
    }
}

pub(super) fn board_presence_images(board: &BoardSnapshot) -> Vec<Option<crate::ui::Arc<Image>>> {
    board
        .presence
        .as_ref()
        .into_iter()
        .flat_map(|presence| presence.profiles.iter())
        .map(|profile| {
            let avatar = profile.avatar.as_ref()?;
            let format = ImageFormat::from_mime_type(&avatar.mimetype)?;
            let bytes = BASE64_STANDARD.decode(&avatar.base64).ok()?;
            Some(crate::ui::Arc::new(Image::from_bytes(format, bytes)))
        })
        .collect()
}

pub(super) fn empty_board_snapshot() -> BoardSnapshot {
    BoardSnapshot {
        share_target_id: None,
        page_title: String::new(),
        database_title: String::new(),
        edited_label: String::new(),
        user_time_zone: "UTC".to_string(),
        user_utc_offset_seconds: 0,
        is_private: false,
        is_locked: false,
        is_favorited: false,
        presence: None,
        columns: Vec::new(),
        view_tabs: Vec::new(),
        items: Vec::new(),
        table_view_columns: Vec::new(),
        active_view_property_layout: Default::default(),
        active_view_sorts: Default::default(),
        active_view_group: Default::default(),
        database_properties: Vec::new(),
        timeline_view: None,
        calendar_view: None,
        date_undated_count: None,
        page_content: None,
        page_shell: None,
    }
}

pub(super) fn notion_sidebar_should_be_visible(board: &BoardSnapshot) -> bool {
    board.page_shell.as_ref().is_some_and(|page_shell| {
        !page_shell.builtin_links.is_empty() || !page_shell.sidebar_sections.is_empty()
    })
}

pub(super) fn build_columns(
    board: &BoardSnapshot,
    snapshot_pages: Option<&HashMap<String, CardPage>>,
    appearance_mode: AppearanceMode,
) -> Vec<ColumnState> {
    board
        .columns
        .iter()
        .map(|column| ColumnState {
            title: column.title.clone(),
            style: column_style(
                &column.title,
                column.option_color.as_deref(),
                appearance_mode,
            ),
            cards: column
                .cards
                .iter()
                .map(|card| Card {
                    title: card.title.clone(),
                    block_id: card.block_id.clone(),
                    height: card.height,
                    has_content: snapshot_pages
                        .and_then(|pages| pages.get(&card.block_id))
                        .map(|page| !page.blocks.is_empty())
                        .unwrap_or(card.has_content),
                    icon: card.icon.clone(),
                    fill_override: None,
                })
                .collect(),
        })
        .collect()
}
