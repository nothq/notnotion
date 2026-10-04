use super::super::super::super::Value;
use crate::model::PageShellIcon;

pub(in crate::live::board::load) fn page_shell_icon(
    block: &Value,
    fallback: &str,
) -> PageShellIcon {
    explicit_page_shell_icon(block, fallback).unwrap_or_else(|| named_page_shell_icon(fallback))
}

pub(in crate::live::board) fn explicit_page_shell_icon(
    block: &Value,
    fallback: &str,
) -> Option<PageShellIcon> {
    let icon = block
        .get("format")
        .and_then(|format| format.get("page_icon"))
        .and_then(Value::as_str)?;
    if let Some(asset_name) = icon.strip_prefix("/icons/") {
        let name = asset_name.strip_suffix(".svg").unwrap_or(asset_name);
        let name = if name.is_empty() { fallback } else { name };
        return Some(named_page_shell_icon(name));
    }
    if icon.starts_with("notion://custom_emoji/") {
        return PageShellIcon::custom(icon).ok();
    }
    if icon.starts_with("attachment:") || icon.contains("://") {
        return notion_external_page_shell_icon(block, icon)
            .or_else(|| Some(named_page_shell_icon(fallback)));
    }
    Some(PageShellIcon::emoji(icon))
}

pub(in crate::live::board::load::sidebar) fn named_page_shell_icon(name: &str) -> PageShellIcon {
    PageShellIcon::named(name)
}

fn notion_external_page_shell_icon(block: &Value, source: &str) -> Option<PageShellIcon> {
    let block_id = block.get("id").and_then(Value::as_str)?;
    let space_id = block.get("space_id").and_then(Value::as_str)?;
    let render_url = crate::live::notion_page_icon_render_url(source, block_id, space_id).ok()?;
    PageShellIcon::external_with_render_url(source, render_url).ok()
}
