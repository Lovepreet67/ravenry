use iam_contracts::streams::DELIVERY_RESULTS; // ADJUST

use crate::core::StaticSource;

/// All outcomes arrive on one global stream.
pub fn source() -> StaticSource {
    StaticSource::new([DELIVERY_RESULTS])
}
