use super::super::editing::{PageEditEffect, PageEditSession};
use super::super::persistence::{PageMutationAction, PageMutationPlan};
use super::persistence::{PageLinkIconPersistence, PreparedPageLinkIconEdit};
use crate::model::PageMutation;
use crate::ui::PageEditFocus;

pub(super) struct PageLinkIconEditApplication {
    pub(super) applied: bool,
    pub(super) mutation: Option<PageMutationAction>,
}

impl PageEditSession<'_> {
    pub(super) fn apply_page_link_icon_edit(
        &mut self,
        edit: PreparedPageLinkIconEdit,
        persistence: PageLinkIconPersistence,
        workspace_available: bool,
        focus: Option<PageEditFocus>,
    ) -> PageLinkIconEditApplication {
        let PreparedPageLinkIconEdit::Change {
            mut page,
            block_index,
            request,
        } = edit
        else {
            return PageLinkIconEditApplication {
                applied: true,
                mutation: None,
            };
        };
        if persistence == PageLinkIconPersistence::Queue && !workspace_available {
            return PageLinkIconEditApplication {
                applied: false,
                mutation: None,
            };
        }
        self.editor.record_page_structural_edit(&page, focus);
        page.blocks[block_index].icon = request.icon().cloned();
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::ReplaceLoadedPage(*page));
        let mutation = (persistence == PageLinkIconPersistence::Queue).then(|| {
            PageMutationAction::ApplyPlan(PageMutationPlan::mutation(
                page_id,
                PageMutation::SetPageIcon(request),
            ))
        });
        PageLinkIconEditApplication {
            applied: true,
            mutation,
        }
    }
}
