use sqlx::PgPool;
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
pub struct AggregateMetadataModel {
    pub id: sqlx::types::Uuid,
    pub last_run: DateTime<Utc>,
}

impl AggregateMetadataModel {
    pub async fn get_last_run(pool: &PgPool) -> anyhow::Result<Option<AggregateMetadataModel>> {
        let aggregation_metadata =
            sqlx::query_as!(AggregateMetadataModel, "SELECT * FROM aggregation_metadata")
                .fetch_optional(pool)
                .await?;

        Ok(aggregation_metadata)
    }
}
