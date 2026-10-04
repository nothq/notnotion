use super::{
    alpha, div, img, px, rgb, AnyElement, AppContext, Div, FluentBuilder,
    InlineDatabaseToolbarTarget, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Rc, StatefulInteractiveElement, Styled, TextInput, TextInputAction,
    TextInputChange, TextInputProps, TextInputStyle,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(crate) fn render_toolbar_search_button(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        div()
            .id(database_search_toggle_id(inline_target))
            .size(px(28.0))
            .rounded(px(6.0))
            .role(gpui::Role::Button)
            .aria_label("Search")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::ToggleSearch
                }),
            )
            .child(
                img(self.icons.toolbar_search.render(cx))
                    .relative()
                    .left(px(-1.0))
                    .size(px(14.0)),
            )
    }

    pub(crate) fn render_toolbar_search_input(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let query_empty = self.database_search.query.is_empty();
        let input_width = if query_empty { 150.0 } else { 126.0 };
        let input = self.database_search_input_entity(inline_target, cx);
        div()
            .id(database_search_input_wrapper_id(inline_target))
            .w(px(150.0))
            .h(px(28.0))
            .flex()
            .items_center()
            .child(div().w(px(input_width)).h_full().child(input))
            .when(!query_empty, |this| {
                this.child(self.render_database_search_clear_button(inline_target, cx))
            })
    }

    fn database_search_input_entity(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> gpui::Entity<TextInput> {
        let props = TextInputProps::single_line(self.database_search.query.clone())
            .placeholder("Type to search…")
            .request_focus(self.database_search.take_focus_request())
            .style(TextInputStyle {
                height: px(28.0),
                min_height: px(28.0),
                padding_x: px(0.0),
                padding_y: px(0.0),
                radius: px(0.0),
                background: alpha(self.theme.app_bg, 0.0),
                border: alpha(self.theme.app_bg, 0.0),
                focused_border: alpha(self.theme.app_bg, 0.0),
                text: rgb(self.theme.text_primary).into(),
                placeholder: rgb(self.theme.text_muted).into(),
                selection: alpha(0x2383e2, 0.28),
                caret: rgb(self.theme.text_primary).into(),
                font_size: px(14.0),
                line_height: px(14.0),
                font_family: None,
            })
            .accessibility(database_search_input_id(inline_target), "Type to search…")
            .on_change(self.database_search_on_change())
            .on_escape(self.database_search_on_escape());
        if let Some(input) = self.database_search.input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.database_search
            .input
            .borrow_mut()
            .replace(input.clone());
        input
    }

    fn database_search_on_change(&self) -> TextInputChange {
        let actions = self.actions.clone();
        Rc::new(move |value, window, cx| {
            actions.emit(
                BoardToolbarAction::SetSearchQuery(value.to_string()),
                window,
                cx,
            );
        })
    }

    fn database_search_on_escape(&self) -> TextInputAction {
        let actions = self.actions.clone();
        Rc::new(move |window, cx| actions.emit(BoardToolbarAction::CloseSearch, window, cx))
    }

    fn render_database_search_clear_button(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        let existing_clear_focus = self.database_search.clear_focus.borrow().clone();
        let clear_focus = existing_clear_focus.unwrap_or_else(|| {
            let focus = cx.focus_handle();
            self.database_search
                .clear_focus
                .borrow_mut()
                .replace(focus.clone());
            focus
        });
        (div()
            .id(database_search_clear_id(inline_target))
            .size(px(24.0))
            .rounded(px(5.0))
            .role(gpui::Role::Button)
            .aria_label("Clear Input")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .track_focus(&clear_focus)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(move |_: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        window.focus(&clear_focus, cx);
                        BoardToolbarAction::ClearSearch
                    }),
            )
            .child(img(self.icons.close.render(cx)).size(px(12.0))))
        .into_any_element()
    }
}

fn database_search_toggle_id(target: Option<&InlineDatabaseToolbarTarget>) -> String {
    database_search_element_id("toggle", target)
}

fn database_search_input_wrapper_id(target: Option<&InlineDatabaseToolbarTarget>) -> String {
    database_search_element_id("field", target)
}

fn database_search_input_id(target: Option<&InlineDatabaseToolbarTarget>) -> String {
    database_search_element_id("input", target)
}

fn database_search_clear_id(target: Option<&InlineDatabaseToolbarTarget>) -> String {
    database_search_element_id("clear", target)
}

fn database_search_element_id(
    element: &str,
    target: Option<&InlineDatabaseToolbarTarget>,
) -> String {
    match target {
        Some(target) => format!(
            "notion-inline-database-search-{element}-{}",
            target.database_block_id
        ),
        None => format!("notion-database-search-{element}"),
    }
}
