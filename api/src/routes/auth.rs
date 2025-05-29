use axum::extract::State;
use axum::routing::post;
use axum::{Extension, Json, Router};
use clerk_rs::validators::authorizer::ClerkJwt;

use super::HttpResponse;
use crate::error::AppError;
use crate::models::users::User;
use crate::{AppState, handlers};

pub fn auth_routes() -> Router<AppState> {
    Router::new().route("/sync-user", post(sync_user))
}

async fn sync_user(
    State(state): State<AppState>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<HttpResponse<User>>, AppError> {
    let user = handlers::auth::sync_user(state, jwt).await?;
    Ok(Json(user.into()))
}
