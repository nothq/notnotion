use gpui_components::text_input::floor_grapheme_boundary;

use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::{PageCommand, PAGE_COMMANDS};
use crate::ui::{CardPageBlockKind, PageCommandTarget};

pub(super) fn slash_menu_token(text: &str, cursor: usize) -> Option<(usize, String)> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }
    let prefix = &text[..cursor];
    let trigger_offset = prefix.rfind('/')?;
    let preceded_by_boundary = trigger_offset == 0
        || prefix[..trigger_offset]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
    let query = &prefix[trigger_offset + 1..];
    let followed_by_boundary = text[cursor..]
        .chars()
        .next()
        .is_none_or(char::is_whitespace);
    (preceded_by_boundary && followed_by_boundary && !query.chars().any(char::is_whitespace))
        .then(|| (trigger_offset, query.to_string()))
}

pub(in super::super) fn page_block_markdown_shortcut(
    text: &str,
    cursor: usize,
) -> Option<PageCommandTarget> {
    if cursor != text.len() {
        return None;
    }
    match text {
        "# " => Some(PageCommandTarget::Editable(CardPageBlockKind::SubHeader)),
        "## " => Some(PageCommandTarget::Editable(CardPageBlockKind::SubSubHeader)),
        "### " => Some(PageCommandTarget::Editable(CardPageBlockKind::Heading3)),
        "#### " => Some(PageCommandTarget::Editable(CardPageBlockKind::Heading4)),
        "- " | "* " | "+ " => Some(PageCommandTarget::Editable(CardPageBlockKind::BulletedList)),
        "1. " => Some(PageCommandTarget::Editable(CardPageBlockKind::NumberedList)),
        "[] " => Some(PageCommandTarget::Editable(CardPageBlockKind::ToDoList)),
        "> " => Some(PageCommandTarget::Editable(CardPageBlockKind::ToggleList)),
        "\" " => Some(PageCommandTarget::Editable(CardPageBlockKind::Quote)),
        "```" => Some(PageCommandTarget::Code),
        "---" => Some(PageCommandTarget::Divider),
        _ => None,
    }
}

pub(in super::super) fn previous_cursor_offset(old_text: &str, new_text: &str) -> usize {
    let prefix = old_text
        .char_indices()
        .zip(new_text.char_indices())
        .take_while(|((_, old), (_, new))| old == new)
        .map(|((offset, old), _)| offset + old.len_utf8())
        .last()
        .unwrap_or(0);
    let old_tail = &old_text[prefix..];
    let new_tail = &new_text[prefix..];
    let suffix_bytes = old_tail
        .chars()
        .rev()
        .zip(new_tail.chars().rev())
        .take_while(|(old, new)| old == new)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>()
        .min(old_tail.len());
    floor_grapheme_boundary(old_text, old_text.len().saturating_sub(suffix_bytes))
}

pub(super) fn filtered_page_slash_commands(query: &str) -> Vec<PageCommand> {
    let query = query.to_ascii_lowercase();
    PAGE_COMMANDS
        .into_iter()
        .filter(|command| match command.target {
            PageCommandTarget::Editable(kind) => VerifiedPageTextBlockKind::parse(kind).is_some(),
            PageCommandTarget::Code | PageCommandTarget::Divider => true,
        })
        .filter(|command| {
            query.is_empty()
                || command.label.to_ascii_lowercase().contains(&query)
                || command
                    .keywords
                    .iter()
                    .any(|keyword| keyword.to_ascii_lowercase().contains(&query))
        })
        .collect()
}

pub(super) fn clamped_slash_command_index(selected_index: usize, command_count: usize) -> usize {
    selected_index.min(command_count.saturating_sub(1))
}
