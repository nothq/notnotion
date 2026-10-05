use super::renderer::{InlineDatabaseViewMenuAction, InlineDatabaseViewMenuRenderer};
use super::{
    alpha, div, inline_database_view_menu_shadow, px, rgb, AnyElement, AppContext, FontWeight,
    InlineDatabaseViewMenuLayout, InlineDatabaseViewMenuState, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Rc, Role, StatefulInteractiveElement, Styled,
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
    NEW_VIEW_PICKER_HEIGHT, NEW_VIEW_PICKER_WIDTH, VIEW_MENU_MARGIN, VIEW_MENU_WIDTH,
};
use gpui::{App, Div};

impl InlineDatabaseViewMenuRenderer {
    pub(super) fn render_inline_database_new_view_picker(
        &self,
        state: &InlineDatabaseViewMenuState,
        menu_layout: InlineDatabaseViewMenuLayout,
    ) -> AnyElement {
        let (left, top) = self.inline_database_new_view_picker_position(menu_layout);
        div()
            .id(format!(
                "notion-inline-database-new-view-picker-{}",
                state.database_block_id
            ))
            .role(Role::Dialog)
            .aria_label("Add a new view")
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(NEW_VIEW_PICKER_WIDTH))
            .h(px(NEW_VIEW_PICKER_HEIGHT))
            .rounded(px(10.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(inline_database_view_menu_shadow(self.appearance_mode))
            .p(px(12.0))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(
                div()
                    .h(px(28.0))
                    .flex_none()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_muted))
                    .child("Add a new view"),
            )
            .child(self.render_inline_database_new_view_types(state))
            .into_any_element()
    }

    fn inline_database_new_view_picker_position(
        &self,
        menu_layout: InlineDatabaseViewMenuLayout,
    ) -> (f32, f32) {
        let left = (menu_layout.left + VIEW_MENU_WIDTH - 4.0).clamp(
            VIEW_MENU_MARGIN,
            (self.viewport.app_width() - NEW_VIEW_PICKER_WIDTH - VIEW_MENU_MARGIN)
                .max(VIEW_MENU_MARGIN),
        );
        let top = (menu_layout.top + 70.0).clamp(
            VIEW_MENU_MARGIN,
            (self.viewport.app_height() - NEW_VIEW_PICKER_HEIGHT - VIEW_MENU_MARGIN)
                .max(VIEW_MENU_MARGIN),
        );
        (left, top)
    }

    fn render_inline_database_new_view_types(
        &self,
        state: &InlineDatabaseViewMenuState,
    ) -> gpui::Stateful<Div> {
        div()
            .id(format!(
                "notion-inline-database-new-view-types-{}",
                state.database_block_id
            ))
            .role(Role::ListBox)
            .aria_label("View types")
            .grid()
            .grid_cols(4)
            .gap(px(4.0))
            .children(
                INLINE_DATABASE_VIEW_KINDS
                    .into_iter()
                    .map(|(label, glyph)| {
                        self.render_inline_database_new_view_type(state, label, glyph)
                    }),
            )
    }

    fn render_inline_database_new_view_type(
        &self,
        state: &InlineDatabaseViewMenuState,
        label: &'static str,
        glyph: &'static str,
    ) -> gpui::Stateful<Div> {
        div()
            .id(format!(
                "notion-inline-database-new-view-{}-{}",
                state.database_block_id,
                label.to_lowercase()
            ))
            .role(Role::ListBoxOption)
            .aria_label(label)
            .h(px(55.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.055)))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .text_size(px(12.0))
            .text_color(rgb(self.theme.text_primary))
            .child(
                div()
                    .text_size(px(17.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(glyph),
            )
            .child(label)
    }

    pub(super) fn new_inline_database_view_menu_input(
        &self,
        database_block_id: &str,
        cx: &mut App,
    ) -> gpui::Entity<TextInput> {
        let input_id = format!("notion-inline-database-view-menu-search-{database_block_id}");
        let on_change_actions = self.actions.clone();
        let on_submit_actions = self.actions.clone();
        let on_escape_actions = self.actions.clone();
        let on_up_actions = self.actions.clone();
        let on_down_actions = self.actions.clone();
        let on_change: TextInputChange = Rc::new(move |value, window, cx| {
            on_change_actions.emit(InlineDatabaseViewMenuAction::Query(value), window, cx);
        });
        let on_submit: TextInputAction = Rc::new(move |window, cx| {
            on_submit_actions.emit(InlineDatabaseViewMenuAction::Submit, window, cx);
        });
        let on_escape: TextInputAction = Rc::new(move |window, cx| {
            on_escape_actions.emit(InlineDatabaseViewMenuAction::Escape, window, cx);
        });
        let on_up: TextInputAction = Rc::new(move |window, cx| {
            on_up_actions.emit(InlineDatabaseViewMenuAction::Move(-1), window, cx);
        });
        let on_down: TextInputAction = Rc::new(move |window, cx| {
            on_down_actions.emit(InlineDatabaseViewMenuAction::Move(1), window, cx);
        });
        let props = TextInputProps::single_line("")
            .placeholder("Search for a view...")
            .request_focus(true)
            .style(self.inline_database_view_menu_input_style())
            .accessibility(input_id, "Search for a view...")
            .on_change(on_change)
            .on_submit(on_submit)
            .on_escape(on_escape)
            .on_up(on_up)
            .on_down(on_down);
        cx.new(|cx| TextInput::new(props, cx))
    }

    fn inline_database_view_menu_input_style(&self) -> TextInputStyle {
        TextInputStyle {
            height: px(28.0),
            min_height: px(28.0),
            padding_x: px(6.0),
            padding_y: px(4.0),
            radius: px(6.0),
            background: alpha(self.theme.text_primary, 0.03),
            border: alpha(self.theme.text_primary, 0.0),
            focused_border: rgb(0x2383e2).into(),
            text: rgb(self.theme.text_primary).into(),
            placeholder: rgb(self.theme.text_hint).into(),
            selection: alpha(0x2383e2, 0.28),
            caret: rgb(self.theme.text_primary).into(),
            font_size: px(14.0),
            line_height: px(20.0),
            font_family: None,
        }
    }
}

const INLINE_DATABASE_VIEW_KINDS: [(&str, &str); 11] = [
    ("Table", "▦"),
    ("Board", "▥"),
    ("Gallery", "▦"),
    ("List", "≡"),
    ("Chart", "◔"),
    ("Dashboard", "⌘"),
    ("Timeline", "▤"),
    ("Feed", "▧"),
    ("Map", "⌑"),
    ("Calendar", "▣"),
    ("Form", "▱"),
];
