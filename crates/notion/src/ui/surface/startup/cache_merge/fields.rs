use crate::model::{CardPage, CardPageBlock, CardPageEditableBlock};

pub(super) fn merge_cached_page_root_fields(
    target: &mut CardPage,
    baseline: &CardPage,
    edited: &CardPage,
) {
    merge_cached_field(&mut target.title, &baseline.title, &edited.title);
    merge_cached_field(&mut target.status, &baseline.status, &edited.status);
}

pub(super) fn merge_cached_page_block(
    baseline: &CardPageBlock,
    edited: &CardPageBlock,
    live: &CardPageBlock,
    preserve_edited_structure: bool,
) -> CardPageBlock {
    let mut target = live.clone();
    if preserve_edited_structure {
        target.parent_block_id.clone_from(&edited.parent_block_id);
        target.depth = edited.depth;
    }
    merge_cached_field(&mut target.color, &baseline.color, &edited.color);
    merge_cached_field(&mut target.icon, &baseline.icon, &edited.icon);

    let (Some(baseline_editable), Some(edited_editable)) =
        (baseline.editable_content(), edited.editable_content())
    else {
        if baseline.content != edited.content {
            target.content.clone_from(&edited.content);
        }
        return target;
    };
    let Some(target_editable) = target.editable_content_mut() else {
        if baseline_editable != edited_editable {
            target.content.clone_from(&edited.content);
        }
        return target;
    };
    merge_cached_editable_fields(target_editable, baseline_editable, edited_editable);
    target
}

fn merge_cached_editable_fields(
    target: &mut CardPageEditableBlock,
    baseline: &CardPageEditableBlock,
    edited: &CardPageEditableBlock,
) {
    if baseline.kind != edited.kind {
        target.set_kind(edited.kind);
    }
    merge_cached_field(&mut target.text, &baseline.text, &edited.text);
    merge_cached_field(
        &mut target.annotations,
        &baseline.annotations,
        &edited.annotations,
    );
    if baseline.to_do_state() != edited.to_do_state() {
        if let Some(state) = edited.to_do_state() {
            if target.to_do_state().is_none() {
                target.set_kind(edited.kind);
            }
            target.set_to_do_state(state);
        }
    }
    if baseline.quote_size() != edited.quote_size() {
        if let Some(size) = edited.quote_size() {
            if target.quote_size().is_none() {
                target.set_kind(edited.kind);
            }
            target.set_quote_size(size);
        }
    }
    if baseline.code_language() != edited.code_language() {
        if let Some(language) = edited.code_language() {
            if target.code_language().is_none() {
                target.set_kind(edited.kind);
            }
            target.set_code_language(language.clone());
        }
    }
    if baseline.code_wrap() != edited.code_wrap() {
        if let Some(wrap) = edited.code_wrap() {
            if target.code_wrap().is_none() {
                target.set_kind(edited.kind);
            }
            target.set_code_wrap(wrap);
        }
    }
}

fn merge_cached_field<T: Clone + PartialEq>(target: &mut T, baseline: &T, edited: &T) {
    if baseline != edited {
        target.clone_from(edited);
    }
}
