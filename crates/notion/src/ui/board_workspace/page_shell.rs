use super::{
    alpha, div, img, notion_ai_button_image, notion_ai_face_image, point, px, relative,
    render_svg_image, rgb, rgba, svg_from_body, AnyElement, AppearanceMode, Arc, BoxShadow,
    Context, Div, FluentBuilder, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, NotionAiMode, ParentElement, StatefulInteractiveElement, Styled, SurfaceState,
    Window,
};
use crate::ui::DraggedNotionSidebarResize;
use gpui::DragMoveEvent;

const NOTION_PAGE_BODY_WIDTH: f32 = 640.0;
const NOTION_PAGE_BODY_LEFT_GUTTER: f32 = 206.0;
const NOTION_PAGE_BODY_TOP_INSET: f32 = 128.0;

const fn notion_sidebar_bg(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0xf9f8f7,
        AppearanceMode::Dark => 0x202020,
    }
}

const fn notion_sidebar_active_bg(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x211b17,
        AppearanceMode::Dark => 0x2f2f2f,
    }
}

const fn notion_sidebar_text(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x37352f,
        AppearanceMode::Dark => 0xbbbab6,
    }
}

const fn notion_sidebar_active_text(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x37352f,
        AppearanceMode::Dark => 0xf3f2ef,
    }
}

const fn notion_sidebar_muted(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x9b9a97,
        AppearanceMode::Dark => 0x8a8884,
    }
}

const fn notion_page_body_text(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x37352f,
        AppearanceMode::Dark => 0xf7f6f3,
    }
}

const fn notion_page_body_muted(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x9b9a97,
        AppearanceMode::Dark => 0x8f8d89,
    }
}

mod ai;
mod body;
mod icons;
mod root;
mod sidebar;

pub(crate) use body::PageShellBodyRenderer;
pub(crate) use icons::PageShellIconRenderer;
pub(crate) use sidebar::{activate_sidebar_tab, SidebarView};

#[cfg(test)]
mod tests {
    use super::{
        notion_page_body_muted, notion_page_body_text, notion_sidebar_active_bg,
        notion_sidebar_active_text, notion_sidebar_bg, notion_sidebar_muted, notion_sidebar_text,
        AppearanceMode,
    };

    #[gpui::test]
    fn notion_page_shell_light_palette_matches_expected_values() {
        assert_eq!(notion_sidebar_bg(AppearanceMode::Light), 0xf9f8f7);
        assert_eq!(notion_sidebar_active_bg(AppearanceMode::Light), 0x211b17);
        assert_eq!(notion_sidebar_text(AppearanceMode::Light), 0x37352f);
        assert_eq!(notion_sidebar_active_text(AppearanceMode::Light), 0x37352f);
        assert_eq!(notion_sidebar_muted(AppearanceMode::Light), 0x9b9a97);
        assert_eq!(notion_page_body_text(AppearanceMode::Light), 0x37352f);
        assert_eq!(notion_page_body_muted(AppearanceMode::Light), 0x9b9a97);
    }
}
