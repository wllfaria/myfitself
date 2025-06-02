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
    fn aggregate(&mut self, pool: PgPool) -> BoxFuture<anyhow::Result<AggregateStatus>> {
        Box::pin(async move {
            // Use one entry from limiter to account for the first request
            // Safety: first request will not fail rate-limit.
            self.limiter.check().unwrap();
            let mut tx = pool.begin().await?;

            // This first request is made separately in order to fetch the total_pages from USDA
            // api, so that we can coordinate the concurrent syncing
            let result = self.client.fetch(1).await?;
            let total_pages = result.total_pages;
            persist_food_data(tx.as_mut(), result).await?;

            let client = self.client.clone();
            let mut supervisor = AggregatorSupervisor::new(&mut self.limiter, client, total_pages);

            let result = supervisor.run(tx.as_mut()).await;
            match result {
                Ok(_) => tx.commit().await?,
                Err(_) => tx.rollback().await?,
            }

            result
        })
    }
}
