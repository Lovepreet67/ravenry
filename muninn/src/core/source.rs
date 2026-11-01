use async_trait::async_trait;

use crate::error::Result;

/// Answers one question: which streams should be consumed right now?
///
/// The ingester calls this periodically and reconciles. Streams that
/// appear get a consumer group and a reader. Streams that disappear are
/// dropped. A static source returns the same list every time; a
/// registry-backed one returns whatever is currently active.
#[async_trait]
pub trait StreamSource: Send + Sync {
    async fn streams(&self) -> Result<Vec<String>>;
}

/// A fixed list of streams, e.g. the global delivery results stream.
#[derive(Debug, Clone)]
pub struct StaticSource {
    streams: Vec<String>,
}

impl StaticSource {
    pub fn new(streams: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            streams: streams.into_iter().map(Into::into).collect(),
        }
    }
}

#[async_trait]
impl StreamSource for StaticSource {
    async fn streams(&self) -> Result<Vec<String>> {
        Ok(self.streams.clone())
    }
}