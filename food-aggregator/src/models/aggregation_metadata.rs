use sqlx::PgConnection;
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
pub struct AggregateMetadataModel {
    pub id: sqlx::types::Uuid,
    pub last_run: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AggregateMetadataModel {
    pub async fn get_last_run(
        executor: &mut PgConnection,
    ) -> sqlx::Result<Option<AggregateMetadataModel>> {
        let aggregation_metadata = sqlx::query_as!(
            AggregateMetadataModel,
            r#"
            SELECT
                *
            FROM
                aggregation_metadata;
            "#
        )
        .fetch_optional(executor)
        .await?;

        Ok(aggregation_metadata)
    }

    pub async fn create(
        executor: &mut PgConnection,
    ) -> Result<AggregateMetadataModel, sqlx::Error> {
        let aggregation_metadata = sqlx::query_as!(
            AggregateMetadataModel,
            "INSERT INTO aggregation_metadata DEFAULT VALUES RETURNING *;",
        )
        .fetch_one(executor)
        .await?;

        Ok(aggregation_metadata)
    }
}
