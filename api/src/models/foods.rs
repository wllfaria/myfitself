use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct Foods {
    pub id: sqlx::types::Uuid,
    pub name: String,
    pub source: String,
    pub external_id: i32,
    pub fndds_code: Option<String>,
    pub wweia_category: Option<sqlx::types::Uuid>,
}
