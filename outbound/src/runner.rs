use std::{sync::Arc, time::Duration};

use common::queue::Queue;
use tokio::time::sleep;
pub struct Runner {
    queue: Arc<dyn Queue>,
}
impl Runner {
    pub fn new(queue: Arc<dyn Queue>) -> Self {
        Self { queue }
    }

    pub async fn start(&self) -> ! {
        loop {
            match self.queue.consume("events", "g1", "test", 10, 100).await {
                Err(e) => {
                    tracing::error!("{:?}", e);
                }
                Ok(v) => {
                    tracing::info!("{:?}", v);
                    for message in v {
                        // if let Err(e) = self.queue.ack("events", "g1", &message.id).await {
                        //     tracing::error!("{:?}", e);
                        // }
                    }
                }
            }
            sleep(Duration::new(10, 0)).await;
        }
    }
}
