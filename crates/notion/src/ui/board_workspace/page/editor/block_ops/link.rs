use gpui::ClipboardItem;

use super::super::editing::{PageEditEffect, PageEditSession, PageEditorEffect};

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn copy_page_block_link(
        &mut self,
        block_id: &str,
        page_url: Option<String>,
    ) {
        let Some(page_url) = page_url else {
            return;
        };
        let page_url = page_url.split('#').next().unwrap_or(page_url.as_str());
        let fragment = block_id.replace('-', "");
        self.effects
            .push(PageEditEffect::WriteClipboard(ClipboardItem::new_string(
                format!("{page_url}#{fragment}"),
            )));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenu,
        ));
        self.effects.push(PageEditEffect::Notify);
    }
}
