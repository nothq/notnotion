use chrono::{DateTime, Local, Utc};
use gpui::{point, App, BoxShadow, Stateful};
use gpui_components::text_input::TextInput;

use super::super::{
    alpha, div, img, px, rgb, rgba, AnyElement, AppearanceMode, CardPageBlock, Div, ElementId,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use super::catalog::{
    PageBlockMenuAction, PAGE_BLOCK_MENU_PANEL_INSET, PAGE_BLOCK_MENU_ROW_HEIGHT,
};
use super::render::PageBlockContextMenuRenderer;
use crate::model::CardPageCodeWrap;

#[derive(Clone, Copy)]
pub(super) struct PageBlockMenuRowState {
    pub(super) selected: bool,
    pub(super) enabled: bool,
}

impl PageBlockContextMenuRenderer {
    pub(super) fn render_page_block_menu_search(&self, input: gpui::Entity<TextInput>) -> Div {
        div().h(px(48.0)).pt(px(8.0)).child(
            div()
                .id("notion-block-menu-search")
                .mx(px(12.0))
                .w(px(241.0))
                .h(px(28.0))
                .relative()
                .rounded(px(6.0))
                .flex()
                .items_center()
                .overflow_hidden()
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(6.0))
                        .border_2()
                        .border_color(rgb(0x2383e2)),
                )
                .child(div().w_full().h_full().child(input)),
        )
    }

    pub(super) fn render_page_code_wrap_switch(&self) -> Div {
        let checked = self
            .target
            .code_wrap
            .is_some_and(CardPageCodeWrap::is_enabled);
        div()
            .w(px(30.0))
            .h(px(18.0))
            .rounded(px(44.0))
            .bg(if checked {
                rgb(0x2783de).into()
            } else {
                alpha(self.theme.text_primary, 0.22)
            })
            .flex()
            .items_center()
            .p(px(2.0))
            .when(checked, |track| track.justify_end())
            .child(div().size(px(14.0)).rounded(px(44.0)).bg(rgb(0xffffff)))
    }

    pub(super) fn page_block_menu_surface(&self, width: f32, max_height: f32) -> Div {
        div()
            .w(px(width))
            .max_h(px(max_height))
            .rounded(px(10.0))
            .overflow_hidden()
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(page_block_menu_shadow(self.appearance_mode))
            .text_size(px(14.0))
            .line_height(px(16.8))
            .text_color(rgb(self.theme.text_primary))
    }

    pub(super) fn page_block_menu_row(
        &self,
        menu_width: f32,
        id: String,
        aria_label: &str,
        state: PageBlockMenuRowState,
    ) -> Stateful<Div> {
        let PageBlockMenuRowState { selected, enabled } = state;
        div()
            .id(ElementId::Name(id.into()))
            .w(px(menu_width - PAGE_BLOCK_MENU_PANEL_INSET * 2.0))
            .h(px(PAGE_BLOCK_MENU_ROW_HEIGHT))
            .rounded(px(6.0))
            .role(Role::ListBoxOption)
            .aria_label(if enabled {
                aria_label.to_string()
            } else {
                format!("{aria_label}, unavailable")
            })
            .aria_selected(selected && enabled)
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .bg(if selected && enabled {
                rgba(self.theme.command_menu_active_bg)
            } else {
                alpha(self.theme.elevated_surface_bg, 0.0)
            })
            .opacity(if enabled { 1.0 } else { 0.38 })
            .when(enabled, |row| {
                row.cursor_pointer()
                    .hover(|style| style.bg(rgba(self.theme.command_menu_active_bg)))
            })
    }

    pub(super) fn render_page_block_menu_icon(
        &self,
        action: PageBlockMenuAction,
        fallback: &'static str,
        cx: &mut App,
    ) -> AnyElement {
        let asset = match action {
            PageBlockMenuAction::CodeLanguage => Some(&self.icons.block_menu_code_language),
            PageBlockMenuAction::CodeWrap => Some(&self.icons.block_menu_code_wrap),
            PageBlockMenuAction::CopyCode => Some(&self.icons.block_menu_copy_code),
            PageBlockMenuAction::TurnInto => Some(&self.icons.block_menu_turn_into),
            PageBlockMenuAction::Color => Some(&self.icons.block_menu_color),
            PageBlockMenuAction::EditIcon => None,
            PageBlockMenuAction::QuoteSize => Some(&self.icons.block_menu_quote_size),
            PageBlockMenuAction::CopyLink => Some(&self.icons.block_menu_copy_link),
            PageBlockMenuAction::Duplicate => Some(&self.icons.block_menu_duplicate),
            PageBlockMenuAction::MoveTo => Some(&self.icons.block_menu_move),
            PageBlockMenuAction::Delete => Some(&self.icons.block_menu_delete),
            PageBlockMenuAction::Comment => Some(&self.icons.block_menu_comment),
            PageBlockMenuAction::SuggestEdits => Some(&self.icons.block_menu_suggest_edits),
            PageBlockMenuAction::Present => Some(&self.icons.block_menu_present),
            PageBlockMenuAction::AskAi => Some(&self.icons.block_menu_ask_ai),
            PageBlockMenuAction::Skills => Some(&self.icons.block_menu_skills),
        };
        if let Some(asset) = asset {
            return img(asset.render(cx)).size(px(20.0)).into_any_element();
        }
        self.render_page_block_menu_glyph(fallback)
    }

    pub(super) fn render_page_block_menu_glyph(&self, glyph: &'static str) -> AnyElement {
        div()
            .size(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(17.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_secondary))
            .child(glyph)
            .into_any_element()
    }

    pub(super) fn render_page_block_menu_section_title(&self, title: &'static str) -> Div {
        div()
            .h(px(28.9))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .line_height(px(14.4))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_muted))
            .child(title)
    }

    pub(super) fn render_page_block_color_section_title(&self, title: &'static str) -> Div {
        div()
            .h(px(14.9))
            .mt(px(6.0))
            .mb(px(8.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .line_height(px(14.4))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_muted))
            .child(title)
    }

    pub(super) fn render_page_block_menu_footer(&self, block: &CardPageBlock) -> Option<Div> {
        let last_edited = block.last_edited.as_ref()?;
        let edited_at = format_page_block_menu_edited_at(last_edited.timestamp_ms)?;
        Some(
            div()
                .border_t_1()
                .border_color(rgba(self.theme.surface_border))
                .p(px(4.0))
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child(format!("Last edited by {}", last_edited.editor_name))
                        .child(edited_at),
                ),
        )
    }
}

fn format_page_block_menu_edited_at(timestamp_ms: u64) -> Option<String> {
    let edited_at =
        DateTime::<Utc>::from_timestamp_millis(timestamp_ms as i64)?.with_timezone(&Local);
    let age_days = Local::now()
        .date_naive()
        .signed_duration_since(edited_at.date_naive())
        .num_days();
    let time = edited_at.format("%-I:%M %p");
    Some(match age_days {
        0 => format!("Today at {time}"),
        1 => format!("Yesterday at {time}"),
        _ => edited_at.format("%b %-d, %Y, %-I:%M %p").to_string(),
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_menu_shadow(
    appearance_mode: AppearanceMode,
) -> Vec<BoxShadow> {
    if appearance_mode == AppearanceMode::Dark {
        return page_block_menu_dark_shadow();
    }
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.05),
            offset: point(px(0.0), px(20.0)),
            blur_radius: px(24.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: point(px(0.0), px(5.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x2a1c00, 0.07),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}

fn page_block_menu_dark_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: rgb(0x383836).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.20),
            offset: point(px(0.0), px(14.0)),
            blur_radius: px(28.0),
            spread_radius: px(-6.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.118),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-1.0),
            inset: false,
        },
    ]
}
