use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use derive_more::{Display, Error, From};
use governor::RateLimiter;
use governor::clock::{Clock, QuantaClock, Reference};
use governor::state::{InMemoryState, NotKeyed};
use sqlx::{PgConnection, QueryBuilder};
use tokio::task::{JoinError, JoinHandle};

use crate::models::food_nutrients::{CreateFoodNutrientPayload, FoodNutrients};
use crate::models::food_sources::{CreateFoodSourcePayload, FoodSources};
use crate::models::foods::{CreateFoodPayload, Foods};
use crate::models::nutrients::Nutrients;
use crate::models::units::Units;
use crate::models::wweia_categories::{CreateWWEIACategoryPayload, WWEIACategories};
use crate::{AggregateStatus, FoodSource, SourceError};

pub trait FoodData {
    type Entry: FoodEntry + Send + Sync;
    type EntryIter<'a>: Iterator<Item = &'a Self::Entry> + Send + Sync
    where
        Self: 'a;

    fn entries(&self) -> Self::EntryIter<'_>;
}

pub trait FoodEntry {
    type Nutrient: FoodEntryNutrient + Send + Sync;
    type NutrientIter<'a>: Iterator<Item = &'a Self::Nutrient> + Send + Sync
    where
        Self: 'a;

    fn source(&self) -> String;
    fn wweia_data(&self) -> Option<(i32, &String)>;
    fn name(&self) -> &str;
    fn fndds_code(&self) -> Option<i32>;
    fn id(&self) -> i32;
    fn nutrients(&self) -> Self::NutrientIter<'_>;
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

#[derive(Debug, Display, Error, From)]
pub enum WorkerError {
    #[display("{_0}")]
    #[error(ignore)]
    SendMessage(String),
    #[from]
    FoodSource(SourceError),
}

#[derive(Debug, Display, Error, From)]
pub enum SupervisorError {
    #[from]
    Database(sqlx::Error),
    #[from]
    Join(JoinError),
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
    workers: HashMap<WorkerId, JoinHandle<Result<D, WorkerError>>>,
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
    pub async fn run(&mut self, tx: &mut PgConnection) -> Result<AggregateStatus, SupervisorError> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(self.task_bound);
        // Start from page 2 as first page will always be fetched outside to get the total_pages
        // data from the api
        // TODO: maybe receive this as an argument
        let mut current_page = 2;
        tracing::info!(%current_page, "supervisor starting");
        let mut status = AggregateStatus::Finished;

        loop {
            while self.workers.len() < self.task_bound {
                // stop creating workers if the client is finished
                if self.client.is_finished(current_page) {
                    break;
                }

                // if we hit the rate limit, we stop creating workers, but cache the status to
                // return later
                if let Err(err) = self.limiter.check() {
                    let now = governor::clock::QuantaClock::default().now();
                    let earliest = err.earliest_possible();

                    let wait_duration = earliest.duration_since(now);
                    let wake_time = tokio::time::Instant::now() + wait_duration.into();
                    status = AggregateStatus::PendingUntil(wake_time);
                    break;
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
                            tracing::info!("Fetched page successfully");
                            data
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "Failed to fetch page");
                            // TODO: this is bad as every other worker will start erroring due to
                            // the channel closing
                            return Err(e.into());
                        }
                    };

                    if let Err(e) = sender.send(WorkerMessage::Finished(worker_id)).await {
                        tracing::error!(error = ?e, "Failed to send finished message");
                        // TODO: this is bad as every other worker will start erroring due to the
                        // channel closing
                        return Err(WorkerError::SendMessage(format!(
                            "worker {worker_id} failed to send message through channel"
                        )));
                    };

                    Ok(result)
                });

                current_page += 1;
                self.workers.insert(worker_id, handle);
            }

            if self.workers.is_empty() {
                break;
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
                                let now = std::time::Instant::now();
                                tracing::debug!(%worker_id, "Persisting food data");
                                persist_food_data(tx, data).await?;
                                tracing::info!(%worker_id, "Data persisted, took: {took:?}", took=now.elapsed());
                                break;
                            }
                            Err(e) => tracing::error!(?worker_id, error = ?e, "Worker failed"),
                        };
                    }
                }

                if self.client.is_finished(current_page) && self.workers.is_empty() {
                    tracing::info!("All workers completed");
                    break;
                }
            }
        }

        Ok(status)
    }
}

pub async fn persist_food_data<D>(tx: &mut PgConnection, data: D) -> Result<(), SupervisorError>
where
    D: FoodData + Send + Sync,
{
    let mut sources = HashSet::new();
    let mut categories = HashSet::new();
    let mut nutrients = HashSet::new();
    let mut units = HashSet::new();

    for entry in data.entries() {
        sources.insert(entry.source());

        if let Some((id, name)) = entry.wweia_data() {
            categories.insert((id, name));
        }

        for nutrient in entry.nutrients() {
            nutrients.insert(nutrient.name());
            units.insert(nutrient.unit_name());
        }
    }

    let source_id_map = FoodSources::maybe_create_bulk(tx, sources.into_iter()).await?;
    let category_id_map = WWEIACategories::maybe_create_bulk(tx, categories.into_iter()).await?;
    let nutient_id_map = Nutrients::maybe_create_bulk(tx, nutrients.into_iter()).await?;
    let unit_id_map = Units::maybe_create_bulk(tx, units.into_iter()).await?;

    let mut foods = vec![];
    for entry in data.entries() {
        let source_id = source_id_map[&entry.source()];
        let category_id = entry
            .wweia_data()
            .and_then(|(_, name)| category_id_map.get(name).copied());

        let payload = CreateFoodPayload::new(
            entry.name(),
            entry.fndds_code(),
            source_id,
            entry.id(),
            category_id,
        );
        foods.push(payload);
    }

    let food_id_map = Foods::create_or_update_bulk(tx, foods.into_iter()).await?;

    for entry in data.entries() {
        let food_key = (entry.source(), entry.id());
        let food_id = food_id_map[&food_key];
        let source_id = source_id_map[&entry.source()];
        let mut food_nutrients = vec![];

        for nutrient in entry.nutrients() {
            let nutrient_id = nutient_id_map[nutrient.name()];
            let unit_id = unit_id_map[nutrient.unit_name()];

            let payload = CreateFoodNutrientPayload::new(
                food_id,
                nutrient_id,
                unit_id,
                source_id,
                nutrient.value(),
            );

            food_nutrients.push(payload);
        }

        FoodNutrients::create_or_update_bulk(tx, food_nutrients).await?;
    }

    Ok(())
}
