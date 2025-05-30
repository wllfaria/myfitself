mod usda_client;
mod usda_types;

use std::num::NonZeroU32;

use governor::clock::{Clock, QuantaClock, Reference};
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
pub use usda_client::UsdaClient;
use usda_types::UsdaFoodSearchResponse;

use crate::{AggregateStatus, Aggregator, BoxFuture, FoodSource, FoodSourceStatus};

#[derive(Debug)]
pub struct UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse>,
{
    limiter: RateLimiter<NotKeyed, InMemoryState, QuantaClock>,
    client: C,
}

impl<C> UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse>,
{
    pub fn new(client: C) -> Self {
        let quota = Quota::per_hour(NonZeroU32::new(1000).unwrap());
        let limiter = RateLimiter::direct(quota);

        Self { limiter, client }
    }
}

impl<C> Aggregator for UsdaAggregator<C>
where
    C: FoodSource<Data = UsdaFoodSearchResponse>,
{
    fn aggregate(&mut self) -> BoxFuture<AggregateStatus> {
        Box::pin(async move {
            let mut status = FoodSourceStatus::HasRemainingResults;

            while status == FoodSourceStatus::HasRemainingResults {
                if let Err(err) = self.limiter.check() {
                    let now = governor::clock::QuantaClock::default().now();
                    let earliest = err.earliest_possible();

                    let wait_duration = earliest.duration_since(now);
                    let wake_time = tokio::time::Instant::now() + wait_duration.into();
                    return AggregateStatus::PendingUntil(wake_time);
                }

                let result = self.client.fetch_next().await;
                status = result.status;
            }

            AggregateStatus::Finished
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_something() {
        let client = UsdaClient::new();
        let mut aggregator = UsdaAggregator::new(client);

        let result = aggregator.aggregate().await;
        println!("{result:?}");

        panic!();
    }
}
