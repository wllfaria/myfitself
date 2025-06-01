use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Units {
    id: sqlx::types::Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Units {
    pub async fn maybe_create(conn: &mut PgConnection, name: &str) -> anyhow::Result<Units> {
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
        .fetch_one(conn)
        .await?;

        Ok(units)
    }
}
