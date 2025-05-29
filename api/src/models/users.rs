use serde::Serialize;
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};

use crate::AppState;
use crate::services::clerk::ClerkUser;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: sqlx::types::Uuid,
    pub clerk_id: String,
    pub username: Option<String>,
    pub needs_setup: bool,
    pub calorie_goal: Option<i32>,
    pub email: String,
    pub has_image: bool,
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub async fn sync_from_clerk(state: &AppState, clerk_user: ClerkUser) -> anyhow::Result<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE clerk_id = $1",
            clerk_user.id()
        )
        .fetch_optional(&state.db)
        .await?;

        match user {
            Some(user) => Ok(user),
            None => User::create_from_clerk(state, clerk_user).await,
        }
    }

    pub async fn create_from_clerk(
        state: &AppState,
        clerk_user: ClerkUser,
    ) -> anyhow::Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (clerk_id, email, has_image, image_url)
            VALUES ($1, $2, $3, $4)
            RETURNING *;
            "#,
            clerk_user.id(),
            clerk_user.email(),
            clerk_user.has_image(),
            clerk_user.image_url(),
        )
        .fetch_one(&state.db)
        .await?;

        Ok(user)
    }
}
