use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_LIMIT: u32 = 50;
pub const MAX_LIMIT: u32 = 200;

/// Keyset position: the sort timestamp plus the row id as a tiebreaker.
/// Lists are ordered `(ts DESC, id DESC)` and resume strictly after this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub ts: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid pagination cursor")]
pub struct InvalidCursor;

impl Cursor {
    /// Opaque to clients. Not a secret, and not tamper-proof: a forged
    /// cursor can only move within the caller's own tenant-scoped results.
    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("cursor serializes");
        URL_SAFE_NO_PAD.encode(json)
    }

    pub fn decode(s: &str) -> Result<Self, InvalidCursor> {
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| InvalidCursor)?;
        serde_json::from_slice(&bytes).map_err(|_| InvalidCursor)
    }
}

#[derive(Debug, Clone)]
pub struct PageRequest {
    pub cursor: Option<Cursor>,
    pub limit: u32,
}

impl PageRequest {
    pub fn new(cursor: Option<&str>, limit: Option<u32>) -> Result<Self, InvalidCursor> {
        Ok(Self {
            cursor: cursor.map(Cursor::decode).transpose()?,
            limit: limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        })
    }

    /// Repositories fetch one extra row to learn whether another page exists.
    pub fn fetch_limit(&self) -> i64 {
        i64::from(self.limit) + 1
    }
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl<T> Page<T> {
    /// `rows` is what the query returned (up to `limit + 1`).
    pub fn from_fetched(mut rows: Vec<T>, limit: u32, key: impl Fn(&T) -> Cursor) -> Self {
        let limit = limit as usize;
        let next_cursor = if rows.len() > limit {
            rows.truncate(limit);
            rows.last().map(|r| key(r).encode())
        } else {
            None
        };
        Self {
            items: rows,
            next_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        let c = Cursor {
            ts: Utc::now(),
            id: Uuid::new_v4(),
        };
        assert_eq!(Cursor::decode(&c.encode()).unwrap(), c);
    }

    #[test]
    fn garbage_cursor_is_rejected() {
        assert!(Cursor::decode("not-a-cursor").is_err());
    }

    #[test]
    fn limit_is_clamped() {
        assert_eq!(
            PageRequest::new(None, Some(10_000)).unwrap().limit,
            MAX_LIMIT
        );
        assert_eq!(PageRequest::new(None, Some(0)).unwrap().limit, 1);
        assert_eq!(PageRequest::new(None, None).unwrap().limit, DEFAULT_LIMIT);
    }

    #[test]
    fn extra_row_becomes_next_cursor() {
        let rows: Vec<(DateTime<Utc>, Uuid)> =
            (0..3).map(|_| (Utc::now(), Uuid::new_v4())).collect();
        let page = Page::from_fetched(rows.clone(), 2, |r| Cursor { ts: r.0, id: r.1 });
        assert_eq!(page.items.len(), 2);
        assert!(page.next_cursor.is_some());

        let last = Page::from_fetched(rows[..2].to_vec(), 2, |r| Cursor { ts: r.0, id: r.1 });
        assert!(last.next_cursor.is_none());
    }
}
