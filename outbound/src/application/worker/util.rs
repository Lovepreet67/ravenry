use std::time::Duration;

use tokio_util::sync::CancellationToken;

pub async fn sleep_or_cancel(token: &CancellationToken, duration: Duration) {
    tokio::select! {
        _ = token.cancelled() => {}
        _ = tokio::time::sleep(duration) => {}
    }
}
