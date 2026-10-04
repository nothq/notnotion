use chrono::NaiveDate;

use crate::model::BoardItem;

#[derive(Clone, Debug)]
pub struct LoadCalendarItemsRequest {
    query: CalendarItemsQuery,
}

impl LoadCalendarItemsRequest {
    pub fn month(grid_start_date: String, grid_end_date: String) -> Result<Self, String> {
        let grid_start = parse_calendar_date(&grid_start_date)?;
        let grid_end = parse_calendar_date(&grid_end_date)?;
        if grid_end.signed_duration_since(grid_start).num_days() != 41 {
            return Err(
                "Notion Calendar month queries require exactly 42 inclusive days".to_string(),
            );
        }
        Ok(Self {
            query: CalendarItemsQuery::Month {
                grid_start_date,
                grid_end_date,
            },
        })
    }

    pub fn undated(limit: usize, search_query: String) -> Result<Self, String> {
        if !(20..=200).contains(&limit) || !limit.is_multiple_of(20) {
            return Err(
                "Notion Calendar undated limits must be 20 through 200 in increments of 20"
                    .to_string(),
            );
        }
        Ok(Self {
            query: CalendarItemsQuery::Undated {
                limit,
                search_query,
            },
        })
    }

    pub(crate) fn into_query(self) -> CalendarItemsQuery {
        self.query
    }
}

#[derive(Clone, Debug)]
pub(crate) enum CalendarItemsQuery {
    Month {
        grid_start_date: String,
        grid_end_date: String,
    },
    Undated {
        limit: usize,
        search_query: String,
    },
}

#[derive(Clone, Debug)]
pub struct LoadCalendarItemsResult {
    pub items: Vec<BoardItem>,
    pub returned_block_count: usize,
}

#[derive(Clone, Debug)]
pub struct CreateCalendarPageRequest {
    target_date: CalendarDate,
}

impl CreateCalendarPageRequest {
    pub fn for_day(target_date: String) -> Result<Self, String> {
        Ok(Self {
            target_date: CalendarDate::parse(target_date)?,
        })
    }

    pub fn target_date(&self) -> NaiveDate {
        self.target_date.get()
    }
}

#[derive(Clone, Debug)]
pub struct SetCalendarPageDateRequest {
    block_id: CalendarPageId,
    target_date: CalendarDate,
    source: CalendarDateAssignmentSource,
}

impl SetCalendarPageDateRequest {
    pub fn from_no_date_click(block_id: String, target_date: String) -> Result<Self, String> {
        Self::new(
            block_id,
            target_date,
            CalendarDateAssignmentSource::NoDateClick,
        )
    }

    pub fn from_calendar_day_drop(block_id: String, target_date: String) -> Result<Self, String> {
        Self::new(
            block_id,
            target_date,
            CalendarDateAssignmentSource::CalendarDayDrop,
        )
    }

    fn new(
        block_id: String,
        target_date: String,
        source: CalendarDateAssignmentSource,
    ) -> Result<Self, String> {
        Ok(Self {
            block_id: CalendarPageId::parse(block_id)?,
            target_date: CalendarDate::parse(target_date)?,
            source,
        })
    }

    pub fn block_id(&self) -> &str {
        self.block_id.as_str()
    }

    pub fn target_date(&self) -> NaiveDate {
        self.target_date.get()
    }

    pub(crate) fn source(&self) -> CalendarDateAssignmentSource {
        self.source
    }
}

#[derive(Clone, Debug)]
pub struct SetCalendarPageDateRangeRequest {
    block_id: CalendarPageId,
    inclusive_range: InclusiveCalendarDateRange,
}

impl SetCalendarPageDateRangeRequest {
    pub fn from_calendar_endpoint_resize(
        block_id: String,
        inclusive_start_date: String,
        inclusive_end_date: String,
    ) -> Result<Self, String> {
        Ok(Self {
            block_id: CalendarPageId::parse(block_id)?,
            inclusive_range: InclusiveCalendarDateRange::parse(
                inclusive_start_date,
                inclusive_end_date,
            )?,
        })
    }

    pub fn block_id(&self) -> &str {
        self.block_id.as_str()
    }

    pub fn inclusive_start_date(&self) -> NaiveDate {
        self.inclusive_range.start_date()
    }

    pub fn inclusive_end_date(&self) -> NaiveDate {
        self.inclusive_range.end_date()
    }

    pub(crate) fn source(&self) -> CalendarDateAssignmentSource {
        CalendarDateAssignmentSource::CalendarEndpointResize
    }
}

#[derive(Clone, Debug)]
struct CalendarPageId(String);

impl CalendarPageId {
    fn parse(value: String) -> Result<Self, String> {
        if value.trim().is_empty() {
            return Err("Notion Calendar date mutation requires a block id".to_string());
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug)]
struct CalendarDate(NaiveDate);

impl CalendarDate {
    fn parse(value: String) -> Result<Self, String> {
        parse_calendar_date(&value).map(Self)
    }

    fn get(self) -> NaiveDate {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
struct InclusiveCalendarDateRange {
    start_date: CalendarDate,
    end_date: CalendarDate,
}

impl InclusiveCalendarDateRange {
    fn parse(start_date: String, end_date: String) -> Result<Self, String> {
        let start_date = CalendarDate::parse(start_date)?;
        let end_date = CalendarDate::parse(end_date)?;
        if end_date.get() < start_date.get() {
            return Err(
                "Notion Calendar inclusive end date must not precede its start date".to_string(),
            );
        }
        Ok(Self {
            start_date,
            end_date,
        })
    }

    fn start_date(self) -> NaiveDate {
        self.start_date.get()
    }

    fn end_date(self) -> NaiveDate {
        self.end_date.get()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CalendarDateAssignmentSource {
    NoDateClick,
    CalendarDayDrop,
    CalendarEndpointResize,
}

fn parse_calendar_date(value: &str) -> Result<NaiveDate, String> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| format!("invalid Notion Calendar date {value}: {error}"))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(format!("Notion Calendar date must use YYYY-MM-DD: {value}"));
    }
    Ok(date)
}
