use super::{
    alpha, point, px, rgb, AppearanceMode, Bounds, BoxShadow, DateUndatedDialogLayout, Pixels,
    DATE_UNDATED_BOTTOM_PADDING, DATE_UNDATED_DIALOG_MARGIN, DATE_UNDATED_DIALOG_MAX_HEIGHT,
    DATE_UNDATED_DIALOG_WIDTH, DATE_UNDATED_INSTRUCTION_HEIGHT, DATE_UNDATED_ROW_GAP,
    DATE_UNDATED_ROW_HEIGHT, DATE_UNDATED_SEARCH_HEADER_HEIGHT, DATE_UNDATED_VISIBLE_ROW_COUNT,
};

pub(super) struct DateUndatedLayoutInput {
    pub(super) anchor: Bounds<Pixels>,
    pub(super) viewport_width: f32,
    pub(super) viewport_height: f32,
    pub(super) row_count: usize,
    pub(super) pending: bool,
    pub(super) loaded: bool,
}

pub(super) fn date_undated_dialog_layout(input: DateUndatedLayoutInput) -> DateUndatedDialogLayout {
    let desired_list_height = if input.row_count > 0 {
        let visible_row_count = input.row_count.min(DATE_UNDATED_VISIBLE_ROW_COUNT);
        visible_row_count as f32 * DATE_UNDATED_ROW_HEIGHT
            + visible_row_count.saturating_sub(1) as f32 * DATE_UNDATED_ROW_GAP
    } else if input.pending && !input.loaded {
        2.0 * DATE_UNDATED_ROW_HEIGHT + DATE_UNDATED_ROW_GAP
    } else {
        64.0
    };
    let fixed_height = DATE_UNDATED_SEARCH_HEADER_HEIGHT
        + DATE_UNDATED_INSTRUCTION_HEIGHT
        + DATE_UNDATED_BOTTOM_PADDING;
    let desired_height = (fixed_height + desired_list_height)
        .ceil()
        .min(DATE_UNDATED_DIALOG_MAX_HEIGHT);
    let height =
        desired_height.min((input.viewport_height - DATE_UNDATED_DIALOG_MARGIN * 2.0).max(0.0));
    let list_height = (height - fixed_height).max(0.0);
    let left = input.anchor.left().as_f32().clamp(
        DATE_UNDATED_DIALOG_MARGIN,
        (input.viewport_width - DATE_UNDATED_DIALOG_WIDTH - DATE_UNDATED_DIALOG_MARGIN)
            .max(DATE_UNDATED_DIALOG_MARGIN),
    );
    let below = input.anchor.bottom().as_f32() + 4.0;
    let above = input.anchor.top().as_f32() - height - 4.0;
    let top = if below + height <= input.viewport_height - DATE_UNDATED_DIALOG_MARGIN {
        below
    } else {
        above.max(DATE_UNDATED_DIALOG_MARGIN)
    };
    DateUndatedDialogLayout {
        left,
        top,
        height,
        list_height,
    }
}

pub(super) fn date_undated_dialog_background(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(if appearance_mode == AppearanceMode::Light {
        0xffffff
    } else {
        0x252525
    })
    .into()
}

pub(super) fn date_undated_dialog_border(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(if appearance_mode == AppearanceMode::Light {
        0xd9d9d7
    } else {
        0x383836
    })
    .into()
}

pub(super) fn date_undated_dialog_shadow(appearance_mode: AppearanceMode) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(
                0x000000,
                if appearance_mode == AppearanceMode::Light {
                    0.12
                } else {
                    0.28
                },
            ),
            offset: point(px(0.0), px(14.0)),
            blur_radius: px(28.0),
            spread_radius: px(-6.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 0.12),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-1.0),
            inset: false,
        },
    ]
}
