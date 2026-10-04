use super::super::property_icons::render_property_picker_option_icon;
use super::{
    ai_autofill_property_icon_kind, ai_autofill_shadow, alpha, div, img, px, rgb, AiAutofillAction,
    AiAutofillLayout, AiAutofillView, AnyElement, App, DatabaseProperty, ElementId, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled, MODE_MENU_HEIGHT, MODE_MENU_WIDTH, PROPERTY_MENU_WIDTH,
    PROPERTY_ROW_GAP, PROPERTY_ROW_HEIGHT,
};

impl AiAutofillView<'_> {
    pub(super) fn render_ai_autofill_property_menu(
        &self,
        layout: AiAutofillLayout,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id("notion-ai-autofill-property-menu")
            .absolute()
            .left(px(layout.property_left))
            .top(px(layout.property_top))
            .w(px(PROPERTY_MENU_WIDTH))
            .p(px(4.0))
            .rounded(px(10.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(ai_autofill_shadow())
            .role(Role::Dialog)
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .flex()
            .flex_col()
            .gap(px(PROPERTY_ROW_GAP))
            .children(
                self.state
                    .properties
                    .iter()
                    .enumerate()
                    .map(|(index, property)| {
                        self.render_ai_autofill_property_row(index, property, cx)
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    fn render_ai_autofill_property_row(
        &self,
        property_index: usize,
        property: &DatabaseProperty,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(ElementId::Name(
                format!("notion-ai-autofill-property-{property_index}").into(),
            ))
            .w_full()
            .h(px(PROPERTY_ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(format!("Configure AI Autofill for {}", property.label))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    AiAutofillAction::SelectProperty(property_index)
                }),
            )
            .child(render_property_picker_option_icon(
                *self.theme,
                self.appearance_mode,
                ai_autofill_property_icon_kind(&property.property_type),
                false,
                cx,
            ))
            .child(
                div()
                    .flex_grow(1.0)
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child(property.label.clone()),
            )
            .child(img(self.icons.ai_autofill_chevron.render(cx)).size(px(16.0)))
            .into_any_element()
    }

    pub(super) fn render_ai_autofill_mode_menu(
        &self,
        property_index: usize,
        layout: AiAutofillLayout,
        cx: &mut App,
    ) -> AnyElement {
        let property = self
            .state
            .properties
            .get(property_index)
            .expect("selected AI Autofill property must exist");
        div()
            .id("notion-ai-autofill-mode-menu")
            .absolute()
            .left(px(layout.mode_left))
            .top(px(layout.mode_top))
            .w(px(MODE_MENU_WIDTH))
            .h(px(MODE_MENU_HEIGHT))
            .p(px(4.0))
            .rounded(px(10.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(ai_autofill_shadow())
            .role(Role::Dialog)
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(self.render_ai_autofill_basic_mode(property, cx))
            .child(self.render_ai_autofill_custom_agent_mode(cx))
            .into_any_element()
    }

    fn render_ai_autofill_basic_mode(
        &self,
        _property: &DatabaseProperty,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id("notion-ai-autofill-basic")
            .w_full()
            .h(px(60.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .opacity(0.4)
            .flex()
            .items_start()
            .gap(px(8.0))
            .child(
                img(self.icons.toolbar_magic_wand.render(cx))
                    .mt(px(4.0))
                    .size(px(16.0)),
            )
            .child(
                div()
                    .pt(px(2.0))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .line_height(px(20.0))
                            .text_color(rgb(self.theme.text_primary))
                            .child("Basic"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(rgb(self.theme.text_muted))
                            .child("Not available for this property type"),
                    ),
            )
            .into_any_element()
    }

    fn render_ai_autofill_custom_agent_mode(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-autofill-custom-agent")
            .w_full()
            .h(px(79.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Use a Custom Agent for AI Autofill")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_start()
            .gap(px(8.0))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    AiAutofillAction::OpenTrial
                }),
            )
            .child(
                img(self.icons.toolbar_magic_wand.render(cx))
                    .mt(px(6.0))
                    .size(px(16.0)),
            )
            .child(self.render_ai_autofill_custom_agent_copy())
            .into_any_element()
    }

    fn render_ai_autofill_custom_agent_copy(&self) -> AnyElement {
        div()
            .pt(px(4.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .line_height(px(20.0))
                            .text_color(rgb(self.theme.text_primary))
                            .child("Custom Agent"),
                    )
                    .child(
                        div()
                            .px(px(5.0))
                            .h(px(16.0))
                            .rounded(px(4.0))
                            .bg(alpha(0x2783de, 0.18))
                            .text_size(px(10.0))
                            .line_height(px(16.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x4aa8ff))
                            .child("New"),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("Uses advanced models to fill properties based on web search")
                    .child(" and your connected workspace."),
            )
            .into_any_element()
    }
}
