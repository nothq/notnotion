use crate::model::PageMutationRequest;
use crate::ui::board_workspace::PageWriteOperation;

use super::action::{
    PageMutationAction, PageMutationBackgroundJob, PageMutationInternalAction,
    PageMutationRecoveryLoadCompletion, PageMutationWriteCompletion,
};

pub(super) fn run_page_mutation_job(job: PageMutationBackgroundJob) -> PageMutationAction {
    match job {
        PageMutationBackgroundJob::Write(dispatch) => run_page_write(*dispatch),
        PageMutationBackgroundJob::LoadRecovery(dispatch) => {
            let result = dispatch.workspace_api.load_card_page(&dispatch.page_id);
            PageMutationAction::Internal(PageMutationInternalAction::RecoveryLoaded(Box::new(
                PageMutationRecoveryLoadCompletion {
                    page_id: dispatch.page_id,
                    token: dispatch.token,
                    result,
                },
            )))
        }
    }
}

fn run_page_write(dispatch: crate::ui::surface::PageMutationDispatch) -> PageMutationAction {
    let page_id = dispatch.page_id;
    let token = dispatch.token;
    let result = match dispatch.write.operation {
        PageWriteOperation::Mutation(mutation) => {
            dispatch
                .workspace_api
                .apply_page_mutation(PageMutationRequest {
                    page_block_id: page_id.clone(),
                    mutation,
                })
        }
        PageWriteOperation::RichText(request) => {
            dispatch.workspace_api.edit_page_block_text(request)
        }
    };
    PageMutationAction::Internal(PageMutationInternalAction::WriteCompleted(Box::new(
        PageMutationWriteCompletion {
            page_id,
            token,
            result,
        },
    )))
}
