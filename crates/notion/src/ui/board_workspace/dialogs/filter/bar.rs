use crate::ui::board_workspace::dialogs::filter::{
    database_filter_renderer, helpers::*, prelude::*, types::*,
};

impl SurfaceState {
    pub(crate) fn render_database_filter_bar(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let host = if inline_target.is_some() {
            DatabaseFilterHost::Inline
        } else {
            DatabaseFilterHost::FullPage
        };
        database_filter_renderer(self, host, cx).render_database_filter_bar(cx)
    }
}

impl DatabaseFilterRenderer<'_> {
    fn render_database_filter_bar(&self, cx: &mut App) -> AnyElement {
        let chips = self.state.chips(self.board);
        let inline = self.resources.host == DatabaseFilterHost::Inline;
        let (advanced_rule_count, show_reset, show_save) =
            self.state
                .bar_controls(self.board, inline, self.resources.workspace_editable);

        database_filter_bar_container(inline)
            .children(
                chips
                    .into_iter()
                    .map(|chip| self.render_database_filter_chip(chip, cx)),
            )
            .when_some(advanced_rule_count, |bar, rule_count| {
                bar.child(self.render_database_advanced_filter_chip(rule_count))
            })
            .child(self.render_database_filter_add_button())
            .when(show_reset || show_save, |bar| {
                bar.child(div().flex_grow(1.0))
            })
            .when(show_save, |bar| {
                bar.child(self.render_database_filter_save_button())
            })
            .when(show_reset, |bar| {
                bar.child(self.render_database_filter_reset_button())
            })
            .into_any_element()
    }

    fn render_database_filter_reset_button(&self) -> AnyElement {
        let theme = self.resources.theme;
        let action = DatabaseFilterAction::Reset;
        div()
            .id("notion-database-filter-reset")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Reset filters")
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(theme.text_secondary))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .child("Reset")
            .into_any_element()
    }

    fn render_database_advanced_filter_chip(&self, rule_count: usize) -> AnyElement {
        let theme = self.resources.theme;
        let action = DatabaseFilterAction::ShowAdvanced(self.resources.host);
        let label = format!(
            "{rule_count} {}",
            if rule_count == 1 { "rule" } else { "rules" }
        );
        div()
            .id("notion-database-advanced-filter-chip")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(theme.text_primary, 0.10))
            .bg(alpha(theme.text_primary, 0.035))
            .role(Role::Button)
            .aria_label(label.clone())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .on_click(self.resources.filter_click_listener(action.clone()))
            .on_key_down(self.resources.filter_key_listener(action))
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .text_color(rgb(theme.text_primary))
            .child("⌘")
            .child(label)
            .child("⌄")
            .into_any_element()
    }

    fn render_database_filter_add_button(&self) -> AnyElement {
        let theme = self.resources.theme;
        let editable = self.resources.workspace_editable
            && matches!(
                self.state.effective_state(self.board).simple(),
                DatabaseSimpleFiltersState::Entries(_)
            );
        let action = DatabaseFilterAction::ShowPropertyPicker(self.resources.host);
        div()
            .id("notion-database-filter-add")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Add filter")
            .focusable()
            .tab_stop(true)
            .when(editable, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
                    .on_click(self.resources.filter_click_listener(action.clone()))
                    .on_key_down(self.resources.filter_key_listener(action))
            })
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(theme.text_secondary))
            .opacity(if editable { 1.0 } else { 0.45 })
            .child("+ Filter")
            .into_any_element()
    }

    fn render_database_filter_save_button(&self) -> AnyElement {
        let save_in_flight = self.state.save_in_flight;
        let projection_pending = self.state.applied_revision != self.state.revision
            || self.state.query_in_flight_revision == Some(self.state.revision);
        let enabled = !save_in_flight && !projection_pending;
        let action = DatabaseFilterAction::Save(self.resources.host);
        let label = "Save for everyone";
        div()
            .id("notion-database-filter-save")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(label)
            .focusable()
            .tab_stop(true)
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(0x2383e2, 0.12)))
                    .on_click(self.resources.filter_click_listener(action.clone()))
                    .on_key_down(self.resources.filter_key_listener(action))
            })
            .opacity(if enabled { 1.0 } else { 0.55 })
            .flex()
            .items_center()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0x2383e2))
            .child(label)
            .into_any_element()
    }
}

impl DatabaseFilterUiState {
    fn bar_controls(
        &self,
        board: &BoardSnapshot,
        inline: bool,
        workspace_editable: bool,
    ) -> (Option<usize>, bool, bool) {
        let advanced_rule_count = match self.effective_state(board).advanced() {
            DatabaseAdvancedFilterState::Editable(group) => Some(database_filter_rule_count(group)),
            DatabaseAdvancedFilterState::None | DatabaseAdvancedFilterState::Unsupported => None,
        };
        let persisted = self.persisted_state(board);
        let filters_changed = self
            .temporary
            .as_ref()
            .is_some_and(|temporary| temporary != &persisted);
        let draft_changed = !inline
            && self
                .draft
                .as_ref()
                .is_some_and(|draft| !draft.present_in_effective_state);
        let query_failed = self.query_failed_revision == Some(self.revision);
        let show_save = filters_changed && !query_failed && workspace_editable;
        (
            advanced_rule_count,
            filters_changed || draft_changed,
            show_save,
        )
    }
}

fn database_filter_bar_container(inline: bool) -> Div {
    div()
        .h(px(DATABASE_FILTER_BAR_HEIGHT))
        .w_full()
        .flex_none()
        .pl(px(if inline { 8.0 } else { BOARD_VIEWPORT_X }))
        .pr(px(if inline {
            8.0
        } else {
            BOARD_VIEWPORT_RIGHT_GUTTER
        }))
        .flex()
        .items_center()
        .gap(px(6.0))
}
