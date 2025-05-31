use axum::Extension;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use clerk_rs::validators::authorizer::ClerkJwt;

use crate::error::AppError;
use crate::{AppState, models, services};

#[tracing::instrument(skip_all)]
pub async fn attach_user(
    State(state): State<AppState>,
    Extension(jwt): Extension<ClerkJwt>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let clerk_user = services::clerk::get_clerk_user(&state, jwt).await?;
    let user = models::users::User::sync_from_clerk(&state, clerk_user).await?;
    tracing::info!("{user:?}");

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
