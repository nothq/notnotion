//! notnotion's mention chrome against the numbers measured on Notion desktop.
//!
//! The reference JSON lives in `crates/notion/reference/notion-7.31.3/`; a
//! drift shows up here as a named failure rather than a fuzzy image diff.

use serde_json::Value;

use app_model::AppearanceMode;

use super::super::rich_text::{page_mention_ghost_foreground, page_mention_input_pill_background};
use chrono::{NaiveDate, NaiveTime};

use super::ghost::{
    page_mention_ghost_completion, PAGE_MENTION_PILL_PADDING_X, PAGE_MENTION_PILL_RADIUS,
};
use super::picker::{
    PAGE_MENTION_PICKER_CELL, PAGE_MENTION_PICKER_DAY_BUTTON, PAGE_MENTION_PICKER_FIELD_HEIGHT,
    PAGE_MENTION_PICKER_MUTED_DAY, PAGE_MENTION_PICKER_ROW_HEIGHT,
    PAGE_MENTION_PICKER_SELECTED_DAY, PAGE_MENTION_PICKER_WIDTH,
};
use super::rows::{build_mention_menu_sections, PageMentionMenuInputs, PageMentionMenuRow};
use super::PageMentionClock;
use super::{
    PAGE_MENTION_MENU_ANCHOR_X_INSET, PAGE_MENTION_MENU_ANCHOR_Y_GAP,
    PAGE_MENTION_MENU_MAX_HEIGHT_RATIO, PAGE_MENTION_MENU_PAGE_ROW_HEIGHT,
    PAGE_MENTION_MENU_ROW_HEIGHT,
};
use crate::ui::{
    alpha, Theme, COMMAND_MENU_ICON_SIZE, COMMAND_MENU_INSET, COMMAND_MENU_LABEL_SIZE,
    COMMAND_MENU_RADIUS, COMMAND_MENU_ROW_GAP, COMMAND_MENU_ROW_HEIGHT, COMMAND_MENU_ROW_PADDING_X,
    COMMAND_MENU_ROW_RADIUS, COMMAND_MENU_SECTION_TITLE_SIZE, COMMAND_MENU_SUBLABEL_SIZE,
    COMMAND_MENU_WIDTH,
};

const MENTION_MENU_REFERENCE: &str =
    include_str!("../../../../../../reference/notion-7.31.3/mention-menu.json");
const DATE_PICKER_REFERENCE: &str =
    include_str!("../../../../../../reference/notion-7.31.3/date-picker.json");

fn reference(json: &str) -> Value {
    serde_json::from_str(json).expect("reference JSON parses")
}

fn number(value: &Value, path: &[&str]) -> f64 {
    let mut cursor = value;
    for key in path {
        cursor = cursor
            .get(key)
            .unwrap_or_else(|| panic!("reference key {} is missing", path.join(".")));
    }
    cursor
        .as_f64()
        .unwrap_or_else(|| panic!("reference key {} is not a number", path.join(".")))
}

fn text<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    let mut cursor = value;
    for key in path {
        cursor = cursor
            .get(key)
            .unwrap_or_else(|| panic!("reference key {} is missing", path.join(".")));
    }
    cursor
        .as_str()
        .unwrap_or_else(|| panic!("reference key {} is not a string", path.join(".")))
}

fn hex(color: &str) -> u32 {
    u32::from_str_radix(color.trim_start_matches('#'), 16).expect("reference colour is hex")
}

/// `rgba(r,g,b,a)` from the reference into notnotion's hex plus opacity.
fn rgba(color: &str) -> (u32, f32) {
    let inner = color
        .trim_start_matches("rgba(")
        .trim_end_matches(')')
        .split(',')
        .map(|part| part.trim().parse::<f32>().expect("rgba component"))
        .collect::<Vec<_>>();
    let [r, g, b, a] = inner.as_slice() else {
        panic!("reference colour {color} is not rgba");
    };
    (((*r as u32) << 16) | ((*g as u32) << 8) | (*b as u32), *a)
}

#[test]
fn mention_menu_geometry_matches_notion() {
    let json = reference(MENTION_MENU_REFERENCE);
    assert_eq!(
        f64::from(COMMAND_MENU_WIDTH),
        number(&json, &["menu", "width_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_RADIUS),
        number(&json, &["menu", "border_radius_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_INSET),
        number(&json, &["menu", "inset_px"])
    );
    assert_eq!(
        f64::from(PAGE_MENTION_MENU_ROW_HEIGHT),
        number(&json, &["menu", "row", "height_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_ROW_HEIGHT),
        number(&json, &["menu", "row", "height_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_ROW_RADIUS),
        number(&json, &["menu", "row", "border_radius_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_ROW_GAP),
        number(&json, &["menu", "row", "gap_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_ICON_SIZE),
        number(&json, &["menu", "row", "icon_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_LABEL_SIZE),
        number(&json, &["menu", "row", "label_font_size_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_SUBLABEL_SIZE),
        number(&json, &["menu", "row", "sublabel_font_size_px"])
    );
    assert_eq!(
        f64::from(COMMAND_MENU_SECTION_TITLE_SIZE),
        number(&json, &["menu", "section_header", "font_size_px"])
    );
    assert_eq!(
        f64::from(PAGE_MENTION_MENU_PAGE_ROW_HEIGHT),
        number(&json, &["menu", "two_line_page_row", "height_px"])
    );
    assert_eq!(text(&json, &["menu", "max_height"]), "40vh");
    assert!((PAGE_MENTION_MENU_MAX_HEIGHT_RATIO - 0.4).abs() < f32::EPSILON);
    assert_eq!(text(&json, &["menu", "row", "padding"]), "0 8px");
    assert_eq!(f64::from(COMMAND_MENU_ROW_PADDING_X), 8.0);
}

#[test]
fn mention_menu_anchor_matches_notion_samples() {
    let json = reference(MENTION_MENU_REFERENCE);
    let samples = json["menu"]["anchor"]["measured_samples"]
        .as_array()
        .expect("anchor samples");
    for sample in samples {
        let glyph_left = number(sample, &["at_glyph_left"]);
        let line_bottom = number(sample, &["line_box_bottom"]);
        let expected_left = number(sample, &["menu_left"]);
        let expected_top = number(sample, &["menu_top"]);
        let left = glyph_left - f64::from(PAGE_MENTION_MENU_ANCHOR_X_INSET);
        let top = line_bottom + f64::from(PAGE_MENTION_MENU_ANCHOR_Y_GAP);
        assert!(
            (left - expected_left).abs() <= 0.5,
            "menu left {left} vs Notion {expected_left}"
        );
        assert!(
            (top - expected_top).abs() <= 0.5,
            "menu top {top} vs Notion {expected_top}"
        );
    }
}

#[test]
fn mention_menu_colours_match_notion() {
    let json = reference(MENTION_MENU_REFERENCE);
    let theme = Theme::for_appearance_mode(AppearanceMode::Light);
    let (selected_hex, selected_alpha) = rgba(text(&json, &["menu", "row", "selected_background"]));
    assert_eq!(theme.command_menu_active_bg.hex, selected_hex);
    assert!((theme.command_menu_active_bg.opacity - selected_alpha).abs() < 0.001);
    assert_eq!(
        theme.menu_secondary_text,
        hex(text(&json, &["menu", "section_header", "color"]))
    );
    assert_eq!(
        theme.menu_dash,
        hex(text(&json, &["menu", "row", "sublabel_dash_color"]))
    );
    let (ring_hex, ring_alpha) = rgba(text(&json, &["menu", "divider", "color"]));
    assert_eq!(theme.menu_ring.hex, ring_hex);
    assert!((theme.menu_ring.opacity - ring_alpha).abs() < 0.001);
    assert_eq!(text(&json, &["chip", "color"]), "#7d7a75");
    assert_eq!(text(&json, &["text_block", "color"]), "#2c2c2b");
    assert_eq!(theme.text_primary, 0x2c2c2b);
}

#[test]
fn date_picker_geometry_matches_notion() {
    let json = reference(DATE_PICKER_REFERENCE);
    assert_eq!(
        f64::from(PAGE_MENTION_PICKER_WIDTH),
        number(&json, &["dialog", "width_px"])
    );
    assert_eq!(
        f64::from(PAGE_MENTION_PICKER_FIELD_HEIGHT),
        json["date_field"]["size_px"][1]
            .as_f64()
            .expect("field height")
    );
    assert_eq!(
        f64::from(PAGE_MENTION_PICKER_CELL),
        number(&json, &["day_grid", "cell_px"])
    );
    assert_eq!(
        f64::from(PAGE_MENTION_PICKER_DAY_BUTTON),
        number(&json, &["day_grid", "day_button_px"])
    );
    assert_eq!(
        f64::from(PAGE_MENTION_PICKER_ROW_HEIGHT),
        json["row"]["size_px"][1].as_f64().expect("row height")
    );
    assert_eq!(
        PAGE_MENTION_PICKER_SELECTED_DAY,
        hex(text(&json, &["day_grid", "selected_background"]))
    );
    assert_eq!(
        PAGE_MENTION_PICKER_MUTED_DAY,
        hex(text(&json, &["day_grid", "adjacent_month_color"]))
    );
}

#[test]
fn temporary_input_pill_matches_notion() {
    let json = reference(MENTION_MENU_REFERENCE);
    let (background, opacity) = rgba(text(&json, &["menu", "temporary_input_pill", "background"]));
    assert_eq!(
        page_mention_input_pill_background(AppearanceMode::Light),
        alpha(background, opacity)
    );
    let outline = text(&json, &["menu", "temporary_input_pill", "outline"]);
    let width = outline
        .split_once("px")
        .and_then(|(width, _)| width.parse::<f32>().ok())
        .expect("the outline starts with a pixel width");
    assert_eq!(PAGE_MENTION_PILL_PADDING_X, width);
    assert_eq!(
        f64::from(PAGE_MENTION_PILL_RADIUS),
        number(&json, &["menu", "temporary_input_pill", "border_radius_px"])
    );
    assert_eq!(
        page_mention_ghost_foreground(AppearanceMode::Light),
        gpui::rgb(hex(text(
            &json,
            &["menu", "temporary_input_pill", "ghost_text_color"],
        )))
        .into()
    );
}

/// The clock of the capture session: Wednesday 2 September 2026, at the
/// evening hour the prefix samples were typed.
fn capture_clock() -> PageMentionClock {
    PageMentionClock {
        today: NaiveDate::from_ymd_opt(2026, 9, 2).expect("the captured date is valid"),
        now: NaiveTime::from_hms_opt(20, 30, 0).expect("the captured time is valid"),
        time_zone: "America/Toronto".to_string(),
    }
}

#[test]
fn partial_queries_resolve_to_the_rows_notion_showed() {
    let json = reference(MENTION_MENU_REFERENCE);
    let clock = capture_clock();
    for sample in json["prefix_completion_samples"]["samples"]
        .as_array()
        .expect("prefix samples")
        .iter()
        .filter(|sample| sample.get("section").is_none())
    {
        let query = text(sample, &["query"]);
        let sections = build_mention_menu_sections(PageMentionMenuInputs {
            query,
            clock: &clock,
            users: &[],
            pages: &[],
        });
        let section = sections
            .first()
            .expect("a partial date query keeps a Date section");
        assert_eq!(section.title, "Date", "query {query} lost its Date section");
        let PageMentionMenuRow::Date {
            label,
            sublabel,
            mention,
        } = section
            .rows
            .first()
            .expect("the Date section leads with a date")
        else {
            panic!("query {query} did not lead with a date row");
        };
        assert_eq!(label, text(sample, &["row_label"]), "label for {query}");
        assert_eq!(
            sublabel.as_deref(),
            sample["row_sublabel"].as_str(),
            "sublabel for {query}"
        );
        assert_eq!(
            mention.start_date.to_string(),
            text(sample, &["start_date"]),
            "date for {query}"
        );
        assert_eq!(
            page_mention_ghost_completion(label, query).as_deref(),
            sample["ghost"].as_str(),
            "ghost for {query}"
        );
    }
}

#[test]
fn a_person_prefix_ghosts_the_rest_of_the_name() {
    let json = reference(MENTION_MENU_REFERENCE);
    let sample = json["prefix_completion_samples"]["samples"]
        .as_array()
        .expect("prefix samples")
        .iter()
        .find(|sample| sample.get("section").and_then(Value::as_str) == Some("People"))
        .expect("a People sample");
    // Notion ghosts the member's name; the row adds the "(You)" suffix after it.
    let name = text(sample, &["row_label"])
        .split(" (")
        .next()
        .expect("the row label starts with the name");
    assert_eq!(
        page_mention_ghost_completion(name, text(sample, &["query"])).as_deref(),
        sample["ghost"].as_str()
    );
}
