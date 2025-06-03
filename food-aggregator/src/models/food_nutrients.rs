use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;

#[derive(Debug, FromRow)]
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
}
