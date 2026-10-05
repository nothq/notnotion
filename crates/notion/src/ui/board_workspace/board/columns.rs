use super::super::{
    alpha, div, img, point, px, relative, rgb, rgba, AnyElement, BoxShadow, Card, CardLocation,
    ColumnState, Div, FluentBuilder, FontWeight, HoveredCard, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Styled, Tone, COLUMN_WIDTH,
};
use crate::ui::surface::BoardViewState;
use gpui::App;

mod context;
mod new_page;

use context::CardRenderContext;

impl BoardColumnsRenderer<'_> {
    pub(crate) fn render_column(&self, column_index: usize, cx: &mut App) -> AnyElement {
        self.render_column_with_metrics(column_index, 16.0, 36.0, cx)
    }

    pub(crate) fn render_inline_column(&self, column_index: usize, cx: &mut App) -> AnyElement {
        self.render_column_with_metrics(column_index, 8.0, 40.0, cx)
    }

    fn render_column_with_metrics(
        &self,
        column_index: usize,
        top_padding: f32,
        header_height: f32,
        cx: &mut App,
    ) -> AnyElement {
        let column = &self.columns[column_index];
        let cards = self.render_column_cards(column_index, column, cx);

        (div()
            .w(px(COLUMN_WIDTH))
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(top_padding))
            .child(
                div()
                    .w_full()
                    .rounded(px(10.0))
                    .bg(rgba(column.style.lane))
                    .child(
                        (self.render_column_header_with_height(
                            column_index,
                            column,
                            header_height,
                            cx,
                        ))
                        .into_any_element(),
                    )
                    .child(self.render_column_lane(column_index, column, cards, cx)),
            ))
        .into_any_element()
    }

    fn render_column_cards(
        &self,
        column_index: usize,
        column: &ColumnState,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        column
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| self.database_search.card_is_visible(card))
            .map(|(card_index, card)| {
                let location = CardLocation::new(column_index, card_index);
                self.render_card(
                    card,
                    CardRenderContext::new(
                        location,
                        column.style.default_card_fill,
                        column.style.tone,
                        self.board_view.is_drag_source(location),
                    ),
                    cx,
                )
            })
            .collect()
    }

    fn render_column_lane(
        &self,
        column_index: usize,
        column: &ColumnState,
        cards: Vec<AnyElement>,
        cx: &mut App,
    ) -> AnyElement {
        (div()
            .w_full()
            .h(px(self.database_search.visible_lane_height(&column.cards)))
            .px(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap_2()
            .children(cards)
            .child(self.render_new_page_affordance(column_index, column.style.tone, cx)))
        .into_any_element()
    }

    fn render_column_header_with_height(
        &self,
        column_index: usize,
        column: &ColumnState,
        height: f32,
        cx: &mut App,
    ) -> Div {
        let mut header = div()
            .w_full()
            .h(px(height))
            .px(px(8.0))
            .flex()
            .items_center()
            .child(self.render_column_pill(column))
            .child(
                div()
                    .h(px(20.0))
                    .px(px(6.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .text_size(px(14.0))
                    .text_color(rgb(column.style.tone.action_text_color()))
                    .child(
                        self.database_search
                            .visible_card_count(&column.cards)
                            .to_string(),
                    ),
            )
            .child(div().flex_grow(1.0))
            .when(
                self.board_view.hovered_column_header == Some(column_index),
                |this| this.child(self.render_column_header_actions(cx)),
            );
        header
            .interactivity()
            .on_hover(self.actions.listener(move |is_hovered: &bool, _, _| {
                BoardColumnAction::HoverHeader {
                    column_index,
                    hovered: *is_hovered,
                }
            }));
        header
    }

    pub(crate) fn render_column_header_actions(&self, cx: &mut App) -> Div {
        div()
            .h_full()
            .pr(px(8.0))
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img(self.icons.card_action_ellipsis.render(cx)).size(px(16.0))),
            )
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .ml(px(4.0))
                    .rounded(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img(self.icons.header_plus.render(cx)).size(px(16.0))),
            )
    }

    pub(crate) fn render_column_pill(&self, column: &ColumnState) -> AnyElement {
        (div()
            .h(px(26.0))
            .px(px(3.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .flex_none()
            .self_start()
            .child(
                div()
                    .h(px(20.0))
                    .pl(px(7.0))
                    .pr(px(9.0))
                    .rounded(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .text_size(px(14.0))
                    .text_color(rgb(column.style.tone.pill_text_color(self.appearance_mode)))
                    .bg(rgba(column.style.pill))
                    .child(
                        div()
                            .size(px(8.0))
                            .rounded_full()
                            .bg(rgb(column.style.pill.hex)),
                    )
                    .child(column.title.clone()),
            ))
        .into_any_element()
    }

    fn render_card(&self, card: &Card, context: CardRenderContext, cx: &mut App) -> AnyElement {
        let hovered_card = self.board_view.hovered_card_state(context.location);
        let fill = self.card_fill(card, context.default_fill, hovered_card.is_some());
        let mut card_view = self
            .card_shell(card, fill, context.tone)
            .child(self.render_card_body(card, cx));
        if let Some(hovered) = hovered_card {
            card_view =
                card_view.child(self.render_card_actions(context.location, hovered.action, cx));
        }
        let card_view = self.attach_card_interactivity(card_view, context);
        (card_view).into_any_element()
    }

    fn card_fill(&self, card: &Card, default_fill: Option<u32>, is_hovered: bool) -> u32 {
        let fill = card
            .fill_override
            .or(default_fill)
            .unwrap_or(self.theme.card_fill);
        if is_hovered && fill == self.theme.card_fill {
            self.theme.card_fill_hover
        } else {
            fill
        }
    }

    fn card_shell(&self, card: &Card, fill: u32, tone: Tone) -> gpui::Stateful<Div> {
        div()
            .id(format!("card-{}", card.block_id))
            .relative()
            .h(px(card.height))
            .w_full()
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(tone.action_border(self.appearance_mode)))
            .bg(rgb(fill))
            .shadow(vec![BoxShadow {
                color: rgba(self.theme.card_shadow),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(4.0),
                spread_radius: px(0.0),
                inset: false,
            }])
    }

    fn render_card_body(&self, card: &Card, cx: &mut App) -> Div {
        div()
            .size_full()
            .flex()
            .items_center()
            .pl(px(11.0))
            .pr(px(9.0))
            .pt(px(8.0))
            .pb(px(8.0))
            .when_some(card.icon.as_ref(), |this, icon| {
                this.child(self.render_card_explicit_icon(icon, cx))
            })
            .when(card.icon.is_none() && card.has_content, |this| {
                this.child(self.render_card_page_icon(cx))
            })
            .child(self.render_card_title(card))
    }

    fn render_card_explicit_icon(&self, icon: &crate::ui::PageShellIcon, cx: &mut App) -> Div {
        div()
            .w(px(24.0))
            .h(px(24.0))
            .ml(px(-2.0))
            .mr(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .child(self.page_icons.render(icon, 18.0, cx))
    }

    fn render_card_page_icon(&self, cx: &mut App) -> Div {
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

    fn render_card_title(&self, card: &Card) -> Div {
        div()
            .w(px(if card.icon.is_some() || card.has_content {
                214.0
            } else {
                240.0
            }))
            .text_size(px(15.0))
            .line_height(relative(1.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .child(card.title.clone())
    }

    fn attach_card_interactivity(
        &self,
        mut card_view: gpui::Stateful<Div>,
        context: CardRenderContext,
    ) -> gpui::Stateful<Div> {
        card_view.interactivity().on_hover(self.actions.listener(
            move |is_hovered: &bool, _, _| BoardColumnAction::HoverCard {
                location: context.location,
                hovered: *is_hovered,
            },
        ));
        if context.is_drag_source {
            return card_view.opacity(0.16);
        }
        card_view
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |event: &MouseDownEvent, _, _| {
                    BoardColumnAction::MouseDown {
                        location: context.location,
                        position: event.position,
                    }
                }),
            )
            .cursor_pointer()
    }
}

impl BoardViewState {
    fn is_drag_source(&self, location: CardLocation) -> bool {
        self.drag.as_ref().is_some_and(|drag| {
            drag.moved
                && drag.source_column == location.column_index
                && drag.source_index == location.card_index
        })
    }

    fn hovered_card_state(&self, location: CardLocation) -> Option<HoveredCard> {
        self.hovered_card.filter(|hovered| {
            hovered.column_index == location.column_index
                && hovered.card_index == location.card_index
        })
    }
}

pub(in crate::ui::board_workspace) mod host;
mod renderer;
use host::BoardColumnAction;
pub(in crate::ui) use renderer::BoardColumnsRenderer;
