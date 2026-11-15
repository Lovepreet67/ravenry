use iam_contracts::events::RetryEvent; // ADJUST: real import path
use serde_json::Value;

pub const SOURCE: &str = "retry";
pub const MESSAGE_TYPE: &str = "manual_retry";
pub const SUPPORTED_VERSIONS: &[u16] = &[1];

/// Decodes raw queue bytes. Returns the typed plus the untouched
/// JSON envelope for the inbox. `Err` is a human redable reason.
pub fn decode(raw: &[u8]) -> Result<(RetryEvent, Value), String> {
    let value: Value = serde_json::from_slice(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let retry_event: RetryEvent = serde_json::from_value(value.clone())
        .map_err(|e| format!("invalid retry command outcome: {e}"))?;
    Ok((retry_event, value))
}
