use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_in_space_with_session, NotionPrivateApiEndpoint},
    NotionLiveError,
};

pub(crate) fn load_recent_page_visits(
    session: &NotionDesktopSession,
    space_id: &str,
    limit: u8,
) -> Result<Vec<NotionRecentPageVisit>, NotionLiveError> {
    let response = post_private_api_in_space_with_session::<_, RecentPageVisitsResponse>(
        session,
        NotionPrivateApiEndpoint::GetRecentPageVisits,
        space_id,
        &RecentPageVisitsRequest {
            before_timestamp: current_unix_timestamp_millis()?,
            limit,
            since_timestamp: 0,
            space_id,
        },
    )?;
    Ok(normalize_recent_page_visits(response.pages, limit))
}

pub(crate) fn load_recent_page_ids(
    session: &NotionDesktopSession,
    space_id: &str,
    limit: u8,
) -> Result<Vec<NotionPageId>, NotionLiveError> {
    load_recent_page_visits(session, space_id, limit)
        .map(|visits| visits.into_iter().map(|visit| visit.page_id).collect())
}

fn current_unix_timestamp_millis() -> Result<u64, NotionLiveError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            NotionLiveError::Fatal(format!("system clock predates the Unix epoch: {error}"))
        })?
        .as_millis();
    u64::try_from(millis).map_err(|_| {
        NotionLiveError::Fatal("current Unix timestamp does not fit in a u64".to_string())
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentPageVisitsRequest<'a> {
    before_timestamp: u64,
    limit: u8,
    since_timestamp: u64,
    space_id: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentPageVisitsResponse {
    pages: Vec<RecentPageVisitWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentPageVisitWire {
    id: NotionPageId,
    visited_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NotionRecentPageVisit {
    pub(crate) page_id: NotionPageId,
    pub(crate) visited_at_unix_millis: u64,
}

fn normalize_recent_page_visits(
    visits: Vec<RecentPageVisitWire>,
    limit: u8,
) -> Vec<NotionRecentPageVisit> {
    let mut latest_visit_by_page = BTreeMap::<NotionPageId, u64>::new();
    for visit in visits {
        latest_visit_by_page
            .entry(visit.id)
            .and_modify(|visited_at| *visited_at = (*visited_at).max(visit.visited_at))
            .or_insert(visit.visited_at);
    }
    let mut visits = latest_visit_by_page
        .into_iter()
        .map(|(page_id, visited_at_unix_millis)| NotionRecentPageVisit {
            page_id,
            visited_at_unix_millis,
        })
        .collect::<Vec<_>>();
    visits.sort_by(|left, right| {
        right
            .visited_at_unix_millis
            .cmp(&left.visited_at_unix_millis)
            .then_with(|| left.page_id.cmp(&right.page_id))
    });
    visits.truncate(usize::from(limit));
    visits
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct NotionPageId(Uuid);

impl NotionPageId {
    pub(crate) fn into_string(self) -> String {
        self.0.to_string()
    }

    pub(crate) fn compact(&self) -> String {
        self.0.simple().to_string()
    }
}

impl TryFrom<String> for NotionPageId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Uuid::parse_str(value.trim())
            .map(Self)
            .map_err(|error| format!("invalid Notion page ID `{value}`: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{normalize_recent_page_visits, RecentPageVisitsResponse};

    #[test]
    fn recent_page_visits_keep_latest_dedupe_sort_and_limit() {
        let response = serde_json::from_value::<RecentPageVisitsResponse>(json!({
            "pages": [
                {
                    "id": "00000000-0000-0000-0000-000000000003",
                    "visitedAt": 100
                },
                {
                    "id": "00000000-0000-0000-0000-000000000002",
                    "visitedAt": 300
                },
                {
                    "id": "00000000-0000-0000-0000-000000000004",
                    "visitedAt": 200
                },
                {
                    "id": "00000000-0000-0000-0000-000000000003",
                    "visitedAt": 200
                },
                {
                    "id": "00000000-0000-0000-0000-000000000001",
                    "visitedAt": 200
                }
            ]
        }))
        .expect("deserialize getRecentPageVisits response");

        let visits = normalize_recent_page_visits(response.pages, 3);
        let actual = visits
            .into_iter()
            .map(|visit| (visit.page_id.into_string(), visit.visited_at_unix_millis))
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                ("00000000-0000-0000-0000-000000000002".to_string(), 300),
                ("00000000-0000-0000-0000-000000000001".to_string(), 200),
                ("00000000-0000-0000-0000-000000000003".to_string(), 200),
            ]
        );
    }
}
