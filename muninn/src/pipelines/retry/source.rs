use iam_contracts::streams::MANUALY_RETRY;

use crate::core::StaticSource;

/// All outcomes arrive on one global stream.
pub fn source() -> StaticSource {
    StaticSource::new([MANUALY_RETRY])
}
