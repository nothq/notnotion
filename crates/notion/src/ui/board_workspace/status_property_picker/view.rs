use gpui::{
    div, px, AnyElement, Context, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::dismissible_backdrop_with_handler;

use crate::model::DatabasePropertyOption;
use crate::ui::surface::StatusPropertyPickerState;
use crate::ui::{
    alpha, column_style, rgb, rgba, view_actions::ViewActionSink, AppearanceMode, FluentBuilder,
    SurfaceState, Theme, Viewport,
};

use super::actions::StatusPropertyPickerAction;
use super::{PICKER_MARGIN, PICKER_MAX_HEIGHT, PICKER_ROW_HEIGHT, PICKER_WIDTH};

struct PickerGeometry {
    left: f32,
    top: f32,
}

struct StatusPropertyPickerRenderer<'a> {
    state: &'a StatusPropertyPickerState,
    theme: Theme,
    appearance_mode: AppearanceMode,
    viewport: Viewport,
    actions: ViewActionSink<StatusPropertyPickerAction>,
}

impl SurfaceState {
    pub(crate) fn render_status_property_picker_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        StatusPropertyPickerRenderer {
            state: self
                .notion_chrome
                .status_property_picker
                .as_ref()
                .expect("status picker overlay requires picker state"),
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            viewport: self.viewport,
            actions: ViewActionSink::new(cx, |surface, action, _, cx| {
                surface.dispatch_status_property_picker_action(action, cx);
            }),
        }
        .render()
    }
}

impl StatusPropertyPickerRenderer<'_> {
    fn render(&self) -> AnyElement {
        let state = self.state;
        let geometry = status_property_picker_geometry(self.viewport, state);
        div()
            .id("notion-status-property-picker-overlay")
            .absolute()
            .inset_0()
            .child(dismissible_backdrop_with_handler(
                div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                self.actions
                    .listener(|_, _, _| StatusPropertyPickerAction::Dismiss),
            ))
            .child(self.render_status_property_picker_panel(state, geometry))
            .into_any_element()
    }

    fn render_status_property_picker_panel(
        &self,
        state: &StatusPropertyPickerState,
        geometry: PickerGeometry,
    ) -> AnyElement {
        div()
            .id("notion-status-property-picker")
            .role(Role::Dialog)
            .aria_label("Edit Status")
            .absolute()
            .left(px(geometry.left))
            .top(px(geometry.top))
            .w(px(PICKER_WIDTH))
            .max_h(px(PICKER_MAX_HEIGHT))
            .overflow_y_scroll()
            .occlude()
            .p(px(4.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .children(
                state
                    .property
                    .options()
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        self.render_status_property_picker_option(state, index, option)
                    }),
            )
            .into_any_element()
    }

    fn render_status_property_picker_option(
        &self,
        state: &StatusPropertyPickerState,
        index: usize,
        option: &DatabasePropertyOption,
    ) -> AnyElement {
        let selected = state.current_option_id.as_deref() == Some(option.id.as_str());
        let option_id = SharedString::from(option.id.clone());
        let style = column_style(
            &option.value,
            Some(option.color.as_str()),
            self.appearance_mode,
        );
        div()
            .id(format!("notion-status-property-option-{index}"))
            .h(px(PICKER_ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(6.0))
            .when(selected, |row| row.bg(alpha(self.theme.text_primary, 0.08)))
            .when(!state.commit_in_flight, |row| {
                row.cursor_pointer()
                    .hover(|hover| hover.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            StatusPropertyPickerAction::Select(option_id.clone())
                        }),
                    )
            })
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .h(px(22.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(style.tone.pill_text_color(self.appearance_mode)))
                    .bg(rgba(style.pill))
                    .child(option.value.clone()),
            )
            .child(div().flex_grow(1.0))
            .when(selected, |row| {
                row.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(self.theme.text_secondary))
                        .child("✓"),
                )
            })
            .into_any_element()
    }
}

fn status_property_picker_geometry(
    viewport: Viewport,
    state: &StatusPropertyPickerState,
) -> PickerGeometry {
    let desired_height =
        (state.property.options().len() as f32 * PICKER_ROW_HEIGHT + 8.0).min(PICKER_MAX_HEIGHT);
    let left = state.anchor.left().as_f32().clamp(
        PICKER_MARGIN,
        (viewport.app_width() - PICKER_WIDTH - PICKER_MARGIN).max(PICKER_MARGIN),
    );
    let below = state.anchor.bottom().as_f32() + 4.0;
    let above = state.anchor.top().as_f32() - desired_height - 4.0;
    let top = if below + desired_height <= viewport.app_height() - PICKER_MARGIN {
        below
    } else {
        above.max(PICKER_MARGIN)
    };
    PickerGeometry { left, top }
}
