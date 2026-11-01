use iam_contracts::delivery_outcome::DeliveryOutcome; // ADJUST: real import path
use serde_json::Value;

pub const SOURCE: &str = "deliveries";
pub const MESSAGE_TYPE: &str = "delivery_outcome";
pub const SUPPORTED_VERSIONS: &[u16] = &[1];

/// Logical delivery status derived from one attempt's outcome.
pub fn status_for(o: &DeliveryOutcome) -> &'static str {
    if o.success {
        "success"
    } else if o.will_retry {
        "pending"
    } else {
        "failed"
    }
}

/// Structural and version checks shared by `prepare` and the projector.
pub fn validate(o: &DeliveryOutcome) -> Result<(), String> {
    if !SUPPORTED_VERSIONS.contains(&o.schema_version) {
        return Err(format!("unsupported schema_version {}", o.schema_version));
    }
    if o.attempt == 0 {
        return Err("attempt must be >= 1".into());
    }
    Ok(())
}

/// Decodes raw queue bytes. Returns the typed outcome plus the untouched
/// JSON envelope for the inbox. `Err` is a quarantine reason.
pub fn decode(raw: &[u8]) -> Result<(DeliveryOutcome, Value), String> {
    let value: Value = serde_json::from_slice(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let outcome: DeliveryOutcome = serde_json::from_value(value.clone())
        .map_err(|e| format!("invalid delivery outcome: {e}"))?;
    validate(&outcome)?;
    Ok((outcome, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample(success: bool, will_retry: bool) -> DeliveryOutcome {
        DeliveryOutcome {
            schema_version: 1,
            delivery_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            attempt: 1,
            event_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            topic_id: Uuid::new_v4(),
            destination_id: Uuid::new_v4(),
            success,
            will_retry,
            status_code: Some(500),
            error: None,
            attempted_at: Utc::now(),
        }
    }

    #[test]
    fn status_mapping() {
        assert_eq!(status_for(&sample(true, false)), "success");
        assert_eq!(status_for(&sample(false, true)), "pending");
        assert_eq!(status_for(&sample(false, false)), "failed");
    }

    #[test]
    fn roundtrip() {
        let o = sample(false, true);
        let raw = serde_json::to_vec(&o).unwrap();
        let (d, _) = decode(&raw).unwrap();
        assert_eq!(d.attempt_id, o.attempt_id);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(decode(b"nope").is_err());

        let mut o = sample(true, false);
        o.attempt = 0;
        assert!(decode(&serde_json::to_vec(&o).unwrap()).is_err());

        let mut o = sample(true, false);
        o.schema_version = 99;
        assert!(decode(&serde_json::to_vec(&o).unwrap())
            .unwrap_err()
            .contains("unsupported"));
    }
}
