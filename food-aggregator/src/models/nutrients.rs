use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Nutrients {
    pub id: sqlx::types::Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
