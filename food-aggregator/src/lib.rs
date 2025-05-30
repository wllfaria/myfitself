use std::collections::BinaryHeap;
use std::pin::Pin;

use tokio::time::Instant;
use usda::{UsdaAggregator, UsdaClient};

mod usda;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum FoodSourceStatus {
    HasRemainingResults,
    SyncFinished,
}

#[derive(Debug)]
pub struct FoodSourceData<D> {
    pub data: D,
    pub status: FoodSourceStatus,
}

pub trait FoodSource: Send + Sync {
    type Data;

    fn fetch_next(&mut self) -> impl Future<Output = FoodSourceData<Self::Data>>;
}

#[derive(Debug)]
pub enum AggregateStatus {
    Finished,
    PendingUntil(Instant),
}

pub trait Aggregator {
    fn aggregate(&mut self) -> BoxFuture<AggregateStatus>;
}

struct ScheduledAggregator {
    aggregator: Box<dyn Aggregator>,
    wake_time: Instant,
}

impl Ord for ScheduledAggregator {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.wake_time.cmp(&self.wake_time)
    }
}

impl PartialOrd for ScheduledAggregator {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScheduledAggregator {
    fn eq(&self, other: &Self) -> bool {
        self.wake_time == other.wake_time
    }
}

impl Eq for ScheduledAggregator {}

pub async fn aggregate_food_data() {
    let client = UsdaClient::new();
    let usda_aggregator = UsdaAggregator::new(client);

    let mut queue = BinaryHeap::new();

    queue.push(ScheduledAggregator {
        aggregator: Box::new(usda_aggregator),
        wake_time: Instant::now(),
    });

    while let Some(mut task) = queue.pop() {
        let now = Instant::now();
        if task.wake_time > now {
            let delay = task.wake_time - now;
            tokio::time::sleep_until(tokio::time::Instant::now() + delay).await;
        }

        if let AggregateStatus::PendingUntil(when) = task.aggregator.aggregate().await {
            task.wake_time = when;
            queue.push(task);
        }
    }
}
