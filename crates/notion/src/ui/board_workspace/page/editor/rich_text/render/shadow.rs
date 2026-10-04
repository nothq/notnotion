use gpui::{point, px, BoxShadow};

use crate::ui::{alpha, AppearanceMode};

pub(super) fn page_rich_text_shadow(appearance_mode: AppearanceMode) -> Vec<BoxShadow> {
    if appearance_mode == AppearanceMode::Dark {
        return vec![
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
        ];
    }
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.05),
            offset: point(px(0.0), px(20.0)),
            blur_radius: px(24.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x2a1c00, 0.07),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}
