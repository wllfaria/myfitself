use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct FoodNutrients {
    id: sqlx::types::Uuid,
    food_id: sqlx::types::Uuid,
    nutrient_id: sqlx::types::Uuid,
    unit_id: sqlx::types::Uuid,
    source_id: sqlx::types::Uuid,
    value: f32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
