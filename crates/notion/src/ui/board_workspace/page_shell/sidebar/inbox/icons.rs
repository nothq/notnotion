use crate::ui::{render_svg_image, svg_from_body};

use super::super::{img, px, AnyElement, IntoElement, PageShellIcon, Styled};
use gpui::App;

#[derive(Clone, Copy)]
pub(super) enum InboxActionIcon {
    Archive,
    ArchiveRead,
    Check,
    Clock,
    Filter,
    Inbox,
    Read,
    Unarchive,
    Unread,
}

pub(super) fn inbox_action_icon(
    icon: InboxActionIcon,
    color: u32,
    size: f32,
    cx: &mut App,
) -> AnyElement {
    let body = match icon {
        InboxActionIcon::Archive => format!(
            r##"<path d="M2.25 4.25h11.5v8.25a1.25 1.25 0 0 1-1.25 1.25h-9a1.25 1.25 0 0 1-1.25-1.25zM1.75 2.25h12.5v2H1.75zm4 4.5h4.5" fill="none" stroke="#{color:06x}" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::ArchiveRead => format!(
            r##"<path d="M2.25 4.25h11.5v8.25a1.25 1.25 0 0 1-1.25 1.25h-9a1.25 1.25 0 0 1-1.25-1.25zM1.75 2.25h12.5v2H1.75zM5 9l1.7 1.7L11 6.5" fill="none" stroke="#{color:06x}" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Check => format!(
            r##"<path d="m3.2 8.1 3 3.1 6.7-7" fill="none" stroke="#{color:06x}" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Clock => format!(
            r##"<circle cx="8" cy="8" r="5.75" fill="none" stroke="#{color:06x}" stroke-width="1.25"/><path d="M8 4.6V8l-2.2 1.5" fill="none" stroke="#{color:06x}" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Filter => format!(
            r##"<path d="M2.5 4h11M4.5 8h7M6.5 12h3" fill="none" stroke="#{color:06x}" stroke-width="1.35" stroke-linecap="round"/>"##
        ),
        InboxActionIcon::Inbox => format!(
            r##"<path d="M3.1 4.2h9.8l1 3v4.5c0 .7-.6 1.2-1.2 1.2H3.3c-.7 0-1.2-.6-1.2-1.2V7.2zM2.2 7.2h3.3c.2 1.2 1.1 1.8 2.5 1.8s2.3-.6 2.5-1.8h3.3" fill="none" stroke="#{color:06x}" stroke-width="1.2" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Read => format!(
            r##"<rect x="2.5" y="2.5" width="11" height="11" rx="2" fill="none" stroke="#{color:06x}" stroke-width="1.2"/><path d="m5 8 1.8 1.8L11 5.7" fill="none" stroke="#{color:06x}" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Unarchive => format!(
            r##"<path d="M5.2 5.1H2.5V2.4M2.8 5a5.5 5.5 0 1 1-.3 5.2" fill="none" stroke="#{color:06x}" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
        InboxActionIcon::Unread => format!(
            r##"<rect x="2.5" y="2.5" width="11" height="11" rx="2" fill="none" stroke="#{color:06x}" stroke-width="1.2"/><circle cx="11.1" cy="4.9" r="1.2" fill="#{color:06x}"/>"##
        ),
    };
    img(render_svg_image(svg_from_body("0 0 16 16", body), cx))
        .size(px(size))
        .into_any_element()
}

pub(super) fn inbox_target_icon() -> PageShellIcon {
    PageShellIcon::named("page")
}

pub(super) fn inbox_icon() -> PageShellIcon {
    PageShellIcon::named("inbox")
}
