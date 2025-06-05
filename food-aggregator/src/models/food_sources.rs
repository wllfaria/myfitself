use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;
use sqlx::{PgConnection, QueryBuilder, Row};

#[derive(Debug, Serialize, FromRow)]
pub struct FoodSources {
    pub id: Uuid,
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
    ) -> sqlx::Result<FoodSources> {
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

    pub async fn maybe_create_bulk(
        executor: &mut PgConnection,
        bulk_payload: impl Iterator<Item = String>,
    ) -> sqlx::Result<HashMap<String, Uuid>> {
        let bulk_payload = bulk_payload.collect::<Vec<_>>();
        if bulk_payload.is_empty() {
            return Ok(HashMap::default());
        }

        let mut query_builder = QueryBuilder::new("INSERT INTO food_sources (name) ");

        query_builder.push_values(bulk_payload, |mut b, source| {
            b.push_bind(source);
        });

        query_builder.push(" ON CONFLICT (name) DO NOTHING");
        query_builder.build().execute(executor.as_mut()).await?;

        let rows = sqlx::query("SELECT id, name FROM food_sources")
            .fetch_all(executor)
            .await?;

        let map = rows
            .into_iter()
            .map(|row| (row.get("name"), row.get("id")))
            .collect();

        Ok(map)
    }
}
