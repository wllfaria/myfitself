use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;
use sqlx::{PgConnection, QueryBuilder, Row};

#[derive(Debug, Serialize, FromRow)]
pub struct WWEIACategories {
    pub id: Uuid,
    pub code: i32,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateWWEIACategoryPayload<'data> {
    code: i32,
    name: &'data str,
}

impl<'data> CreateWWEIACategoryPayload<'data> {
    pub fn new(code: i32, name: &'data str) -> Self {
        Self { code, name }
    }
}

impl WWEIACategories {
    pub async fn maybe_create(
        executor: &mut PgConnection,
        create_category_payload: CreateWWEIACategoryPayload<'_>,
    ) -> sqlx::Result<WWEIACategories> {
        let category = sqlx::query_as!(
            WWEIACategories,
            r#"
            INSERT INTO wweia_categories (code, name)
            VALUES ($1, $2)
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *;
            "#,
            create_category_payload.code,
            create_category_payload.name,
        )
        .fetch_one(executor)
        .await?;

        Ok(category)
    }

    pub async fn maybe_create_bulk(
        executor: &mut PgConnection,
        bulk_payload: impl Iterator<Item = (i32, &String)>,
    ) -> sqlx::Result<HashMap<String, Uuid>> {
        let bulk_payload = bulk_payload.collect::<Vec<_>>();
        if bulk_payload.is_empty() {
            return Ok(HashMap::default());
        }

        let mut query_builder = QueryBuilder::new("INSERT INTO wweia_categories (code, name) ");

        query_builder.push_values(bulk_payload, |mut b, (code, name)| {
            b.push_bind(code).push_bind(name);
        });

        query_builder.push(" ON CONFLICT (name) DO NOTHING");
        query_builder.build().execute(executor.as_mut()).await?;

        let rows = sqlx::query("SELECT id, name FROM wweia_categories")
            .fetch_all(executor)
            .await?;

        let map = rows
            .into_iter()
            .map(|row| (row.get("name"), row.get("id")))
            .collect();

        Ok(map)
    }
}
