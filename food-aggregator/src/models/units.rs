use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;
use sqlx::{PgConnection, QueryBuilder, Row};

#[derive(Debug, FromRow)]
pub struct Units {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Units {
    pub async fn maybe_create(executor: &mut PgConnection, name: &str) -> sqlx::Result<Units> {
        let units = sqlx::query_as!(
            Units,
            r#"
            INSERT INTO units (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *;
            "#,
            name
        )
        .fetch_one(executor)
        .await?;

        Ok(units)
    }

    pub async fn maybe_create_bulk(
        executor: &mut PgConnection,
        bulk_payload: impl Iterator<Item = &str>,
    ) -> sqlx::Result<HashMap<String, Uuid>> {
        let bulk_payload = bulk_payload.collect::<Vec<_>>();
        if bulk_payload.is_empty() {
            return Ok(HashMap::default());
        }

        let mut query_builder = QueryBuilder::new("INSERT INTO units (name) ");
        query_builder.push_values(bulk_payload, |mut b, name| {
            b.push_bind(name);
        });
        query_builder.push(" ON CONFLICT (name) DO NOTHING");
        query_builder.build().execute(executor.as_mut()).await?;

        let rows = sqlx::query("SELECT id, name FROM units")
            .fetch_all(executor)
            .await?;

        let map = rows
            .into_iter()
            .map(|row| (row.get("name"), row.get("id")))
            .collect();

        Ok(map)
    }
}
