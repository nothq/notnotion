use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::live::board::page_mutation) struct RecordPointer {
    pub(in crate::live::board::page_mutation) table: &'static str,
    pub(in crate::live::board::page_mutation) id: String,
    pub(in crate::live::board::page_mutation) space_id: String,
}

impl RecordPointer {
    pub(in crate::live::board::page_mutation) fn block(id: &str, space_id: &str) -> Self {
        Self {
            table: "block",
            id: id.to_string(),
            space_id: space_id.to_string(),
        }
    }
}
