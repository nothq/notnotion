use super::board::columns::{host::BoardColumnAction, BoardColumnsRenderer};
use super::{
    alpha, div, img, point, px, relative, rgb, rgba, AppearanceMode, BoxShadow, Card, CardLocation,
    Div, DragState, FluentBuilder, FontWeight, HoveredCardAction, IconAsset, InteractiveElement,
    MouseButton, ParentElement, Styled, CARD_WIDTH,
};
use gpui::App;
use gpui::{IntoElement, StatefulInteractiveElement};
use gpui_components::tooltip::Tooltip;

impl BoardColumnsRenderer<'_> {
    pub(crate) fn render_card_actions(
        &self,
        location: CardLocation,
        hovered_action: Option<HoveredCardAction>,
        cx: &mut App,
    ) -> Div {
        div()
            .absolute()
            .top(px(8.0))
            .right(px(8.0))
            .h(px(24.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(vec![BoxShadow {
                color: rgba(self.theme.card_shadow),
                offset: point(px(0.0), px(4.0)),
                blur_radius: px(12.0),
                spread_radius: px(-2.0),
                inset: false,
            }])
            .flex()
            .items_center()
            .child(self.render_card_action_button(
                location,
                CardActionButton {
                    action: HoveredCardAction::Edit,
                    hovered_action,
                    icon: &self.icons.card_action_edit,
                },
                cx,
            ))
            .child(self.render_card_action_button(
                location,
                CardActionButton {
                    action: HoveredCardAction::More,
                    hovered_action,
                    icon: &self.icons.card_action_ellipsis,
                },
                cx,
            ))
    }

    fn render_card_action_button(
        &self,
        location: CardLocation,
        button_spec: CardActionButton<'_>,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let action_segment = match button_spec.action {
            HoveredCardAction::Edit => "edit",
            HoveredCardAction::More => "more",
        };
        let mut button = div()
            .id(format!(
                "notion-card-action-{}-{}-{action_segment}",
                location.column_index, location.card_index
            ))
            .h_full()
            .px(px(6.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .bg(self.card_action_background(button_spec.action, button_spec.hovered_action))
            .child(img(button_spec.icon.render(cx)).size(px(16.0)));
        button
            .interactivity()
            .on_hover(self.actions.listener(move |is_hovered: &bool, _, _| {
                BoardColumnAction::HoverAction {
                    location,
                    action: button_spec.action,
                    hovered: *is_hovered,
                }
            }));
        button
            .interactivity()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            });
        if button_spec.action == HoveredCardAction::More {
            button = button.tooltip(Tooltip::text("Rename, delete, move to and more..."));
        }
        button.into_any_element()
    }

    fn card_action_background(
        &self,
        action: HoveredCardAction,
        hovered_action: Option<HoveredCardAction>,
    ) -> gpui::Hsla {
        if hovered_action == Some(action) {
            rgba(self.theme.tab_active_bg)
        } else {
            alpha(0xffffff, 0.0)
        }
    }

    pub(crate) fn render_drag_ghost(&self, drag: DragState, cx: &mut App) -> Div {
        let column = &self.columns[drag.source_column];
        let fill = drag
            .card
            .fill_override
            .or(column.style.default_card_fill)
            .unwrap_or(self.theme.card_fill);

        div()
            .absolute()
            .left(drag.pointer_position.x - drag.cursor_offset.x)
            .top(drag.pointer_position.y - drag.cursor_offset.y)
            .w(px(CARD_WIDTH))
            .h(px(drag.card.height))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(self.theme.card_border))
            .bg(alpha(fill, 0.96))
            .shadow(vec![BoxShadow {
                color: alpha(
                    0x000000,
                    if self.appearance_mode == AppearanceMode::Light {
                        0.18
                    } else {
                        0.28
                    },
                ),
                offset: point(px(0.0), px(18.0)),
                blur_radius: px(34.0),
                spread_radius: px(-6.0),
                inset: false,
            }])
            .child(self.render_drag_ghost_body(&drag.card, cx))
    }

    fn render_drag_ghost_body(&self, card: &Card, cx: &mut App) -> Div {
        div()
            .size_full()
            .flex()
            .items_start()
            .px(px(10.0))
            .py(px(8.0))
            .when(card.has_content, |this| {
                this.child(self.render_drag_ghost_page_icon(cx))
            })
            .child(self.render_drag_ghost_title(card))
    }

    fn render_drag_ghost_page_icon(&self, cx: &mut App) -> Div {
        div()
            .w(px(24.0))
            .h(px(24.0))
            .ml(px(-2.0))
            .mr(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .child(img(self.icons.page.render(cx)).w(px(18.0)).h(px(18.0)))
    }

    fn render_drag_ghost_title(&self, card: &Card) -> Div {
        div()
            .w(px(if card.has_content { 214.0 } else { 240.0 }))
            .text_size(px(15.0))
            .line_height(relative(1.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .child(card.title.clone())
    }
}

#[derive(Clone, Copy)]
struct CardActionButton<'a> {
    action: HoveredCardAction,
    hovered_action: Option<HoveredCardAction>,
    icon: &'a IconAsset,
}
