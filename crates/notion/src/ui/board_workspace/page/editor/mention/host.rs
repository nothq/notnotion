use gpui::Context;

use super::actions::{PageMentionAction, PageMentionDispatchResult};
use super::PageMentionClock;
use crate::ui::board_workspace::PageFocusSession;
use crate::ui::SurfaceState;

mod completion;
mod effects;
mod loads;

use effects::{PageMentionEffectHost, PageMentionHostOutcome};

impl SurfaceState {
    pub(in crate::ui::board_workspace::page) fn dispatch_page_mention_action(
        &mut self,
        action: PageMentionAction,
        cx: &mut Context<Self>,
    ) -> PageMentionDispatchResult {
        let clock = PageMentionClock::from(&self.board);
        let outcome = self.page_editor.mention.reduce(action, &clock, cx);
        let result = PageMentionDispatchResult {
            changed: outcome.changed,
            consumed: outcome.consumed,
        };
        for effect in outcome.effects {
            let workspace_api = self.notion_startup.workspace_api();
            let current_board_url = self.notion_startup.board_url();
            let host_outcome = PageMentionEffectHost {
                editor: &mut self.page_editor,
                documents: &self.page_documents,
                workspace_api,
                current_board_url,
                theme: self.theme,
            }
            .execute(effect, cx);
            if let PageMentionHostOutcome::Edit(edit) = host_outcome {
                let edit = *edit;
                self.apply_page_edit_transition(edit.transition, cx);
                let post_edit = edit.finish.apply(&mut self.page_editor, cx);
                if let Some((block_id, cursor)) = post_edit.focus {
                    PageFocusSession::new(&mut self.page_editor, &mut self.page_documents)
                        .focus_block(block_id, cursor);
                }
                if post_edit.notify {
                    cx.notify();
                }
            }
        }
        if outcome.notify {
            cx.notify();
        }
        result
    }
}
