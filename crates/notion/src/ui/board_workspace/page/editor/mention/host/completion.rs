use gpui::Context;

use super::super::data::{PageMentionCompletion, PageMentionCompletionResolution};
use crate::ui::SurfaceState;

impl SurfaceState {
    pub(super) fn finish_page_mention_completion(
        &mut self,
        completion: PageMentionCompletion,
        cx: &mut Context<Self>,
    ) {
        match self.page_editor.mention.resolve_completion(completion) {
            PageMentionCompletionResolution::Stale => {}
            PageMentionCompletionResolution::Notify => cx.notify(),
            PageMentionCompletionResolution::Failure(failure) => {
                let failure = *failure;
                if self.handle_notion_workspace_failure(failure.operation, failure.error, cx) {
                    return;
                }
                if let Some(recovery) = failure.recovery {
                    self.page_editor
                        .mention
                        .recover_completion_failure(recovery);
                    cx.notify();
                }
            }
        }
    }
}
