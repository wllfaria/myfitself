mod models;
mod usda;

use std::collections::BinaryHeap;
use std::pin::Pin;

use chrono::Duration;
use sqlx::PgPool;
use sqlx::types::chrono::Utc;
use tokio::time::Instant;
use usda::{UsdaAggregator, UsdaClient};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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

    fn fetch_next(&mut self) -> impl Future<Output = FoodSourceData<Self::Data>> + Send;
}

#[derive(Debug)]
pub enum AggregateStatus {
    Finished,
    PendingUntil(Instant),
}

pub trait Aggregator: Send + Sync {
    fn aggregate(&mut self, pool: PgPool) -> BoxFuture<anyhow::Result<AggregateStatus>>;
}

struct ScheduledAggregator {
    aggregator: Box<dyn Aggregator + Send + Sync>,
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

pub async fn aggregate_food_data(pool: PgPool) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    // TODO: probably not just propagate the error up here
    let last_run_entry =
        models::aggregation_metadata::AggregateMetadataModel::get_last_run(conn.as_mut()).await?;

    let should_run = match last_run_entry {
        Some(entry) => Utc::now() - entry.last_run >= Duration::days(30),
        None => true,
    };

    if !should_run {
        return Ok(());
    };

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

        if let AggregateStatus::PendingUntil(when) = task.aggregator.aggregate(pool.clone()).await?
        {
            task.wake_time = when;
            queue.push(task);
        }
    }

    Ok(())
}
