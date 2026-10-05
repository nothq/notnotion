mod breadcrumb;
mod query;
mod result;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod transport;

pub(super) use breadcrumb::{resolve_search_breadcrumb, search_result_breadcrumb};
pub(super) use result::{
    record_entry_by_id, search_result_attribution_display_name,
    search_result_has_required_collection, shape_page_shell_search_result, SearchResultPage,
};
pub(crate) use transport::search_workspace;
pub(super) use transport::{
    combined_search_attribution_records, combined_search_collection_records,
    missing_attribution_pointers, AttributionRecords,
};
