/// The longest query Notion keeps a mention menu open for.
const MENTION_QUERY_LIMIT: usize = 64;

/// Locate the `@` trigger the caret is typing after.
///
/// Returns the byte offset of the `@` and the query between it and the caret.
/// The `@` must start the text or follow whitespace, and the query may contain
/// spaces (`@sep 7`) but no line break, no leading space, and at most
/// [`MENTION_QUERY_LIMIT`] bytes.
pub(in crate::ui::board_workspace::page) fn mention_menu_token(
    text: &str,
    cursor: usize,
) -> Option<(usize, String)> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }
    let prefix = &text[..cursor];
    let trigger_offset = prefix.rfind('@')?;
    let preceded_by_boundary = trigger_offset == 0
        || prefix[..trigger_offset]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
    if !preceded_by_boundary {
        return None;
    }
    let query = &prefix[trigger_offset + 1..];
    if query.len() > MENTION_QUERY_LIMIT
        || query.contains('\n')
        || query.chars().next().is_some_and(char::is_whitespace)
    {
        return None;
    }
    Some((trigger_offset, query.to_string()))
}

#[cfg(test)]
mod tests {
    use super::mention_menu_token;

    #[test]
    fn trigger_at_start_or_after_whitespace() {
        assert_eq!(mention_menu_token("@", 1), Some((0, String::new())));
        assert_eq!(
            mention_menu_token("@today", 6),
            Some((0, "today".to_string()))
        );
        assert_eq!(
            mention_menu_token("ship @sep 7", 11),
            Some((5, "sep 7".to_string()))
        );
        assert_eq!(mention_menu_token("mail@x", 6), None);
    }

    #[test]
    fn caret_position_bounds_the_query() {
        assert_eq!(mention_menu_token("@today", 3), Some((0, "to".to_string())));
        assert_eq!(mention_menu_token("@today", 0), None);
        assert_eq!(mention_menu_token("@ x", 3), None);
        assert_eq!(mention_menu_token("@a\nb", 4), None);
    }
}
