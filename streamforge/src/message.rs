use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Optional application-generated identifier.
    pub id: Option<String>,

    /// Opaque payload; Stream Forge does not interpret it.
    pub payload: Vec<u8>,

    /// Transport-neutral metadata.
    pub headers: BTreeMap<String, String>,
}

impl Message {
    pub fn new(payload: impl Into<Vec<u8>>) -> Self {
        Self {
            id: None,
            payload: payload.into(),
            headers: BTreeMap::new(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// A message received from a broker.
///
/// `delivery_id` is assigned by the backend and is used for acking.
/// It is separate from the optional application-generated message ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub delivery_id: String,
    pub message: Message,
}
