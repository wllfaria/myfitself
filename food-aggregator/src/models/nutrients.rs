use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Nutrients {
    id: sqlx::types::Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Nutrients {
    pub async fn maybe_create(conn: &mut PgConnection, name: &str) -> anyhow::Result<Nutrients> {
        let nutrients = sqlx::query_as!(
            Nutrients,
            r#"
            INSERT INTO nutrients (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *;
            "#,
            name
        )
        .fetch_one(conn)
        .await?;

        Ok(nutrients)
    }
}
