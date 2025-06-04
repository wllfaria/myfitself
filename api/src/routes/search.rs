use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use super::HttpResponse;
use crate::AppState;
use crate::error::AppError;
use crate::services::search::FoodSearchResult;

pub fn search_routes() -> Router<AppState> {
    Router::new().route("/food", get(search_food))
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    query: String,
    limit: Option<usize>,
}

async fn search_food(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<HttpResponse<Vec<FoodSearchResult>>>, AppError> {
    let results = state
        .search_service
        .search(params.query, params.limit.unwrap_or(50))?;

    Ok(Json(results.into()))
}
