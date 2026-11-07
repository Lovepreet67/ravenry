use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::TenantScope;
use crate::domain::delivery::{Delivery, DeliveryFilter, DeliveryStatus};
use crate::domain::delivery_attempt::DeliveryAttempt;
use crate::domain::event::{EventDetail, EventFilter, EventSummary};
use crate::domain::pagination::{Page, PageRequest};
use crate::error::{AppError, AppResult};

// ---------- shared ----------

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    /// Pass back as `?cursor=` to get the next page. Absent on the last page.
    pub next_cursor: Option<String>,
}

impl<T> ListResponse<T> {
    pub fn from_page<D>(page: Page<D>, map: impl Fn(D) -> T) -> Self {
        Self {
            items: page.items.into_iter().map(map).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

fn check_range(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> AppResult<()> {
    match (from, to) {
        (Some(f), Some(t)) if f >= t => Err(AppError::BadRequest(
            "`from` must be earlier than `to`".into(),
        )),
        _ => Ok(()),
    }
}

// ---------- events ----------

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    pub topic_id: Option<Uuid>,
    pub topic_path: Option<String>,
    /// RFC 3339, inclusive. Use `Z`, since a literal `+` in a query string
    /// must be percent-encoded.
    pub from: Option<DateTime<Utc>>,
    /// RFC 3339, exclusive.
    pub to: Option<DateTime<Utc>>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

impl EventListQuery {
    pub fn into_parts(self) -> AppResult<(EventFilter, PageRequest)> {
        check_range(self.from, self.to)?;
        let page = PageRequest::new(self.cursor.as_deref(), self.limit)?;
        let filter = EventFilter {
            topic_id: self.topic_id,
            topic_path: self.topic_path,
            from: self.from,
            to: self.to,
        };
        Ok((filter, page))
    }
}

#[derive(Debug, Serialize)]
pub struct EventSummaryDto {
    pub event_id: Uuid,
    pub topic_id: Uuid,
    pub topic_path: String,
    pub schema_version: i32,
    pub received_at: DateTime<Utc>,
}

impl From<EventSummary> for EventSummaryDto {
    fn from(e: EventSummary) -> Self {
        Self {
            event_id: e.event_id,
            topic_id: e.topic_id,
            topic_path: e.topic_path,
            schema_version: e.schema_version,
            received_at: e.received_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EventDetailDto {
    #[serde(flatten)]
    pub summary: EventSummaryDto,
    /// Omitted when the caller's role may not view payloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Lets the UI show "hidden for your role" rather than "no payload".
    pub payload_redacted: bool,
}

impl EventDetailDto {
    /// The single place where payload redaction happens.
    pub fn from_domain(detail: EventDetail, scope: &TenantScope) -> Self {
        let visible = scope.can_view_payload();
        Self {
            summary: detail.summary.into(),
            payload: visible.then_some(detail.payload),
            payload_redacted: !visible,
        }
    }
}

// ---------- deliveries ----------

#[derive(Debug, Deserialize)]
pub struct DeliveryListQuery {
    pub event_id: Option<Uuid>,
    pub destination_id: Option<Uuid>,
    /// `pending`, `success`, or `failed`.
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

impl DeliveryListQuery {
    pub fn into_parts(self) -> AppResult<(DeliveryFilter, PageRequest)> {
        check_range(self.from, self.to)?;
        let status = self
            .status
            .as_deref()
            .map(|s| {
                DeliveryStatus::parse(s)
                    .ok_or_else(|| AppError::BadRequest(format!("unknown status '{s}'")))
            })
            .transpose()?;
        let page = PageRequest::new(self.cursor.as_deref(), self.limit)?;
        let filter = DeliveryFilter {
            event_id: self.event_id,
            destination_id: self.destination_id,
            status,
            from: self.from,
            to: self.to,
        };
        Ok((filter, page))
    }
}

#[derive(Debug, Serialize)]
pub struct DeliveryDto {
    pub delivery_id: Uuid,
    pub event_id: Uuid,
    pub topic_id: Uuid,
    pub destination_id: Uuid,
    pub status: &'static str,
    pub current_attempt: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Delivery> for DeliveryDto {
    fn from(d: Delivery) -> Self {
        Self {
            delivery_id: d.delivery_id,
            event_id: d.event_id,
            topic_id: d.topic_id,
            destination_id: d.destination_id,
            status: d.status.as_str(),
            current_attempt: d.current_attempt,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AttemptDto {
    pub attempt_id: Uuid,
    pub attempt_number: i32,
    pub success: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub attempted_at: DateTime<Utc>,
}

impl From<DeliveryAttempt> for AttemptDto {
    fn from(a: DeliveryAttempt) -> Self {
        // `recorded_at` and `delivery_id` are internal detail, so not exposed.
        Self {
            attempt_id: a.attempt_id,
            attempt_number: a.attempt_number,
            success: a.success,
            status_code: a.status_code,
            error: a.error,
            attempted_at: a.attempted_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q() -> EventListQuery {
        EventListQuery {
            topic_id: None,
            topic_path: None,
            from: None,
            to: None,
            cursor: None,
            limit: None,
        }
    }

    #[test]
    fn inverted_range_is_rejected() {
        let now = Utc::now();
        let mut query = q();
        query.from = Some(now);
        query.to = Some(now - chrono::Duration::hours(1));
        assert!(query.into_parts().is_err());
    }

    #[test]
    fn bad_cursor_is_a_bad_request() {
        let mut query = q();
        query.cursor = Some("garbage".into());
        assert!(matches!(query.into_parts(), Err(AppError::BadRequest(_))));
    }

    #[test]
    fn unknown_status_is_rejected() {
        let query = DeliveryListQuery {
            event_id: None,
            destination_id: None,
            status: Some("bogus".into()),
            from: None,
            to: None,
            cursor: None,
            limit: None,
        };
        assert!(query.into_parts().is_err());
    }
}
