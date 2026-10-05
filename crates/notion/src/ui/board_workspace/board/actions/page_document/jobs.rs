use super::action::{
    PageCreationCompletion, PageDocumentAction, PageDocumentInternalAction, PageDocumentJob,
    PageOpenCompletion,
};

pub(super) fn run_page_document_job(job: PageDocumentJob) -> PageDocumentAction {
    match job {
        PageDocumentJob::Open(job) => {
            let result = job.workspace_api.load_card_page(&job.block_id);
            PageDocumentAction::Internal(PageDocumentInternalAction::Opened(Box::new(
                PageOpenCompletion {
                    block_id: job.block_id,
                    authority: job.authority,
                    result,
                },
            )))
        }
        PageDocumentJob::Create(job) => {
            let result = job.workspace_api.create_page_in_column(&job.column_title);
            PageDocumentAction::Internal(PageDocumentInternalAction::FinishCreation(Box::new(
                PageCreationCompletion {
                    column_index: job.column_index,
                    column_title: job.column_title,
                    result,
                },
            )))
        }
    }
}
