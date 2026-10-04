use super::{
    alpha, div, px, relative, rgb, rgba, AnyElement, Bounds, Context, FluentBuilder, FontWeight,
    InlineDatabaseToolbarTarget, InlineDatabaseViewTabLayoutSource, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Pixels, Role,
    StatefulInteractiveElement, Styled, SurfaceState, TextRun, Window,
    INLINE_DATABASE_CONTROLS_HEIGHT, INLINE_DATABASE_MORE_HORIZONTAL_PADDING,
    INLINE_DATABASE_VIEW_TAB_FONT_SIZE, INLINE_DATABASE_VIEW_TAB_GAP,
    INLINE_DATABASE_VIEW_TAB_HORIZONTAL_PADDING, VIEW_TAB_ICON_GAP, VIEW_TAB_ICON_SIZE,
};
use crate::ui::surface::BoardViewState;
use gpui::Div;

mod layout;
mod renderer;
pub(super) use renderer::inline_database_view_tabs_renderer;

use layout::{
    best_visible_tab_indices, inline_database_fallback_visible_indices,
    inline_database_view_layout_source, measure_inline_database_view_label, MeasuredViewTabs,
};

impl BoardViewState {
    pub(super) fn inline_database_visible_tab_indices(
        &self,
        view_tabs: &[crate::ui::ViewTab],
    ) -> Vec<usize> {
        let source_matches = self.inline_database_view_tabs_layout.source.len() == view_tabs.len()
            && self
                .inline_database_view_tabs_layout
                .source
                .iter()
                .zip(view_tabs)
                .all(|(source, tab)| {
                    source.provider_view_id == tab.provider_view_id
                        && source.label == tab.label
                        && source.active == tab.active
                });
        if source_matches
            && !self
                .inline_database_view_tabs_layout
                .visible_indices
                .is_empty()
        {
            self.inline_database_view_tabs_layout
                .visible_indices
                .clone()
        } else {
            inline_database_fallback_visible_indices(view_tabs)
        }
    }

    pub(super) fn update_inline_database_view_tabs_layout(
        &mut self,
        view_tabs: &[crate::ui::ViewTab],
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> bool {
        let source = inline_database_view_layout_source(view_tabs);
        let mut font = window.text_style().font();
        font.weight = FontWeight::MEDIUM;
        let scale_factor_bits = window.scale_factor().to_bits();
        let source_changed = self.inline_database_view_tabs_layout.source != source;
        let font_changed = self.inline_database_view_tabs_layout.font.as_ref() != Some(&font)
            || self.inline_database_view_tabs_layout.scale_factor_bits != scale_factor_bits;
        let width_changed = self
            .inline_database_view_tabs_layout
            .bounds
            .is_none_or(|previous| previous.size.width != bounds.size.width);
        if !source_changed && !font_changed && !width_changed {
            return false;
        }
        if source_changed || font_changed {
            self.inline_database_view_tabs_layout.tab_widths = source
                .iter()
                .map(|tab| {
                    measure_inline_database_view_label(&tab.label, &font, window)
                        + VIEW_TAB_ICON_SIZE
                        + VIEW_TAB_ICON_GAP
                        + INLINE_DATABASE_VIEW_TAB_HORIZONTAL_PADDING * 2.0
                })
                .collect();
            self.inline_database_view_tabs_layout.source = source;
            self.inline_database_view_tabs_layout.font = Some(font);
            self.inline_database_view_tabs_layout.scale_factor_bits = scale_factor_bits;
        }
        self.inline_database_view_tabs_layout.bounds = Some(bounds);
        self.partition_inline_database_view_tabs(bounds.size.width.as_f32(), window);
        true
    }

    fn partition_inline_database_view_tabs(&mut self, available_width: f32, window: &mut Window) {
        let tab_count = self.inline_database_view_tabs_layout.source.len();
        if tab_count == 0 {
            self.inline_database_view_tabs_layout
                .visible_indices
                .clear();
            return;
        }
        let widths = &self.inline_database_view_tabs_layout.tab_widths;
        let all_width = widths.iter().sum::<f32>()
            + INLINE_DATABASE_VIEW_TAB_GAP * tab_count.saturating_sub(1) as f32;
        if all_width <= available_width {
            self.inline_database_view_tabs_layout.visible_indices = (0..tab_count).collect();
            return;
        }
        let active_index = self
            .inline_database_view_tabs_layout
            .source
            .iter()
            .position(|tab| tab.active)
            .expect("inline database view tabs require one active view");
        let font = self
            .inline_database_view_tabs_layout
            .font
            .as_ref()
            .expect("measured inline database tabs require a font")
            .clone();
        self.inline_database_view_tabs_layout.visible_indices = best_visible_tab_indices(
            MeasuredViewTabs {
                tab_count,
                active_index,
                widths,
            },
            available_width,
            &font,
            window,
        );
    }
}
