pub mod models;
mod supervisor;
mod usda;

use std::collections::BinaryHeap;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Duration;
use derive_more::{Display, Error, From};
use models::aggregation_metadata::AggregateMetadataModel;
use sqlx::PgPool;
use sqlx::types::chrono::Utc;
use supervisor::{FoodData, SupervisorError};
use tokio::sync::{Mutex, Notify};
use tokio::time::Instant;
use usda::{UsdaAggregator, UsdaClient};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Display, Error, From)]
pub enum SourceError {
    #[from]
    Database(sqlx::Error),
    #[from]
    Deserialize(reqwest::Error),
}

pub trait FoodSource: Send + Sync {
    type Data: FoodData;

    fn name(&self) -> &str;
    fn is_finished(&self, current_page: usize) -> bool;
    fn fetch(&self, page: usize) -> impl Future<Output = Result<Self::Data, SourceError>> + Send;
}

#[derive(Debug)]
pub enum AggregateStatus {
    Finished,
    PendingUntil(Instant),
}

#[derive(Debug, Display, Error, From)]
pub enum AggregatorError {
    UnexpectedRateLimit,
    #[from]
    Database(sqlx::Error),
    #[from]
    Supervisor(SupervisorError),
    #[from]
    FoodSource(SourceError),
}

pub trait Aggregator: Send + Sync {
    fn aggregate(&mut self, conn: PgPool) -> BoxFuture<Result<AggregateStatus, AggregatorError>>;
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

#[tracing::instrument(skip_all)]
pub async fn aggregate_food_data(pool: PgPool) -> Result<AggregateStatus, AggregatorError> {
    tracing::info!("Starting aggregation workflow");
    let mut conn = pool.acquire().await?;
    // TODO: probably not just propagate the error up here
    let last_run_entry =
        models::aggregation_metadata::AggregateMetadataModel::get_last_run(conn.as_mut()).await?;

    let should_run = match &last_run_entry {
        Some(entry) => Utc::now() - entry.last_run >= Duration::days(30),
        None => true,
    };

    if !should_run {
        tracing::info!("Not enough time has passed since last aggregation");
        // Safety: if should_run is false then an entry must exist.
        let entry = last_run_entry.unwrap();
        let next_run_time = entry.last_run + Duration::days(30);
        let wait_until = (next_run_time - Utc::now()).to_std().unwrap_or_default();
        return Ok(AggregateStatus::PendingUntil(Instant::now() + wait_until));
    };

    let client = UsdaClient::new();
    let usda_aggregator = UsdaAggregator::new(client);

    let queue = Arc::new(Mutex::new(BinaryHeap::new()));
    let active_handles = Arc::new(Mutex::new(Vec::new()));
    let notify = Arc::new(Notify::new());

    queue.lock().await.push(ScheduledAggregator {
        aggregator: Box::new(usda_aggregator),
        wake_time: Instant::now(),
    });

    loop {
        let mut queue_guard = queue.lock().await;
        let maybe_task_time = queue_guard.peek().map(|task| task.wake_time);

        match maybe_task_time {
            // When the task is pending until a future time, we wait either until that time come,
            // or a new notification is received, which could be from a task that has a earlier
            // wait time and should become the new binary heap head
            Some(wake_time) if wake_time > Instant::now() => {
                tokio::select! {
                    _ = tokio::time::sleep_until(wake_time) => {},
                    _ = notify.notified() => {},
                }
                continue;
            }
            // When the task can be started, we add it to the active handles vector and spawn a
            // task for it, the task will notify once its finished, no matter the result
            Some(_) => {
                let mut task = queue_guard.pop().unwrap();

                let pool = pool.clone();
                let queue = queue.clone();
                let notify = notify.clone();

                let handle = tokio::spawn(async move {
                    match task.aggregator.aggregate(pool.clone()).await {
                        Err(_) => {}
                        Ok(AggregateStatus::Finished) => {
                            tracing::info!("Finished aggregation");
                        }
                        Ok(AggregateStatus::PendingUntil(when)) => {
                            task.wake_time = when;
                            queue.lock().await.push(task);
                        }
                    }

                    notify.notify_one();
                    Ok::<(), AggregatorError>(())
                });

                active_handles.lock().await.push(handle);
            }
            // Drains every completed handle from the handles vector, and waits them to check if
            // the task succeeded or errored, and if after draining everything, both the active
            // handles and the queue are empty, we are done syncing
            None => {
                let mut handles_guard = active_handles.lock().await;
                let (complete, pending) = handles_guard
                    .drain(..)
                    .partition::<Vec<_>, _>(|h| h.is_finished());

                *handles_guard = pending;

                for handle in complete {
                    if handle.await.is_err() {
                        todo!();
                    }
                }

                if queue_guard.is_empty() && handles_guard.is_empty() {
                    tracing::info!("No more work to be done");
                    break;
                }

                notify.notified().await;
            }
        }
    }

    AggregateMetadataModel::create(conn.as_mut()).await?;
    tracing::info!("Aggregation metadata stored");

    Ok(AggregateStatus::Finished)
}
