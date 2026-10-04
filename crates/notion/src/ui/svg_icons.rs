use std::sync::Arc;

use gpui::{Image, ImageFormat};

pub(crate) fn svg_from_body(view_box: &str, body: impl Into<String>) -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}">{}</svg>"#,
            body.into()
        )
        .into_bytes(),
    ))
}

pub(crate) fn svg_with_paths(view_box: &str, paths: &[&str], fill: u32) -> Arc<Image> {
    let body = paths
        .iter()
        .map(|path| format!("<path fill=\"#{fill:06x}\" d=\"{path}\"/>"))
        .collect::<String>();
    svg_from_body(view_box, body)
}

pub(crate) fn svg_with_group(
    view_box: &str,
    transform: &str,
    paths: &[&str],
    fill: u32,
) -> Arc<Image> {
    let body = paths
        .iter()
        .map(|path| format!("<path fill=\"#{fill:06x}\" d=\"{path}\"/>"))
        .collect::<String>();
    svg_from_body(
        view_box,
        format!(r#"<g transform="{transform}">{body}</g>"#),
    )
}

pub(crate) fn zed_folder_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 16 16",
        format!(
            r##"<path d="M8.26046 3.97337C8.3527 4.17617 8.4795 4.47151 8.57375 4.69341C8.65258 4.87898 8.83437 4.99999 9.03599 4.99999H12.5C12.7761 4.99999 13 5.22385 13 5.49999V12.125C13 12.4011 12.7761 12.625 12.5 12.625H3.5C3.22386 12.625 3 12.4011 3 12.125V3.86932C3 3.59318 3.22386 3.36932 3.5 3.36932H7.34219C7.74141 3.36932 8.09483 3.60924 8.26046 3.97337Z" fill="none" stroke="#{stroke:06x}" stroke-width="1.2" stroke-linecap="round"/>"##
        ),
    )
}

pub(crate) fn square_plus_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M8 2.74a.66.66 0 0 1 .66.66v3.94h3.94a.66.66 0 0 1 0 1.32H8.66v3.94a.66.66 0 0 1-1.32 0V8.66H3.4a.66.66 0 0 1 0-1.32h3.94V3.4A.66.66 0 0 1 8 2.74",
        ],
        fill,
    )
}

pub(crate) fn lightning_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M9.332 1.38a.575.575 0 0 0-.662.204l-4.936 6.84a.575.575 0 0 0 .466.911h2.398l-.315 4.32a.575.575 0 0 0 1.039.378l4.944-6.832a.575.575 0 0 0-.466-.912H9.401l.309-4.328a.575.575 0 0 0-.378-.582M7.216 8.185H5.324l3.095-4.289-.209 2.927a.575.575 0 0 0 .574.616h1.89l-3.097 4.28.212-2.917a.575.575 0 0 0-.573-.617",
        ],
        fill,
    )
}

pub(crate) fn status_property_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M8 1.75c.21 0 .4.133.47.331l.56 1.63c.064.188.212.336.4.4l1.63.56a.5.5 0 0 1 0 .946l-1.63.56a.63.63 0 0 0-.4.4l-.56 1.63a.5.5 0 0 1-.946 0l-.56-1.63a.63.63 0 0 0-.4-.4l-1.63-.56a.5.5 0 0 1 0-.946l1.63-.56a.63.63 0 0 0 .4-.4l.56-1.63A.5.5 0 0 1 8 1.75m4.25 7.5c.15 0 .286.096.335.238l.287.835c.045.13.147.232.278.277l.835.287a.375.375 0 0 1 0 .71l-.835.287a.44.44 0 0 0-.278.278l-.287.835a.375.375 0 0 1-.71 0l-.287-.835a.44.44 0 0 0-.278-.278l-.835-.287a.375.375 0 0 1 0-.71l.835-.287a.44.44 0 0 0 .278-.277l.287-.835a.375.375 0 0 1 .335-.238",
        ],
        fill,
    )
}

pub(crate) fn number_property_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 16 16",
        format!(
            r##"<path d="M6.15 2.5 4.8 13.5M10.85 2.5 9.5 13.5M3 6.25h10M2.55 9.75h10" fill="none" stroke="#{stroke:06x}" stroke-width="1.25" stroke-linecap="round"/>"##
        ),
    )
}

pub(crate) fn checkbox_property_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 16 16",
        format!(
            r##"<rect x="2.5" y="2.5" width="11" height="11" rx="2" fill="none" stroke="#{stroke:06x}" stroke-width="1.25"/><path d="m5.2 8.05 1.85 1.85 3.75-3.8" fill="none" stroke="#{stroke:06x}" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    )
}

pub(crate) fn time_property_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 16 16",
        format!(
            r##"<circle cx="8" cy="8" r="5.4" fill="none" stroke="#{stroke:06x}" stroke-width="1.2"/><path d="M8 4.7v3.55l2.35 1.35" fill="none" stroke="#{stroke:06x}" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    )
}

pub(crate) fn person_property_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M8 2.2a2.3 2.3 0 1 1 0 4.6 2.3 2.3 0 0 1 0-4.6m-4.75 9.05c0-1.94 1.89-3.45 4.75-3.45s4.75 1.51 4.75 3.45a.55.55 0 0 1-.55.55H3.8a.55.55 0 0 1-.55-.55",
        ],
        fill,
    )
}

pub(crate) fn relation_property_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M4.25 11.75a.75.75 0 0 1 0-1.5h5.69L4.72 5.03a.75.75 0 1 1 1.06-1.06L11 9.19V3.5a.75.75 0 0 1 1.5 0v7.5a.75.75 0 0 1-.75.75z",
        ],
        fill,
    )
}

pub(crate) fn calendar_property_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M4.75 2a.625.625 0 0 1 .625.625V3.5h5.25v-.875a.625.625 0 1 1 1.25 0V3.5h.375A1.75 1.75 0 0 1 14 5.25v6.5a1.75 1.75 0 0 1-1.75 1.75h-8.5A1.75 1.75 0 0 1 2 11.75v-6.5A1.75 1.75 0 0 1 3.75 3.5h.375v-.875A.625.625 0 0 1 4.75 2m-1.625 4.25v5.5c0 .276.224.5.5.5h8.75a.5.5 0 0 0 .5-.5v-5.5z",
        ],
        fill,
    )
}

pub(crate) fn list_property_icon(fill: u32) -> Arc<Image> {
    svg_with_paths(
        "0 0 16 16",
        &[
            "M3 4.125a.875.875 0 1 1 1.75 0A.875.875 0 0 1 3 4.125m3 0a.625.625 0 0 1 .625-.625h6a.625.625 0 1 1 0 1.25h-6A.625.625 0 0 1 6 4.125M3 8a.875.875 0 1 1 1.75 0A.875.875 0 0 1 3 8m3 0a.625.625 0 0 1 .625-.625h6a.625.625 0 1 1 0 1.25h-6A.625.625 0 0 1 6 8m-3 3.875a.875.875 0 1 1 1.75 0 .875.875 0 0 1-1.75 0m3 0a.625.625 0 0 1 .625-.625h6a.625.625 0 1 1 0 1.25h-6A.625.625 0 0 1 6 11.875",
        ],
        fill,
    )
}

/// The clock glyph Notion draws before the mention menu's date row.
pub(crate) fn mention_clock_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 20 20",
        format!(
            r##"<circle cx="10" cy="10.5" r="7" fill="none" stroke="#{stroke:06x}" stroke-width="1.4"/><path d="M10 6.5v4l2.6 1.6" fill="none" stroke="#{stroke:06x}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    )
}

/// The alarm-clock glyph before the mention menu's "Remind me" row.
pub(crate) fn mention_alarm_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 20 20",
        format!(
            r##"<circle cx="10" cy="11" r="6.5" fill="none" stroke="#{stroke:06x}" stroke-width="1.4"/><path d="M10 7.5v3.5l2.2 1.4M4.5 5.2l2-1.8M15.5 5.2l-2-1.8" fill="none" stroke="#{stroke:06x}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    )
}

/// The person-with-plus glyph before the mention menu's "Invite…" row.
pub(crate) fn mention_invite_icon(stroke: u32) -> Arc<Image> {
    svg_from_body(
        "0 0 20 20",
        format!(
            r##"<circle cx="8.5" cy="6.5" r="3" fill="none" stroke="#{stroke:06x}" stroke-width="1.4"/><path d="M3 16.5c0-3 2.5-5 5.5-5s5.5 2 5.5 5M15.5 9.5v5M13 12h5" fill="none" stroke="#{stroke:06x}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    )
}
