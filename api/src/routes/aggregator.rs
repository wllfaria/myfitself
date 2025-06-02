use axum::routing::get;
use axum::{Extension, Json, Router};

use super::HttpResponse;
use crate::AppState;
use crate::error::AppError;
use crate::models::users::User;

pub fn aggregator_routes() -> Router<AppState> {
    Router::new().route("/aggregate", get(run_aggregators))
}

pub async fn run_aggregators(
    Extension(_user): Extension<User>,
) -> Result<Json<HttpResponse<bool>>, AppError> {
    Ok(Json(HttpResponse::from(true)))
}
