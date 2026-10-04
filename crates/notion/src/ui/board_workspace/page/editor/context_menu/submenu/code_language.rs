use std::sync::Arc;

use gpui::{list, point, App, BoxShadow, ClickEvent, ListSizingBehavior, Stateful};

use super::super::super::{
    alpha, div, img, px, rgb, rgba, AnyElement, Div, ElementId, FluentBuilder, InteractiveElement,
    IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, PAGE_CODE_LANGUAGES,
    PAGE_CODE_LANGUAGE_OPTION_STRIDE,
};
use super::super::actions::PageBlockContextMenuAction;
use super::super::catalog::{PAGE_BLOCK_MENU_ROW_HEIGHT, PAGE_CODE_LANGUAGE_MENU_WIDTH};
use super::super::render::PageBlockContextMenuRenderer;
use super::super::state::{PageBlockContextMenuPanel, PageBlockMenuSelection};

struct PageCodeLanguageRows {
    indices: Arc<[usize]>,
    current_language: Option<String>,
    block_id: String,
    selection: PageBlockMenuSelection,
}

#[derive(Clone, Copy)]
struct PageCodeLanguageRow<'a> {
    block_id: &'a str,
    catalog_index: usize,
    row_index: usize,
    selected: bool,
    current: bool,
}

impl PageBlockContextMenuRenderer {
    pub(in crate::ui::board_workspace::page::editor::context_menu) fn render_page_code_language_menu(
        &self,
        cx: &mut App,
    ) -> Stateful<Div> {
        let rows = PageCodeLanguageRows {
            indices: self.menu.code_language_indices.clone(),
            current_language: self
                .target
                .block
                .editable_content()
                .and_then(|editable| editable.code_language())
                .map(|language| language.as_str().to_string()),
            block_id: self.target.block.block_id.clone(),
            selection: self.menu.selection,
        };
        let row_count = rows.indices.len();
        let max_height = self.viewport_height * 0.50;
        self.page_block_menu_surface(PAGE_CODE_LANGUAGE_MENU_WIDTH, max_height)
            .id("notion-code-language-menu")
            .role(Role::Dialog)
            .aria_label("Choose code language")
            .h(px(page_code_language_menu_height(row_count, max_height)))
            .rounded(px(10.0))
            .shadow(page_code_language_menu_shadow())
            .flex()
            .flex_col()
            .child(
                self.render_page_code_language_search(self.menu.code_language_search_input.clone()),
            )
            .child(self.render_page_code_language_options(
                rows,
                self.menu.code_language_list_state.clone(),
                cx,
            ))
    }

    fn render_page_code_language_search(
        &self,
        input: gpui::Entity<gpui_components::text_input::TextInput>,
    ) -> Div {
        div()
            .h(px(48.0))
            .flex_none()
            .pt(px(8.0))
            .px(px(4.0))
            .pb(px(4.0))
            .child(
                div().h(px(36.0)).w_full().py(px(4.0)).px(px(8.0)).child(
                    div()
                        .id("notion-code-language-search")
                        .h(px(28.0))
                        .w_full()
                        .rounded(px(6.0))
                        .overflow_hidden()
                        .child(input),
                ),
            )
    }

    fn render_page_code_language_options(
        &self,
        rows: PageCodeLanguageRows,
        list_state: gpui::ListState,
        cx: &mut App,
    ) -> Stateful<Div> {
        let row_count = rows.indices.len();
        let list = self.render_page_code_language_list(rows, list_state, cx);
        div()
            .id("notion-code-language-options")
            .role(Role::Menu)
            .flex_grow(1.0)
            .min_h(px(0.0))
            .when(row_count > 0, |options| options.child(list))
            .when(row_count == 0, |options| {
                options.child(
                    div()
                        .h(px(PAGE_BLOCK_MENU_ROW_HEIGHT))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .text_size(px(13.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child("No languages found"),
                )
            })
    }

    fn render_page_code_language_list(
        &self,
        rows: PageCodeLanguageRows,
        list_state: gpui::ListState,
        _cx: &mut App,
    ) -> impl gpui::IntoElement {
        let renderer = self.clone();
        list(list_state, move |row_index, _window, cx| {
            let indices = rows.indices.clone();
            let current_language = rows.current_language.clone();
            let block_id = rows.block_id.clone();
            let catalog_index = *indices
                .get(row_index)
                .expect("virtual Code language row must remain in the filtered catalog");
            renderer.render_page_code_language_row(
                PageCodeLanguageRow {
                    block_id: &block_id,
                    catalog_index,
                    row_index,
                    selected: rows.selection.is_row(row_index),
                    current: current_language.as_deref()
                        == Some(PAGE_CODE_LANGUAGES[catalog_index]),
                },
                cx,
            )
        })
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .size_full()
    }

    fn render_page_code_language_row(
        &self,
        spec: PageCodeLanguageRow<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let language = PAGE_CODE_LANGUAGES[spec.catalog_index];
        let row = self.render_page_code_language_row_frame(spec, language, cx);
        let row = self.bind_page_code_language_row(row, spec, cx);
        div()
            .h(px(PAGE_CODE_LANGUAGE_OPTION_STRIDE))
            .child(row)
            .into_any_element()
    }

    fn render_page_code_language_row_frame(
        &self,
        spec: PageCodeLanguageRow<'_>,
        language: &'static str,
        cx: &mut App,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!("notion-code-language-option-{}", spec.catalog_index).into(),
            ))
            .h(px(PAGE_BLOCK_MENU_ROW_HEIGHT))
            .mx(px(4.0))
            .px(px(8.0))
            .role(Role::MenuItem)
            .aria_label(language)
            .aria_selected(spec.current)
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .text_size(px(14.0))
            .line_height(px(16.8))
            .text_color(rgb(self.theme.text_primary))
            .bg(if spec.selected {
                rgba(self.theme.command_menu_active_bg)
            } else {
                alpha(self.theme.elevated_surface_bg, 0.0)
            })
            .hover(|style| style.bg(rgba(self.theme.command_menu_active_bg)))
            .child(div().min_w(px(0.0)).flex_grow(1.0).child(language))
            .when(spec.current, |row| {
                row.child(img(self.icons.block_menu_checked.render(cx)).size(px(20.0)))
            })
    }

    fn bind_page_code_language_row(
        &self,
        row: Stateful<Div>,
        spec: PageCodeLanguageRow<'_>,
        _cx: &mut App,
    ) -> Stateful<Div> {
        let click_block_id = spec.block_id.to_string();
        let hover_block_id = spec.block_id.to_string();
        let row_index = spec.row_index;
        let click_actions = self.actions.clone();
        let hover_actions = self.actions.clone();
        row.on_click(move |_: &ClickEvent, window, cx| {
            cx.stop_propagation();
            click_actions.emit(
                PageBlockContextMenuAction::ActivateSubmenuRow {
                    block_id: click_block_id.clone(),
                    panel: PageBlockContextMenuPanel::CodeLanguage,
                    index: row_index,
                },
                window,
                cx,
            );
        })
        .on_hover(move |hovered: &bool, window, cx| {
            if *hovered {
                hover_actions.emit(
                    PageBlockContextMenuAction::HoverSubmenu {
                        block_id: hover_block_id.clone(),
                        panel: PageBlockContextMenuPanel::CodeLanguage,
                        index: row_index,
                    },
                    window,
                    cx,
                );
            }
        })
    }
}

fn page_code_language_menu_height(row_count: usize, max_height: f32) -> f32 {
    (48.0 + row_count.max(1) as f32 * PAGE_CODE_LANGUAGE_OPTION_STRIDE - 1.0)
        .min(max_height)
        .max(76.0)
}

fn page_code_language_menu_shadow() -> Vec<BoxShadow> {
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
