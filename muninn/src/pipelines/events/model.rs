use iam_contracts::events::PublishedEvent; // ADJUST: real import path
use serde_json::Value;

pub const SOURCE: &str = "events";
pub const MESSAGE_TYPE: &str = "published_event";

/// Schema versions this service knows how to persist.
pub const SUPPORTED_VERSIONS: &[u16] = &[1];

/// Decodes and validates a raw queue message.
///
/// Returns the typed event plus the untouched JSON envelope. The inbox
/// stores the envelope, so nothing is lost even if the `events` schema
/// changes later. `Err` carries a human-readable reason for quarantine.
pub fn decode(raw: &[u8]) -> Result<(PublishedEvent, Value), String> {
    let value: Value = serde_json::from_slice(raw).map_err(|e| format!("invalid JSON: {e}"))?;

    let event: PublishedEvent = serde_json::from_value(value.clone())
        .map_err(|e| format!("invalid event envelope: {e}"))?;

    if !SUPPORTED_VERSIONS.contains(&event.schema_version) {
        return Err(format!(
            "unsupported schema_version {}",
            event.schema_version
        ));
    }
    Ok((event, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample() -> PublishedEvent {
        PublishedEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Vec::new(),
            "orders/created".into(),
            serde_json::json!({ "order": 1 }),
        )
    }

    #[test]
    fn roundtrip_keeps_full_envelope() {
        let e = sample();
        let raw = serde_json::to_vec(&e).unwrap();
        let (decoded, value) = decode(&raw).unwrap();
        assert_eq!(decoded.event_id, e.event_id);
        assert_eq!(value["topic_path"], "orders/created");
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode(b"not json").is_err());
        assert!(decode(br#"{"hello":"world"}"#).is_err());
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let mut e = sample();
        e.schema_version = 99;
        let raw = serde_json::to_vec(&e).unwrap();
        assert!(decode(&raw).unwrap_err().contains("unsupported"));
    }
}
