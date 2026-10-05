use crate::model::{CardPage, CardPageBlockKind, PageShellIcon, SetPageIconRequest};

pub(super) enum PreparedPageLinkIconEdit {
    Unchanged,
    Change {
        page: Box<CardPage>,
        block_index: usize,
        request: SetPageIconRequest,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageLinkIconPersistence {
    Queue,
    AlreadyCommitted,
}

pub(super) fn prepare_page_link_icon_edit(
    page: CardPage,
    block_id: &str,
    icon: Option<PageShellIcon>,
) -> Option<PreparedPageLinkIconEdit> {
    let request = match SetPageIconRequest::new(block_id.to_owned(), icon) {
        Ok(request) => request,
        Err(error) => {
            println!("notnotion: ignored invalid page icon: {error}");
            return None;
        }
    };
    let block_index = page
        .blocks
        .iter()
        .position(|block| block.block_id == block_id)?;
    let block = &page.blocks[block_index];
    if block.editable_content().is_none_or(|editable| {
        !matches!(
            editable.kind,
            CardPageBlockKind::PageLink | CardPageBlockKind::Callout
        )
    }) {
        return None;
    }
    if PageShellIcon::persisted_value_eq(block.icon.as_ref(), request.icon()) {
        return Some(PreparedPageLinkIconEdit::Unchanged);
    }
    Some(PreparedPageLinkIconEdit::Change {
        page: Box::new(page),
        block_index,
        request,
    })
}
