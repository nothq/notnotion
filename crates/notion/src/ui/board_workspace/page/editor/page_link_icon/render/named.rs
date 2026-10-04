use gpui::prelude::FluentBuilder;
use gpui::App;
use gpui::{
    list, AnyElement, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    ListSizingBehavior, ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};

use super::super::{PageLinkIconAction, PageLinkIconSelectionAction};
use crate::ui::{
    board_workspace::{
        alpha, div, img,
        page::editor::{
            PageLinkIconPickerState, PageLinkIconTarget, PageLinkIconView,
            PageLinkNamedIconSelection,
        },
        px, rgb,
    },
    NotionNamedIconAsset, NotionNamedIconColor, NotionNamedIconSlug,
};

use super::{
    PAGE_LINK_NAMED_ICON_COLUMN_COUNT, PAGE_LINK_NAMED_ICON_ROW_HEIGHT,
    PAGE_LINK_NAMED_ICON_SECTION_HEADER_HEIGHT,
};

struct PageLinkNamedIconListRender<'a> {
    target: &'a PageLinkIconTarget,
    matches: &'a [NotionNamedIconSlug],
    recent: &'a [NotionNamedIconSlug],
    query_is_empty: bool,
    color: NotionNamedIconColor,
}

struct PageLinkNamedIconRowRender<'a> {
    target: &'a PageLinkIconTarget,
    icons: &'a [NotionNamedIconSlug],
    list_item_index: usize,
    row_index: usize,
    recent: bool,
    color: NotionNamedIconColor,
}

impl PageLinkIconView {
    pub(super) fn render_page_link_named_icon_grid(
        &self,
        state: &PageLinkIconPickerState,
        page_id: &str,
        block_id: &str,
    ) -> AnyElement {
        let matches = state.named_icon_matches.clone();
        let recent = state.recent_named_icons.clone();
        let query_is_empty = state.query.trim().is_empty();
        let color = state.named_icon_preference.preview_color();
        let target = PageLinkIconTarget {
            page_id: page_id.to_string(),
            block_id: block_id.to_string(),
        };
        let view = self.clone();
        let rows = list(
            state.named_icon_list_state.clone(),
            move |item_index, _, cx| {
                let matches = matches.clone();
                let recent = recent.clone();
                let target = target.clone();
                view.render_page_link_named_icon_list_item(
                    item_index,
                    PageLinkNamedIconListRender {
                        target: &target,
                        matches: &matches,
                        recent: &recent,
                        query_is_empty,
                        color,
                    },
                    cx,
                )
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .size_full();
        div()
            .id("notion-page-link-named-icon-grid")
            .role(Role::Grid)
            .aria_label("Icons")
            .flex_grow(1.0)
            .min_h(px(0.0))
            .child(rows)
            .into_any_element()
    }

    fn render_page_link_named_icon_list_item(
        &self,
        item_index: usize,
        context: PageLinkNamedIconListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        if context.matches.is_empty() {
            return self.render_empty_page_link_named_icon_result();
        }
        if context.query_is_empty {
            return self.render_page_link_named_icon_catalog_item(item_index, context, cx);
        }
        self.render_page_link_named_icon_row(
            PageLinkNamedIconRowRender {
                target: context.target,
                icons: context.matches,
                list_item_index: item_index,
                row_index: item_index,
                recent: false,
                color: context.color,
            },
            cx,
        )
    }

    fn render_empty_page_link_named_icon_result(&self) -> AnyElement {
        div()
            .h(px(64.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_muted))
            .child("No icons found")
            .into_any_element()
    }

    fn render_page_link_named_icon_catalog_item(
        &self,
        item_index: usize,
        context: PageLinkNamedIconListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let has_recent = !context.recent.is_empty();
        if item_index == 0 && has_recent {
            return self.render_page_link_named_icon_section_header("Recent", false);
        }
        if item_index == 1 && has_recent {
            return self.render_page_link_named_icon_recent_row(item_index, context, cx);
        }
        let icons_header_index = if has_recent { 2 } else { 0 };
        if item_index == icons_header_index {
            return self.render_page_link_named_icon_section_header("Icons", has_recent);
        }
        self.render_page_link_named_icon_row(
            PageLinkNamedIconRowRender {
                target: context.target,
                icons: context.matches,
                list_item_index: item_index,
                row_index: item_index.saturating_sub(icons_header_index + 1),
                recent: false,
                color: context.color,
            },
            cx,
        )
    }

    fn render_page_link_named_icon_recent_row(
        &self,
        item_index: usize,
        context: PageLinkNamedIconListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        self.render_page_link_named_icon_row(
            PageLinkNamedIconRowRender {
                target: context.target,
                icons: context.recent,
                list_item_index: item_index,
                row_index: 0,
                recent: true,
                color: context.color,
            },
            cx,
        )
    }

    fn render_page_link_named_icon_section_header(
        &self,
        label: &'static str,
        separated_from_previous_section: bool,
    ) -> AnyElement {
        let separation = if separated_from_previous_section {
            4.0
        } else {
            0.0
        };
        div()
            .h(px(PAGE_LINK_NAMED_ICON_SECTION_HEADER_HEIGHT + separation))
            .px(px(12.0))
            .pt(px(10.0 + separation))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .text_color(rgb(self.theme.text_muted))
            .child(label)
            .into_any_element()
    }

    fn render_page_link_named_icon_row(
        &self,
        context: PageLinkNamedIconRowRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let start = context.row_index * PAGE_LINK_NAMED_ICON_COLUMN_COUNT;
        let end = (start + PAGE_LINK_NAMED_ICON_COLUMN_COUNT).min(context.icons.len());
        context.icons[start..end]
            .iter()
            .copied()
            .enumerate()
            .fold(page_link_named_icon_row(&context), |row, (column, slug)| {
                let index = start + column;
                let cell_key = if context.recent {
                    usize::MAX - index
                } else {
                    index
                };
                row.child(self.render_page_link_named_icon_cell(
                    PageLinkNamedIconSelection {
                        target: context.target.clone(),
                        slug,
                        cell_key,
                        list_item_index: context.list_item_index,
                        column_index: column,
                    },
                    context.color,
                    cx,
                ))
            })
            .into_any_element()
    }

    fn render_page_link_named_icon_cell(
        &self,
        selection: PageLinkNamedIconSelection,
        color: NotionNamedIconColor,
        cx: &mut App,
    ) -> AnyElement {
        let key_selection = selection.clone();
        let asset = NotionNamedIconAsset::new(selection.slug, color, self.appearance_mode);
        let icon = self.resources.named_icon_image(asset, &self.notifier, cx);
        div()
            .id(ElementId::Name(
                format!(
                    "notion-page-link-named-icon-{}-{}",
                    selection.target.block_id, selection.cell_key
                )
                .into(),
            ))
            .role(Role::GridCell)
            .aria_label(format!("{} {}", selection.slug.label(), color.suffix()))
            .focusable()
            .tab_stop(true)
            .relative()
            .size(px(32.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_click(self.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Selection(PageLinkIconSelectionAction::ChooseNamedIcon(
                        selection.clone(),
                    )),
                    window,
                    cx,
                );
            }))
            .on_key_down(
                self.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified()
                        || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Selection(
                            PageLinkIconSelectionAction::ChooseNamedIcon(key_selection.clone()),
                        ),
                        window,
                        cx,
                    );
                }),
            )
            .when_some(icon, |cell, icon| cell.child(img(icon).size(px(24.0))))
            .into_any_element()
    }
}

fn page_link_named_icon_row(context: &PageLinkNamedIconRowRender<'_>) -> Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!(
                "notion-page-link-named-icon-row-{}-{}",
                if context.recent { "recent" } else { "catalog" },
                context.row_index,
            )
            .into(),
        ))
        .role(Role::Row)
        .w_full()
        .h(px(PAGE_LINK_NAMED_ICON_ROW_HEIGHT))
        .pl(px(16.0))
        .flex()
        .items_center()
}
