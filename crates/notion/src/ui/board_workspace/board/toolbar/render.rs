use super::buttons::ToolbarIconNode;
use super::{
    div, px, Div, FluentBuilder, InlineDatabaseToolbarTarget, InlineToolbarDialogAnchors,
    ParentElement, Styled, ToolbarDialogKind,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(super) fn render_board_toolbar_content(
        &self,
        right_margin: f32,
        inline_target: Option<InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> Div {
        let date_undated_count = self.undated_count;
        let inline_toolbar = inline_target.is_some();
        let inline_toolbar_minimized = inline_toolbar && self.inline_toolbar_minimized;
        let toolbar_search_open = self.database_search.open;
        let toolbar =
            self.render_toolbar_base(date_undated_count, right_margin, inline_target.as_ref(), cx);
        let toolbar = if inline_toolbar_minimized {
            self.render_minimized_toolbar(toolbar, inline_target.as_ref(), cx)
        } else {
            self.render_expanded_toolbar(toolbar, inline_target.as_ref(), cx)
        };
        self.track_toolbar_layout(
            toolbar,
            BoardToolbarLayout {
                inline_toolbar,
                inline_toolbar_minimized,
                toolbar_search_open,
                has_undated_badge: date_undated_count.is_some(),
            },
        )
    }

    fn render_toolbar_base(
        &self,
        date_undated_count: Option<usize>,
        right_margin: f32,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(0.0))
            .mr(px(right_margin))
            .when_some(date_undated_count, |this, undated_count| {
                this.child(self.render_timeline_undated_badge(undated_count, inline_target, cx))
            })
    }

    fn render_minimized_toolbar(
        &self,
        toolbar: Div,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> Div {
        toolbar
            .child(self.render_inline_toolbar_visibility_button(false, cx))
            .child(self.render_toolbar_icon_node(
                ToolbarIconNode::new(
                    &self.icons.toolbar_settings,
                    Some(ToolbarDialogKind::Properties),
                ),
                inline_target,
                cx,
            ))
    }

    fn render_expanded_toolbar(
        &self,
        toolbar: Div,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> Div {
        toolbar
            .when(inline_target.is_some(), |this| {
                this.child(self.render_inline_toolbar_visibility_button(true, cx))
            })
            .child(self.render_toolbar_icon_node(
                ToolbarIconNode::new(&self.icons.toolbar_filter, Some(ToolbarDialogKind::Filter)),
                inline_target,
                cx,
            ))
            .child(self.render_toolbar_icon_node(
                ToolbarIconNode::new(&self.icons.toolbar_sort, Some(ToolbarDialogKind::Sort)),
                inline_target,
                cx,
            ))
            .child(self.render_toolbar_icon_node(
                ToolbarIconNode::new(
                    &self.icons.toolbar_lightning,
                    Some(ToolbarDialogKind::Automations),
                ),
                inline_target,
                cx,
            ))
            .child(self.render_ai_autofill_button(inline_target, cx))
            .child(self.render_toolbar_search_node(inline_target, cx))
            .when(self.database_search.open, |this| {
                this.child(self.render_toolbar_search_input(inline_target, cx))
            })
            .when_some(inline_target.cloned(), |this, target| {
                this.child(self.render_inline_database_expand_button(target, cx))
            })
            .child(self.render_toolbar_icon_node(
                ToolbarIconNode::new(
                    &self.icons.toolbar_settings,
                    Some(ToolbarDialogKind::Properties),
                ),
                inline_target,
                cx,
            ))
            .child(self.render_primary_new_button(inline_target, cx))
    }

    fn track_toolbar_layout(&self, toolbar: Div, layout: BoardToolbarLayout) -> Div {
        let actions = self.actions.clone();
        toolbar.on_children_prepainted(move |child_bounds, window, cx| {
            let Some(toolbar_bounds) = toolbar_children_bounds(&child_bounds) else {
                return;
            };
            let inline_dialog_anchors = layout.inline_toolbar.then(|| {
                if layout.inline_toolbar_minimized {
                    InlineToolbarDialogAnchors {
                        properties: Some(toolbar_bounds),
                        ..Default::default()
                    }
                } else {
                    inline_toolbar_dialog_anchors(
                        &child_bounds,
                        layout.has_undated_badge,
                        layout.toolbar_search_open,
                    )
                }
            });
            let measurement = super::BoardToolbarMeasurement {
                toolbar_bounds,
                undated_badge_bounds: if layout.has_undated_badge {
                    child_bounds.first().copied()
                } else {
                    None
                },
                inline_dialog_anchors,
            };
            actions.emit(BoardToolbarAction::Measured(measurement), window, cx);
        })
    }
}

fn toolbar_children_bounds(
    bounds: &[gpui::Bounds<gpui::Pixels>],
) -> Option<gpui::Bounds<gpui::Pixels>> {
    let first = bounds.first()?;
    let mut left = first.left().as_f32();
    let mut top = first.top().as_f32();
    let mut right = first.right().as_f32();
    let mut bottom = first.bottom().as_f32();
    for bounds in &bounds[1..] {
        left = left.min(bounds.left().as_f32());
        top = top.min(bounds.top().as_f32());
        right = right.max(bounds.right().as_f32());
        bottom = bottom.max(bounds.bottom().as_f32());
    }
    Some(gpui::Bounds::from_corners(
        gpui::point(px(left), px(top)),
        gpui::point(px(right), px(bottom)),
    ))
}

fn inline_toolbar_dialog_anchors(
    bounds: &[gpui::Bounds<gpui::Pixels>],
    has_undated_badge: bool,
    search_open: bool,
) -> InlineToolbarDialogAnchors {
    let first_icon = usize::from(has_undated_badge);
    let filter_index = first_icon + 1;
    let properties_index = first_icon + 7 + usize::from(search_open);
    let templates_index = properties_index + 1;
    let templates_button = bounds.get(templates_index).copied().map(|button| {
        gpui::Bounds::from_corners(
            gpui::point(px(button.right().as_f32() - 24.0), button.top()),
            gpui::point(button.right(), button.bottom()),
        )
    });
    InlineToolbarDialogAnchors {
        filter: bounds.get(filter_index).copied(),
        sort: bounds.get(filter_index + 1).copied(),
        automations: bounds.get(filter_index + 2).copied(),
        properties: toolbar_children_bounds(bounds),
        templates: templates_button,
    }
}

pub(super) const fn inline_toolbar_dialog_label(dialog: ToolbarDialogKind) -> &'static str {
    match dialog {
        ToolbarDialogKind::Filter => "Filter",
        ToolbarDialogKind::Sort => "Sort",
        ToolbarDialogKind::Automations => "Automations",
        ToolbarDialogKind::Properties => "Settings",
        ToolbarDialogKind::Templates => "More add options",
        ToolbarDialogKind::Actions => "Actions",
    }
}

pub(super) const fn inline_toolbar_dialog_button_id(dialog: ToolbarDialogKind) -> &'static str {
    match dialog {
        ToolbarDialogKind::Filter => "notion-inline-toolbar-filter",
        ToolbarDialogKind::Sort => "notion-inline-toolbar-sort",
        ToolbarDialogKind::Automations => "notion-inline-toolbar-automations",
        ToolbarDialogKind::Properties => "notion-inline-toolbar-settings",
        ToolbarDialogKind::Templates => "notion-inline-toolbar-templates",
        ToolbarDialogKind::Actions => "notion-inline-toolbar-actions",
    }
}

pub(super) const fn toolbar_dialog_button_id(dialog: ToolbarDialogKind) -> &'static str {
    match dialog {
        ToolbarDialogKind::Filter => "notion-toolbar-filter",
        ToolbarDialogKind::Sort => "notion-toolbar-sort",
        ToolbarDialogKind::Automations => "notion-toolbar-automations",
        ToolbarDialogKind::Properties => "notion-toolbar-settings",
        ToolbarDialogKind::Templates => "notion-toolbar-templates",
        ToolbarDialogKind::Actions => "notion-toolbar-actions",
    }
}

struct BoardToolbarLayout {
    inline_toolbar: bool,
    inline_toolbar_minimized: bool,
    toolbar_search_open: bool,
    has_undated_badge: bool,
}
