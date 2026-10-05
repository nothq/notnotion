use crate::model::{CardPage, ResizePageColumnsRequest};

use super::PageWriteReplayError;

pub(super) fn validate_resize_source(
    page: &CardPage,
    request: &ResizePageColumnsRequest,
) -> Result<(), PageWriteReplayError> {
    request
        .validate_source_on(page)
        .map_err(|detail| PageWriteReplayError::conflict("resize columns", detail))
}

pub(super) fn resize_columns(
    page: &mut CardPage,
    request: &ResizePageColumnsRequest,
) -> Result<(), PageWriteReplayError> {
    request
        .apply_to_page(page)
        .map_err(|detail| PageWriteReplayError::conflict("resize columns", detail))
}
