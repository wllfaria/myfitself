use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;
use sqlx::{PgConnection, QueryBuilder};

#[derive(Debug, Serialize, FromRow)]
pub struct FoodNutrients {
    id: sqlx::types::Uuid,
    food_id: Uuid,
    nutrient_id: Uuid,
    unit_id: Uuid,
    source_id: Uuid,
    value: f32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateFoodNutrientPayload {
    food_id: Uuid,
    nutrient_id: Uuid,
    unit_id: Uuid,
    source_id: Uuid,
    value: f32,
}

impl CreateFoodNutrientPayload {
    pub fn new(
        food_id: Uuid,
        nutrient_id: Uuid,
        unit_id: Uuid,
        source_id: Uuid,
        value: f32,
    ) -> Self {
        Self {
            food_id,
            nutrient_id,
            unit_id,
            source_id,
            value,
        }
    }
}

impl FoodNutrients {
    pub async fn create_or_update(
        executor: &mut PgConnection,
        create_nutrient_payload: CreateFoodNutrientPayload,
    ) -> sqlx::Result<FoodNutrients> {
        let food_nutrient = sqlx::query_as!(
            FoodNutrients,
            r#"
            INSERT INTO food_nutrients (food_id, nutrient_id, unit_id, source_id, value)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (food_id, nutrient_id, source_id)
            DO UPDATE SET value = EXCLUDED.value
            RETURNING *;
            "#,
            create_nutrient_payload.food_id,
            create_nutrient_payload.nutrient_id,
            create_nutrient_payload.unit_id,
            create_nutrient_payload.source_id,
            create_nutrient_payload.value,
        )
        .fetch_one(executor)
        .await?;

        Ok(food_nutrient)
    }

    pub async fn create_or_update_bulk(
        executor: &mut PgConnection,
        bulk_create_payload: Vec<CreateFoodNutrientPayload>,
    ) -> sqlx::Result<()> {
        if bulk_create_payload.is_empty() {
            return Ok(());
        }

        for chunk in bulk_create_payload.chunks(1000) {
            let mut query_builder = QueryBuilder::new(
                "INSERT INTO food_nutrients (food_id, nutrient_id, unit_id, source_id, value) ",
            );

            query_builder.push_values(chunk, |mut b, payload| {
                b.push_bind(payload.food_id)
                    .push_bind(payload.nutrient_id)
                    .push_bind(payload.unit_id)
                    .push_bind(payload.source_id)
                    .push_bind(payload.value);
            });

            query_builder.push(" ON CONFLICT (food_id, nutrient_id, source_id) DO NOTHING");
            query_builder.build().execute(executor.as_mut()).await?;
        }

        Ok(())
    }
}
