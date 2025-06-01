mod usda_client;
mod usda_types;

use std::num::NonZeroU32;

use governor::clock::{Clock, QuantaClock, Reference};
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
pub use usda_client::UsdaClient;
use usda_types::UsdaFoodSearchResponse;

use crate::models::food_nutrients::FoodNutrients;
use crate::models::food_sources::{CreateFoodSourcePayload, FoodSources};
use crate::models::foods::{CreateFoodPayload, Foods};
use crate::models::nutrients::Nutrients;
use crate::models::units::Units;
use crate::models::wweia_categories::{CreateWWEIACategoryPayload, WWEIACategories};
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
    fn aggregate(&mut self, pool: PgPool) -> BoxFuture<anyhow::Result<AggregateStatus>> {
        Box::pin(async move {
            let mut status = FoodSourceStatus::HasRemainingResults;

            while status == FoodSourceStatus::HasRemainingResults {
                if let Err(err) = self.limiter.check() {
                    let now = governor::clock::QuantaClock::default().now();
                    let earliest = err.earliest_possible();

                    let wait_duration = earliest.duration_since(now);
                    let wake_time = tokio::time::Instant::now() + wait_duration.into();
                    return Ok(AggregateStatus::PendingUntil(wake_time));
                }

                let result = self.client.fetch_next().await;
                status = result.status;

                let mut tx = pool.begin().await?;

                for food in result.data.foods {
                    let source = FoodSources::maybe_create(
                        tx.as_mut(),
                        CreateFoodSourcePayload::new(String::from("USDA")),
                    )
                    .await?;

                    let category_id = match (food.food_category_id, food.food_category) {
                        (Some(category_id), Some(category_name)) => {
                            let category = WWEIACategories::maybe_create(
                                tx.as_mut(),
                                CreateWWEIACategoryPayload::new(category_id, category_name),
                            )
                            .await?;

                            Some(category.id)
                        }
                        _ => None,
                    };

                    let stored_food = Foods::create_or_update(
                        tx.as_mut(),
                        CreateFoodPayload::new(
                            food.description,
                            food.food_code,
                            source.id,
                            food.fdc_id,
                            category_id,
                        ),
                    )
                    .await?;

                    for nutrient in food.food_nutrients {
                        let store_nutrient =
                            Nutrients::maybe_create(tx.as_mut(), &nutrient.nutrient_name).await?;

                        let unit = Units::maybe_create(tx.as_mut(), &nutrient.unit_name).await?;

                        // TODO: update every nutrient or create them on the db
                    }
                }
            }

            Ok(AggregateStatus::Finished)
        })
    }
}
