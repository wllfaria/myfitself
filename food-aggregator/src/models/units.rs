use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Units {
    pub id: sqlx::types::Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Units {
    pub async fn maybe_create(conn: &mut PgConnection, name: &str) -> sqlx::Result<Units> {
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
