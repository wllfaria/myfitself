use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct WWEIACategories {
    pub id: sqlx::types::Uuid,
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
    ) -> anyhow::Result<WWEIACategories> {
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
}
