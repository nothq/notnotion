use serde::Serialize;

use super::{EmptyBlockFormatArgs, NewBlockCrdtData};

#[derive(Clone, Debug, Serialize)]
pub(in crate::live::board::page_mutation) struct NewCodeBlockArgs {
    pub(in crate::live::board::page_mutation) id: String,
    #[serde(rename = "type")]
    pub(in crate::live::board::page_mutation) block_type: &'static str,
    pub(in crate::live::board::page_mutation) format: EmptyBlockFormatArgs,
    pub(in crate::live::board::page_mutation) space_id: String,
    pub(in crate::live::board::page_mutation) created_time: u64,
    pub(in crate::live::board::page_mutation) created_by_table: &'static str,
    pub(in crate::live::board::page_mutation) created_by_id: String,
    pub(in crate::live::board::page_mutation) last_edited_time: u64,
    pub(in crate::live::board::page_mutation) crdt_data: NewBlockCrdtData,
    pub(in crate::live::board::page_mutation) crdt_format_version: u8,
}
