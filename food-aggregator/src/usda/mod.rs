mod usda_client;
mod usda_types;

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::QuantaClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
pub use usda_client::UsdaClient;
use usda_types::UsdaFoodSearchResponse;

use crate::supervisor::{AggregatorSupervisor, persist_food_data};
use crate::{AggregateStatus, Aggregator, BoxFuture, FoodSource};

#[derive(Debug)]
pub struct UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse>,
{
    limiter: RateLimiter<NotKeyed, InMemoryState, QuantaClock>,
    client: Arc<C>,
}

impl<C> UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse>,
{
    pub fn new(client: C) -> Self {
        let quota = Quota::per_hour(NonZeroU32::new(1000).unwrap());
        let limiter = RateLimiter::direct(quota);

        Self {
            limiter,
            client: Arc::new(client),
        }
    }
}

impl<C> Aggregator for UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse> + 'static,
{
    #[tracing::instrument(skip(self, pool))]
    fn aggregate(&mut self, pool: PgPool) -> BoxFuture<anyhow::Result<AggregateStatus>> {
        Box::pin(async move {
            // Use one entry from limiter to account for the first request
            // Safety: first request will not fail rate-limit.
            if let Err(e) = self.limiter.check() {
                return Err(anyhow::anyhow!("Rate limit exceeded unexpectedly: {}", e));
            }
            let mut tx = pool.begin().await?;

            // This first request is made separately in order to fetch the total_pages from USDA
            // api, so that we can coordinate the concurrent syncing
            let first_page = match self.client.fetch(1).await {
                Ok(page) => page,
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to fetch first USDA page");
                    tx.rollback().await?;
                    return Err(e);
                }
            };

            let total_pages = first_page.total_pages;
            tracing::info!(%total_pages, "Starting USDA sync");

            if let Err(e) = persist_food_data(tx.as_mut(), first_page).await {
                tracing::error!(error = ?e, "Failed to persist USDA first page food data");
                tx.rollback().await?;
                return Err(e);
            };

            let client = self.client.clone();
            tracing::info!("making supervisor");
            let mut supervisor = AggregatorSupervisor::new(&mut self.limiter, client, total_pages);

            match supervisor.run(tx.as_mut()).await {
                Ok(status) => {
                    tracing::info!(?status, "USDA sync complete");
                    tx.commit().await?;
                    Ok(status)
                }
                Err(e) => {
                    tracing::error!(error = ?e, "USDA sync failed");
                    tx.rollback().await?;
                    Err(e)
                }
            }
        })
    }
}
