use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct FoodSources {
    pub id: sqlx::types::Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateFoodSourcePayload {
    pub name: String,
}

impl CreateFoodSourcePayload {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl FoodSources {
    pub async fn maybe_create(
        executor: &mut PgConnection,
        create_source_payload: CreateFoodSourcePayload,
    ) -> anyhow::Result<FoodSources> {
        let source = sqlx::query_as!(
            FoodSources,
            r#"
            INSERT INTO food_sources (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING
            *;
            "#,
            create_source_payload.name
        )
        .fetch_one(&mut *executor)
        .await?;

        Ok(source)
    }
}
