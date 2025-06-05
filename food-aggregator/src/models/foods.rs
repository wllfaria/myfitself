use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;
use sqlx::{PgConnection, QueryBuilder};

use super::food_nutrients::FoodNutrients;
use super::food_sources::FoodSources;
use super::wweia_categories::WWEIACategories;

#[derive(Debug, FromRow)]
pub struct Foods {
    pub id: Uuid,
    pub name: String,
    pub source_id: Uuid,
    pub external_id: i32,
    pub fndds_code: Option<i32>,
    pub wweia_category: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateFoodPayload<'data> {
    pub name: &'data str,
    pub fndds_code: Option<i32>,
    pub source_id: Uuid,
    pub external_id: i32,
    pub wweia_category: Option<Uuid>,
}

impl<'data> CreateFoodPayload<'data> {
    pub fn new(
        name: &'data str,
        fndds_code: Option<i32>,
        source_id: Uuid,
        external_id: i32,
        wweia_category: Option<Uuid>,
    ) -> Self {
        Self {
            name,
            fndds_code,
            source_id,
            external_id,
            wweia_category,
        }
    }
}

#[derive(Debug, Serialize, FromRow)]
pub struct SearchSchemaFood {
    id: Uuid,
    name: String,
    source: String,
}

impl SearchSchemaFood {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

impl Foods {
    pub async fn get_for_search(
        executor: &mut PgConnection,
    ) -> sqlx::Result<Vec<SearchSchemaFood>> {
        let search_schema = sqlx::query_as!(
            SearchSchemaFood,
            r#"
            SELECT
                f.id AS id,
                f.name AS name,
                fs.name AS source
            FROM
                foods f
                JOIN food_sources fs ON f.source_id = fs.id;
            "#
        )
        .fetch_all(executor)
        .await?;

        Ok(search_schema)
    }

    pub async fn create_or_update(
        executor: &mut PgConnection,
        create_food_payload: CreateFoodPayload<'_>,
    ) -> sqlx::Result<Foods> {
        let food = sqlx::query_as!(
            Foods,
            r#"
            INSERT INTO foods (name, source_id, external_id, fndds_code, wweia_category)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (source_id, external_id) DO UPDATE SET
                name = EXCLUDED.name,
                fndds_code = EXCLUDED.fndds_code,
                wweia_category = EXCLUDED.wweia_category
            RETURNING *;
            "#,
            create_food_payload.name,
            create_food_payload.source_id,
            create_food_payload.external_id,
            create_food_payload.fndds_code,
            create_food_payload.wweia_category,
        )
        .fetch_one(executor)
        .await?;

        Ok(food)
    }

    pub async fn create_or_update_bulk(
        executor: &mut PgConnection,
        mut bulk_create_payload: impl Iterator<Item = CreateFoodPayload<'_>>,
    ) -> sqlx::Result<HashMap<(String, i32), Uuid>> {
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO foods (name, source_id, external_id, fndds_code, wweia_category) ",
        );
        query_builder.push_values(&mut bulk_create_payload, |mut b, payload| {
            b.push_bind(payload.name)
                .push_bind(payload.source_id)
                .push_bind(payload.external_id)
                .push_bind(payload.fndds_code)
                .push_bind(payload.wweia_category);
        });
        query_builder.push(
            r#" ON CONFLICT (source_id, external_id) DO UPDATE SET
                name = EXCLUDED.name,
                fndds_code = EXCLUDED.fndds_code,
                wweia_category = EXCLUDED.wweia_category
            "#,
        );
        query_builder.build().execute(executor.as_mut()).await?;

        let mut query_builder = QueryBuilder::new(
            r#"
            SELECT f.id, f.external_id, s.name as source_name
            FROM foods f
            JOIN food_sources s ON f.source_id = s.id
            "#,
        );

        let map = query_builder
            .build_query_as::<(Uuid, i32, String)>()
            .fetch_all(executor.as_mut())
            .await?
            .into_iter()
            .map(|(id, external_id, source_name)| ((source_name, external_id), id))
            .collect();

        Ok(map)
    }
}
