use clerk_rs::validators::authorizer::ClerkJwt;

use crate::error::AppError;
use crate::models::users::User;
use crate::{AppState, services};

#[tracing::instrument(skip_all)]
pub async fn sync_user(state: AppState, jwt: ClerkJwt) -> Result<User, AppError> {
    let mut conn = state.db.acquire().await?;
    let clerk_user = services::clerk::get_clerk_user(&state, jwt).await?;
    let user = User::sync_from_clerk(conn.as_mut(), clerk_user).await?;
    Ok(user)
}
