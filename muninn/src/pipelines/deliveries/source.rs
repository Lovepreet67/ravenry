use iam_contracts::streams::DELIVERY_RESULTS;

use crate::core::StaticSource;

/// All outcomes arrive on one global stream.
pub fn source() -> StaticSource {
    StaticSource::new([DELIVERY_RESULTS])
}
