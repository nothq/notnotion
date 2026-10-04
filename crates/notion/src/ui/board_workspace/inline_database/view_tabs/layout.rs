use super::{
    px, InlineDatabaseViewTabLayoutSource, TextRun, Window,
    INLINE_DATABASE_MORE_HORIZONTAL_PADDING, INLINE_DATABASE_VIEW_TAB_FONT_SIZE,
    INLINE_DATABASE_VIEW_TAB_GAP,
};

pub(super) fn inline_database_view_layout_source(
    view_tabs: &[crate::ui::ViewTab],
) -> Vec<InlineDatabaseViewTabLayoutSource> {
    view_tabs
        .iter()
        .map(|tab| InlineDatabaseViewTabLayoutSource {
            provider_view_id: tab.provider_view_id.clone(),
            label: tab.label.clone(),
            active: tab.active,
        })
        .collect()
}

/// The measured view tabs: how many there are, which one is active, and each
/// tab's width.
pub(super) struct MeasuredViewTabs<'a> {
    pub(super) tab_count: usize,
    pub(super) active_index: usize,
    pub(super) widths: &'a [f32],
}

pub(super) fn best_visible_tab_indices(
    tabs: MeasuredViewTabs<'_>,
    available_width: f32,
    font: &gpui::Font,
    window: &mut Window,
) -> Vec<usize> {
    let MeasuredViewTabs {
        tab_count,
        active_index,
        widths,
    } = tabs;
    let mut best = Vec::new();
    for visible_count in 1..tab_count {
        let indices = inline_database_visible_indices(tab_count, active_index, visible_count);
        let hidden_count = tab_count - visible_count;
        let more_width =
            measure_inline_database_view_label(&format!("{hidden_count} more…"), font, window)
                + INLINE_DATABASE_MORE_HORIZONTAL_PADDING * 2.0;
        let direct_width = indices.iter().map(|index| widths[*index]).sum::<f32>();
        if direct_width + more_width + INLINE_DATABASE_VIEW_TAB_GAP * visible_count as f32
            <= available_width
        {
            best = indices;
        }
    }
    if best.is_empty() {
        best.push(active_index);
    }
    best
}

pub(super) fn inline_database_fallback_visible_indices(
    view_tabs: &[crate::ui::ViewTab],
) -> Vec<usize> {
    if view_tabs.len() <= 2 {
        return (0..view_tabs.len()).collect();
    }
    let active_index = view_tabs
        .iter()
        .position(|tab| tab.active)
        .expect("inline database view tabs require one active view");
    inline_database_visible_indices(view_tabs.len(), active_index, 2)
}

fn inline_database_visible_indices(
    tab_count: usize,
    active_index: usize,
    visible_count: usize,
) -> Vec<usize> {
    let mut indices = (0..visible_count.min(tab_count)).collect::<Vec<_>>();
    if active_index >= indices.len() {
        *indices
            .last_mut()
            .expect("overflowing inline database tabs require one visible view") = active_index;
    }
    indices
}

pub(super) fn measure_inline_database_view_label(
    label: &str,
    font: &gpui::Font,
    window: &mut Window,
) -> f32 {
    let run = TextRun {
        len: label.len(),
        font: font.clone(),
        color: window.text_style().color,
        ..Default::default()
    };
    f32::from(
        window
            .text_system()
            .layout_line(label, px(INLINE_DATABASE_VIEW_TAB_FONT_SIZE), &[run], None)
            .width,
    )
}
