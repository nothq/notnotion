use crate::model::DatabaseProperty;
use crate::ui::surface::AiAutofillDialogState;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{
    alpha, div, img, notion_ai_face_image, point, px, rgb, AnyElement, AppearanceMode, BoxShadow,
    Div, FluentBuilder, FontWeight, IconSet, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, PropertyPickerIconKind, StatefulInteractiveElement, Styled,
    Theme, Viewport,
};
use gpui::{App, ElementId, Role, Toggled};

mod host;
mod marketing;
mod overlay;
mod picker;
mod trial;
mod trial_icons;

const MAIN_WIDTH: f32 = 290.0;
const MAIN_HEIGHT: f32 = 352.0;
const PROPERTY_MENU_WIDTH: f32 = 220.0;
const PROPERTY_ROW_HEIGHT: f32 = 28.0;
const PROPERTY_ROW_GAP: f32 = 1.0;
const MODE_MENU_WIDTH: f32 = 240.0;
const MODE_MENU_HEIGHT: f32 = 148.0;
const VIEWPORT_MARGIN: f32 = 12.0;

#[derive(Clone, Copy)]
pub(super) enum AiAutofillAction {
    Dismiss,
    OpenPropertyPicker,
    SelectProperty(usize),
    OpenTrial,
    CloseTrial,
    ToggleTrialConsent,
    Escape,
}

pub(super) struct AiAutofillView<'a> {
    state: &'a AiAutofillDialogState,
    theme: &'a Theme,
    icons: &'a IconSet,
    appearance_mode: AppearanceMode,
    viewport: &'a Viewport,
    actions: ViewActionSink<AiAutofillAction>,
}

#[derive(Clone, Copy)]
enum AiAutofillTimelineKind {
    Today,
    Reminder,
    Upgrade,
}

#[derive(Clone, Copy)]
enum AiAutofillMarketingArt {
    CustomAgents,
    NotionAi,
    MeetingNotes,
    Integrations,
}

#[derive(Clone, Copy)]
struct AiAutofillLayout {
    main_left: f32,
    main_top: f32,
    property_left: f32,
    property_top: f32,
    mode_left: f32,
    mode_top: f32,
}

impl AiAutofillLayout {
    fn new(state: &AiAutofillDialogState, viewport_width: f32, viewport_height: f32) -> Self {
        let right_candidate = state.anchor.right().as_f32() + 7.0;
        let main_left = if right_candidate + MAIN_WIDTH <= viewport_width - VIEWPORT_MARGIN {
            right_candidate
        } else {
            (state.anchor.left().as_f32() - MAIN_WIDTH - 7.0).max(VIEWPORT_MARGIN)
        };
        let main_top = if state.inline_database {
            81.0
        } else {
            (state.anchor.bottom().as_f32() + 3.0).clamp(
                VIEWPORT_MARGIN,
                (viewport_height - MAIN_HEIGHT - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN),
            )
        };
        let property_height = state.properties.len() as f32 * PROPERTY_ROW_HEIGHT
            + state.properties.len().saturating_sub(1) as f32 * PROPERTY_ROW_GAP
            + 8.0;
        let property_left = (main_left + 12.0).clamp(
            VIEWPORT_MARGIN,
            (viewport_width - PROPERTY_MENU_WIDTH - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN),
        );
        let property_top = (main_top + MAIN_HEIGHT - 12.0).clamp(
            VIEWPORT_MARGIN,
            (viewport_height - property_height - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN),
        );
        let mode_right = property_left + PROPERTY_MENU_WIDTH - 4.0;
        let mode_left = if mode_right + MODE_MENU_WIDTH <= viewport_width - VIEWPORT_MARGIN {
            mode_right
        } else {
            (property_left - MODE_MENU_WIDTH + 4.0).max(VIEWPORT_MARGIN)
        };
        let selected_index = state.selected_property_index.unwrap_or(0) as f32;
        let mode_top = (property_top + 4.0 + selected_index * 29.0).clamp(
            VIEWPORT_MARGIN,
            (viewport_height - MODE_MENU_HEIGHT - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN),
        );
        Self {
            main_left,
            main_top,
            property_left,
            property_top,
            mode_left,
            mode_top,
        }
    }
}

fn ai_autofill_property_icon_kind(property_type: &str) -> PropertyPickerIconKind {
    match property_type {
        "number" | "auto_increment_id" => PropertyPickerIconKind::Number,
        "checkbox" => PropertyPickerIconKind::Checkbox,
        "formula" => PropertyPickerIconKind::Formula,
        "rollup" => PropertyPickerIconKind::Rollup,
        "created_time" | "last_edited_time" | "last_visited_time" => PropertyPickerIconKind::Time,
        "created_by" | "last_edited_by" | "person" => PropertyPickerIconKind::Person,
        "relation" => PropertyPickerIconKind::Relation,
        "date" => PropertyPickerIconKind::Date,
        "select" | "multi_select" => PropertyPickerIconKind::List,
        "status" => PropertyPickerIconKind::Status,
        _ => PropertyPickerIconKind::Text,
    }
}

fn ai_autofill_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x383836, 1.0),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.20),
            offset: point(px(0.0), px(14.0)),
            blur_radius: px(28.0),
            spread_radius: px(-6.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.118),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-1.0),
            inset: false,
        },
    ]
}

fn ai_autofill_modal_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x000000, 0.24),
            offset: point(px(0.0), px(24.0)),
            blur_radius: px(48.0),
            spread_radius: px(-8.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 0.16),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
