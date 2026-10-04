use app_model::{AppearanceMode, SurfaceColorSpec, SurfaceTheme, DARK_APP_BACKGROUND};

use super::{ColorSpec, ColumnStyle};

#[derive(Clone, Copy)]
pub(crate) struct Theme {
    pub(crate) app_bg: u32,
    pub(crate) elevated_surface_bg: u32,
    pub(crate) surface_border: ColorSpec,
    pub(crate) text_primary: u32,
    pub(crate) text_secondary: u32,
    pub(crate) text_muted: u32,
    pub(crate) text_hint: u32,
    pub(crate) command_menu_active_bg: ColorSpec,
    /// Notion popover chrome: the far and near shadow layers, the 1 px ring,
    /// the secondary text of headers and captions, and the caption dash.
    pub(crate) menu_shadow_far: ColorSpec,
    pub(crate) menu_shadow_near: ColorSpec,
    pub(crate) menu_ring: ColorSpec,
    pub(crate) menu_secondary_text: u32,
    pub(crate) menu_dash: u32,
    pub(crate) view_tab_icon_inactive: u32,
    pub(crate) topbar_chip_text: u32,
    pub(crate) topbar_chip_icon: u32,
    pub(crate) tab_active_bg: ColorSpec,
    pub(crate) card_fill: u32,
    pub(crate) card_fill_hover: u32,
    pub(crate) card_border: ColorSpec,
    pub(crate) card_shadow: ColorSpec,
    pub(crate) calendar_control_border: ColorSpec,
    pub(crate) calendar_grid_border: u32,
    pub(crate) calendar_muted_text: u32,
    pub(crate) calendar_weekend_bg: u32,
    pub(crate) favorite_active_bg: ColorSpec,
    pub(crate) dialog_input_bg: ColorSpec,
    pub(crate) page_icon_bg: ColorSpec,
}

impl Theme {
    pub(crate) const fn for_appearance_mode(appearance_mode: AppearanceMode) -> Self {
        match appearance_mode {
            AppearanceMode::Light => Self::light(),
            AppearanceMode::Dark => Self::dark(),
        }
    }

    const fn light() -> Self {
        Self {
            app_bg: 0xffffff,
            elevated_surface_bg: 0xffffff,
            surface_border: ColorSpec::new(0x37352f, 0.075),
            text_primary: 0x2c2c2b,
            text_secondary: 0x4a4947,
            text_muted: 0x9b9a97,
            text_hint: 0x9b9a97,
            command_menu_active_bg: ColorSpec::new(0x211b17, 0.051),
            menu_shadow_far: ColorSpec::new(0x191919, 0.05),
            menu_shadow_near: ColorSpec::new(0x191919, 0.027),
            menu_ring: ColorSpec::new(0x2a1c00, 0.07),
            menu_secondary_text: 0x7d7a75,
            menu_dash: 0xbcbab6,
            view_tab_icon_inactive: 0x7c7974,
            topbar_chip_text: 0xa19e99,
            topbar_chip_icon: 0xada9a3,
            tab_active_bg: ColorSpec::new(0x7d756a, 0.12),
            card_fill: 0xffffff,
            card_fill_hover: 0xf7f6f3,
            card_border: ColorSpec::new(0x37352f, 0.05),
            card_shadow: ColorSpec::new(0x37352f, 0.014),
            calendar_control_border: ColorSpec::new(0x37352f, 0.16),
            calendar_grid_border: 0xe8e7e4,
            calendar_muted_text: 0x787774,
            calendar_weekend_bg: 0xf7f6f3,
            favorite_active_bg: ColorSpec::new(0x2a1c00, 0.07),
            dialog_input_bg: ColorSpec::new(0x37352f, 0.02),
            page_icon_bg: ColorSpec::new(0x37352f, 0.04),
        }
    }

    const fn dark() -> Self {
        Self {
            app_bg: DARK_APP_BACKGROUND,
            elevated_surface_bg: 0x252525,
            surface_border: ColorSpec::new(0xffffff, 0.08),
            text_primary: 0xf0efed,
            text_secondary: 0xd8d6d1,
            text_muted: 0xada9a3,
            text_hint: 0x65645e,
            command_menu_active_bg: ColorSpec::new(0xffffff, 0.06),
            menu_shadow_far: ColorSpec::new(0x000000, 0.45),
            menu_shadow_near: ColorSpec::new(0x000000, 0.3),
            menu_ring: ColorSpec::new(0xffffff, 0.09),
            menu_secondary_text: 0x9b9b9b,
            menu_dash: 0x6f6e6a,
            view_tab_icon_inactive: 0xada9a3,
            topbar_chip_text: 0x7c7974,
            topbar_chip_icon: 0x7d7a75,
            tab_active_bg: ColorSpec::new(0xffffeb, 0.10),
            card_fill: 0x383836,
            card_fill_hover: 0x31312f,
            card_border: ColorSpec::new(0xfffff3, 0.082),
            card_shadow: ColorSpec::new(0x191919, 0.08),
            calendar_control_border: ColorSpec::new(0xffffeb, 0.10),
            calendar_grid_border: 0x383836,
            calendar_muted_text: 0x7d7a75,
            calendar_weekend_bg: 0x202020,
            favorite_active_bg: ColorSpec::new(0xffffff, 0.08),
            dialog_input_bg: ColorSpec::new(0xffffff, 0.02),
            page_icon_bg: ColorSpec::new(0xffffff, 0.04),
        }
    }
}

impl From<Theme> for SurfaceTheme {
    fn from(theme: Theme) -> Self {
        Self {
            app_bg: theme.app_bg,
            elevated_surface_bg: theme.elevated_surface_bg,
            surface_border: SurfaceColorSpec::new(
                theme.surface_border.hex,
                theme.surface_border.opacity,
            ),
            text_primary: theme.text_primary,
            text_secondary: theme.text_secondary,
            text_muted: theme.text_muted,
            text_hint: theme.text_hint,
            command_menu_active_bg: SurfaceColorSpec::new(
                theme.command_menu_active_bg.hex,
                theme.command_menu_active_bg.opacity,
            ),
            card_fill: theme.card_fill,
            card_border: SurfaceColorSpec::new(theme.card_border.hex, theme.card_border.opacity),
            card_shadow: SurfaceColorSpec::new(theme.card_shadow.hex, theme.card_shadow.opacity),
            favorite_active_bg: SurfaceColorSpec::new(
                theme.favorite_active_bg.hex,
                theme.favorite_active_bg.opacity,
            ),
            page_icon_bg: SurfaceColorSpec::new(theme.page_icon_bg.hex, theme.page_icon_bg.opacity),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Tone {
    Gray,
    Orange,
    Blue,
    Red,
    Purple,
    Green,
}

impl Tone {
    pub const fn pill_text_color(self, appearance_mode: AppearanceMode) -> u32 {
        match appearance_mode {
            AppearanceMode::Light => match self {
                Self::Gray => 0x5c5a57,
                Self::Orange => 0x594538,
                Self::Blue => 0x0f6b94,
                Self::Red => 0xb9473a,
                Self::Purple => 0x8754c1,
                Self::Green => 0x2f7f5b,
            },
            AppearanceMode::Dark => match self {
                Self::Gray => 0xf0efed,
                Self::Orange => 0xf5ede9,
                Self::Blue => 0xe5f2fc,
                Self::Red => 0xfce9e7,
                Self::Purple => 0xf3ebf9,
                Self::Green => 0xe8f1ec,
            },
        }
    }

    pub const fn action_text_color(self) -> u32 {
        match self {
            Self::Gray => 0x8e8b86,
            Self::Orange => 0xb68965,
            Self::Blue => 0x2783de,
            Self::Red => 0xe56458,
            Self::Purple => 0xb577d6,
            Self::Green => 0x46a171,
        }
    }

    pub const fn action_border(self, appearance_mode: AppearanceMode) -> ColorSpec {
        match appearance_mode {
            AppearanceMode::Light => match self {
                Self::Gray => ColorSpec::new(0x37352f, 0.09),
                Self::Orange => ColorSpec::new(0xb68965, 0.18),
                Self::Blue => ColorSpec::new(0x479dff, 0.20),
                Self::Red => ColorSpec::new(0xff6a5e, 0.18),
                Self::Purple => ColorSpec::new(0xc97aff, 0.18),
                Self::Green => ColorSpec::new(0x77ffb3, 0.16),
            },
            AppearanceMode::Dark => match self {
                Self::Gray => ColorSpec::new(0xfffff3, 0.082),
                Self::Orange => ColorSpec::new(0xffae7a, 0.133),
                Self::Blue => ColorSpec::new(0x479dff, 0.173),
                Self::Red => ColorSpec::new(0xff6a5e, 0.173),
                Self::Purple => ColorSpec::new(0xc97aff, 0.165),
                Self::Green => ColorSpec::new(0x77ffb3, 0.118),
            },
        }
    }
}

pub(crate) fn column_style(
    title: &str,
    option_color: Option<&str>,
    appearance_mode: AppearanceMode,
) -> ColumnStyle {
    match title {
        "Recurring" | "Backlog" => backlog_column_style(appearance_mode),
        "Ready" => ready_column_style(appearance_mode),
        "In Progress" => in_progress_column_style(appearance_mode),
        "Blocked" => blocked_column_style(appearance_mode),
        "In Review" => in_review_column_style(appearance_mode),
        "Ready to Merge" => ready_to_merge_column_style(appearance_mode),
        "Done" => done_column_style(appearance_mode),
        _ => fallback_column_style(option_color, appearance_mode),
    }
}

fn backlog_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(
            if_light_hex(appearance_mode, 0x8b8986, 0xfffceb),
            if_light(appearance_mode, 0.14, 0.306),
        ),
        lane: ColorSpec::new(
            if_light_hex(appearance_mode, 0x37352f, 0xfcfcfc),
            if_light(appearance_mode, 0.014, 0.03),
        ),
        default_card_fill: if_light_card_fill(appearance_mode, 0x383836),
        tone: Tone::Gray,
    }
}

fn ready_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(
            if_light_hex(appearance_mode, 0xb68965, 0xffb884),
            if_light(appearance_mode, 0.28, 0.365),
        ),
        lane: ColorSpec::new(
            if_light_hex(appearance_mode, 0xb68965, 0xfd9e65),
            if_light(appearance_mode, 0.042, 0.05),
        ),
        default_card_fill: if_light_card_fill(appearance_mode, 0x493228),
        tone: Tone::Orange,
    }
}

fn in_progress_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    blue_column_style(
        appearance_mode,
        BlueColumnStyleConfig {
            tone: Tone::Blue,
            default_card_fill: if_light_card_fill(appearance_mode, 0x233850),
            pill_hex: 0x51a6ff,
            pill_alpha: (0.14, 0.494),
            lane_hex: 0x298bfd,
            lane_alpha: (0.048, 0.063),
        },
    )
}

fn blocked_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(0xff7469, if_light(appearance_mode, 0.14, 0.525)),
        lane: ColorSpec::new(0xfb6b6b, if_light(appearance_mode, 0.042, 0.047)),
        default_card_fill: if_light_card_fill(appearance_mode, 0x4e2d2a),
        tone: Tone::Red,
    }
}

fn in_review_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(0xd093ff, if_light(appearance_mode, 0.14, 0.427)),
        lane: ColorSpec::new(0xc465fd, if_light(appearance_mode, 0.042, 0.05)),
        default_card_fill: if_light_card_fill(appearance_mode, 0x40314a),
        tone: Tone::Purple,
    }
}

fn ready_to_merge_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    blue_column_style(
        appearance_mode,
        BlueColumnStyleConfig {
            tone: Tone::Blue,
            default_card_fill: if_light_card_fill(appearance_mode, 0x233850),
            pill_hex: 0x51a6ff,
            pill_alpha: (0.14, 0.494),
            lane_hex: 0x298bfd,
            lane_alpha: (0.048, 0.063),
        },
    )
}

fn done_column_style(appearance_mode: AppearanceMode) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(0x71ffaf, if_light(appearance_mode, 0.13, 0.337)),
        lane: ColorSpec::new(0x53ff8c, if_light(appearance_mode, 0.036, 0.035)),
        default_card_fill: if_light_card_fill(appearance_mode, 0x263d30),
        tone: Tone::Green,
    }
}

fn fallback_column_style(
    option_color: Option<&str>,
    appearance_mode: AppearanceMode,
) -> ColumnStyle {
    match option_color {
        Some("brown") => ready_column_style(appearance_mode),
        Some("blue") => in_progress_column_style(appearance_mode),
        Some("purple") => in_review_column_style(appearance_mode),
        Some("red") => blocked_column_style(appearance_mode),
        Some("green") => done_column_style(appearance_mode),
        _ => backlog_column_style(appearance_mode),
    }
}

fn blue_column_style(
    appearance_mode: AppearanceMode,
    config: BlueColumnStyleConfig,
) -> ColumnStyle {
    ColumnStyle {
        pill: ColorSpec::new(
            config.pill_hex,
            if_light(appearance_mode, config.pill_alpha.0, config.pill_alpha.1),
        ),
        lane: ColorSpec::new(
            config.lane_hex,
            if_light(appearance_mode, config.lane_alpha.0, config.lane_alpha.1),
        ),
        default_card_fill: config.default_card_fill,
        tone: config.tone,
    }
}

pub(crate) const fn if_light(appearance_mode: AppearanceMode, light: f32, dark: f32) -> f32 {
    match appearance_mode {
        AppearanceMode::Light => light,
        AppearanceMode::Dark => dark,
    }
}

pub(crate) const fn if_light_hex(appearance_mode: AppearanceMode, light: u32, dark: u32) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => light,
        AppearanceMode::Dark => dark,
    }
}

pub(crate) const fn if_light_card_fill(appearance_mode: AppearanceMode, dark: u32) -> Option<u32> {
    match appearance_mode {
        AppearanceMode::Light => Some(0xffffff),
        AppearanceMode::Dark => Some(dark),
    }
}

#[derive(Clone, Copy)]
struct BlueColumnStyleConfig {
    tone: Tone,
    default_card_fill: Option<u32>,
    pill_hex: u32,
    pill_alpha: (f32, f32),
    lane_hex: u32,
    lane_alpha: (f32, f32),
}
