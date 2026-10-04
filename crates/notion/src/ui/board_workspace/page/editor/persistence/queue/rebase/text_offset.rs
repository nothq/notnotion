use super::PageWriteReplayError;

#[derive(Clone, Copy)]
pub(super) enum TextOffsetBias {
    Before,
    After,
}

pub(super) fn rebase_text_offset(
    block_id: &str,
    local: &str,
    authority: &str,
    local_offset: usize,
    bias: TextOffsetBias,
) -> Result<usize, PageWriteReplayError> {
    validate_utf8_offset(block_id, local, local_offset)?;
    if local == authority {
        return Ok(local_offset);
    }
    let prefix = shared_text_prefix_bytes(local, authority);
    let suffix = shared_text_suffix_bytes(&local[prefix..], &authority[prefix..]);
    let local_suffix_start = local.len() - suffix;
    let authority_suffix_start = authority.len() - suffix;
    if local_offset < prefix {
        return Ok(local_offset);
    }
    if local_offset > local_suffix_start {
        return Ok(authority.len() - (local.len() - local_offset));
    }
    if local_offset == prefix {
        return Ok(match bias {
            TextOffsetBias::Before => prefix,
            TextOffsetBias::After => authority_suffix_start,
        });
    }
    if local_offset == local_suffix_start {
        return Ok(authority_suffix_start);
    }
    Err(PageWriteReplayError::conflict(
        "rich text",
        format!("offset {local_offset} in block {block_id} overlaps a concurrent text replacement"),
    ))
}

pub(super) fn utf8_offset_from_utf16(
    block_id: &str,
    text: &str,
    utf16_offset: usize,
) -> Result<usize, PageWriteReplayError> {
    let mut utf16_count = 0usize;
    for (utf8_offset, character) in text.char_indices() {
        if utf16_count == utf16_offset {
            return Ok(utf8_offset);
        }
        utf16_count += character.len_utf16();
        if utf16_count > utf16_offset {
            break;
        }
    }
    if utf16_count == utf16_offset {
        return Ok(text.len());
    }
    Err(PageWriteReplayError::InvalidTextOffset {
        block_id: block_id.to_string(),
        offset: utf16_offset,
        text_len: text.encode_utf16().count(),
        encoding: "UTF-16",
    })
}

fn validate_utf8_offset(
    block_id: &str,
    text: &str,
    offset: usize,
) -> Result<(), PageWriteReplayError> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(PageWriteReplayError::InvalidTextOffset {
            block_id: block_id.to_string(),
            offset,
            text_len: text.len(),
            encoding: "UTF-8",
        });
    }
    Ok(())
}

fn shared_text_prefix_bytes(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

fn shared_text_suffix_bytes(left: &str, right: &str) -> usize {
    left.chars()
        .rev()
        .zip(right.chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}
