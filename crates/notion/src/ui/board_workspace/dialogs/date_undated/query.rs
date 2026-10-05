use crate::model::{BoardItem, SetCalendarPageDateRequest};
use crate::ui::CivilDate;

use super::calendar_query_date;

pub(super) fn date_undated_click_assignment_request(
    item: &BoardItem,
    target_date: CivilDate,
) -> SetCalendarPageDateRequest {
    SetCalendarPageDateRequest::from_no_date_click(
        item.block_id.clone(),
        calendar_query_date(target_date),
    )
    .expect("Notion no-date click must retain a valid block id and date")
}
