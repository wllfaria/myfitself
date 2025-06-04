mod error;
mod handlers;
mod middlewares;
mod models;
mod routes;
mod services;

use axum::Router;
use axum::middleware::from_fn_with_state;
use clerk_rs::ClerkConfiguration;
use clerk_rs::clerk::Clerk;
use clerk_rs::validators::axum::ClerkLayer;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;
use food_aggregator::AggregateStatus;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::middlewares::attach_user;
use crate::services::search::SearchService;

#[derive(Clone)]
pub struct AppState {
    pub clerk: Clerk,
    pub db: PgPool,
    pub search_service: SearchService,
}

async fn db_connect() -> sqlx::Result<PgPool> {
    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL env var must be set");

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("failed to connect to DATABASE_URL");

    sqlx::migrate!().run(&db).await?;

    Ok(db)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = dotenvy::var("PORT").expect("PORT env var must be set");
    let clerk_key = dotenvy::var("CLERK_SECRET_KEY").expect("CLERK_SECRET_KEY env var must be set");

    let config = ClerkConfiguration::new(None, None, Some(clerk_key), None);

    let clerk = Clerk::new(config);
    let db = db_connect().await?;

    let cron_db = db.clone();
    tokio::spawn(async move {
        loop {
            // TODO: don't ignore the error here
            match food_aggregator::aggregate_food_data(cron_db.clone()).await {
                Ok(status) => match status {
                    AggregateStatus::Finished => tracing::info!("Aggregation finished"),
                    AggregateStatus::PendingUntil(_) => tracing::info!("Aggregation postponed"),
                },
                Err(_) => tracing::error!("Aggregation failed to execute"),
            }

            const ONE_DAY: u64 = 60 * 60 * 24;
            tokio::time::sleep(tokio::time::Duration::from_secs(ONE_DAY)).await;
        }
    });

    let search_service = {
        let mut conn = db.acquire().await?;
        SearchService::new(&mut conn).await?
    };

    let state = AppState {
        clerk: clerk.clone(),
        search_service,
        db,
    };

    let clerk_layer = ClerkLayer::new(MemoryCacheJwksProvider::new(clerk), None, true);
    let auth_routes = routes::auth::auth_routes().layer(clerk_layer.clone());
    let search_routes = routes::search::search_routes();

    let aggregate_routes = routes::aggregator::aggregator_routes()
        .layer(from_fn_with_state(state.clone(), attach_user))
        .layer(clerk_layer);

    let app = Router::<AppState>::new()
        .nest("/auth", auth_routes)
        .nest("/aggregator", aggregate_routes)
        .nest("/search", search_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}
