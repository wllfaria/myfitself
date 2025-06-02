use std::collections::HashMap;
use std::sync::Arc;

use derive_more::Display;
use governor::RateLimiter;
use governor::clock::{Clock, QuantaClock, Reference};
use governor::state::{InMemoryState, NotKeyed};
use sqlx::PgConnection;
use tokio::task::JoinHandle;

use crate::models::food_nutrients::{CreateFoodNutrientPayload, FoodNutrients};
use crate::models::food_sources::{CreateFoodSourcePayload, FoodSources};
use crate::models::foods::{CreateFoodPayload, Foods};
use crate::models::nutrients::Nutrients;
use crate::models::units::Units;
use crate::models::wweia_categories::{CreateWWEIACategoryPayload, WWEIACategories};
use crate::{AggregateStatus, FoodSource};

pub trait FoodData {
    type Entry: FoodEntry + Send + Sync;
    type EntryIter: Iterator<Item = Self::Entry> + Send + Sync;

    fn entries(self) -> Self::EntryIter;
}

pub trait FoodEntry {
    type Nutrient: FoodEntryNutrient + Send + Sync;
    type NutrientIter: Iterator<Item = Self::Nutrient> + Send + Sync;

    fn source(&self) -> String;
    fn wweia_data(&self) -> (Option<i32>, Option<&String>);
    fn name(&self) -> &str;
    fn fndds_code(&self) -> Option<i32>;
    fn id(&self) -> i32;
    fn nutrients(self) -> Self::NutrientIter;
}

pub trait FoodEntryNutrient {
    fn name(&self) -> &str;
    fn unit_name(&self) -> &str;
    fn value(&self) -> f32;
}

#[derive(Debug)]
enum WorkerMessage {
    Finished(WorkerId),
}

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(usize);

impl WorkerId {
    fn next(&mut self) {
        self.0 += 1;
    }
}

#[derive(Debug)]
pub struct AggregatorSupervisor<'a, C, D>
where
    C: FoodSource<Data = D> + Send + Sync + 'static,
    D: FoodData + Send + Sync + 'static,
{
    worker_id: WorkerId,
    task_bound: usize,
    limiter: &'a mut RateLimiter<NotKeyed, InMemoryState, QuantaClock>,
    workers: HashMap<WorkerId, JoinHandle<anyhow::Result<D>>>,
    client: Arc<C>,
}

impl<'a, C, D> AggregatorSupervisor<'a, C, D>
where
    C: FoodSource<Data = D> + Send + Sync + 'static,
    D: FoodData + Send + Sync + 'static,
{
    pub fn new(
        limiter: &'a mut RateLimiter<NotKeyed, InMemoryState, QuantaClock>,
        client: Arc<C>,
        total_pages: usize,
    ) -> Self {
        let remaining_pages = total_pages - 1;
        // Limit concurrent requests to avoid spamming the data source API
        let task_bound = usize::min(10, remaining_pages);

        Self {
            client,
            limiter,
            task_bound,
            worker_id: WorkerId::default(),
            workers: HashMap::with_capacity(task_bound),
        }
    }

    #[tracing::instrument(skip(self, tx), fields(source = %self.client.name()))]
    pub async fn run(&mut self, tx: &mut PgConnection) -> anyhow::Result<AggregateStatus> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(self.task_bound);
        // Start from page 2 as first page will always be fetched outside to get the total_pages
        // data from the api
        // TODO: maybe receive this as an argument
        let mut current_page = 2;
        tracing::info!(%current_page, "supervisor starting");

        loop {
            while self.workers.len() < self.task_bound {
                if self.client.is_finished(current_page) {
                    break;
                }

                if let Err(err) = self.limiter.check() {
                    let now = governor::clock::QuantaClock::default().now();
                    let earliest = err.earliest_possible();

                    let wait_duration = earliest.duration_since(now);
                    let wake_time = tokio::time::Instant::now() + wait_duration.into();
                    return Ok(AggregateStatus::PendingUntil(wake_time));
                }

                let sender = sender.clone();
                let client = self.client.clone();
                let worker_id = self.worker_id;
                self.worker_id.next();

                let span = tracing::span!(
                    tracing::Level::INFO,
                    "page_worker",
                    %worker_id,
                    %current_page
                );

                let handle = tokio::spawn(async move {
                    let _guard = span.enter();
                    tracing::info!("Worker started");

                    let sender = sender.clone();
                    let result = match client.fetch(current_page).await {
                        Ok(data) => {
                            tracing::debug!("Fetched page successfully");
                            data
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "Failed to fetch page");
                            return Err(e);
                        }
                    };

                    if let Err(e) = sender.send(WorkerMessage::Finished(worker_id)).await {
                        tracing::error!(error = ?e, "Failed to send finished message");
                        return Err(e.into());
                    };

                    Ok(result)
                });

                current_page += 1;
                self.workers.insert(worker_id, handle);
            }

            if self.workers.is_empty() {
                // This is a warning as its very suspicious if there is no content on a data source
                tracing::warn!("No work to be done");
                return Ok(AggregateStatus::Finished);
            }

            while let Some(message) = receiver.recv().await {
                match message {
                    WorkerMessage::Finished(worker_id) => {
                        let result = self
                            .workers
                            .remove(&worker_id)
                            .expect("unexisting worker id sent through channel")
                            .await?;

                        match result {
                            Ok(data) => {
                                tracing::debug!(%worker_id, "Persisting food data");
                                persist_food_data(tx, data).await?;
                                tracing::info!(%worker_id, "Data persisted");
                            }
                            Err(e) => tracing::error!(?worker_id, error = ?e, "Worker failed"),
                        };
                    }
                }

                if self.client.is_finished(current_page) && self.workers.is_empty() {
                    tracing::info!("All workers completed");
                    return Ok(AggregateStatus::Finished);
                }
            }
        }
    }
}

pub async fn persist_food_data<D>(tx: &mut PgConnection, data: D) -> anyhow::Result<()>
where
    D: FoodData + Send + Sync,
{
    for entry in data.entries() {
        let source = entry.source();
        let source = FoodSources::maybe_create(tx, CreateFoodSourcePayload::new(source)).await?;

        let category_id = match entry.wweia_data() {
            (Some(id), Some(name)) => {
                let payload = CreateWWEIACategoryPayload::new(id, name);
                let category = WWEIACategories::maybe_create(tx, payload).await?;
                Some(category.id)
            }
            _ => None,
        };

        let payload = CreateFoodPayload::new(
            entry.name(),
            entry.fndds_code(),
            source.id,
            entry.id(),
            category_id,
        );
        let stored_food = Foods::create_or_update(tx, payload).await?;

        for nutrient in entry.nutrients() {
            let stored_nutrient = Nutrients::maybe_create(tx, nutrient.name()).await?;
            let unit = Units::maybe_create(tx, nutrient.unit_name()).await?;

            let payload = CreateFoodNutrientPayload::new(
                stored_food.id,
                stored_nutrient.id,
                unit.id,
                source.id,
                nutrient.value(),
            );
            FoodNutrients::create_or_update(tx, payload).await?;
        }
    }

    Ok(())
}
