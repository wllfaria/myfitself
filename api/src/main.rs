mod error;
mod handlers;
mod middlewares;
mod models;
mod routes;
mod services;

use anyhow::Context;
use axum::Router;
use axum::middleware::from_fn_with_state;
use clerk_rs::ClerkConfiguration;
use clerk_rs::clerk::Clerk;
use clerk_rs::validators::axum::ClerkLayer;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;
use middlewares::attach_user;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Clone)]
pub struct AppState {
    pub clerk: Clerk,
    pub db: PgPool,
}

async fn db_connect() -> anyhow::Result<PgPool> {
    let database_url = dotenvy::var("DATABASE_URL").context("DATABASE_URL env var must be set")?;

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .context("failed to connect to DATABASE_URL")?;

    sqlx::migrate!().run(&db).await?;

    Ok(db)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = dotenvy::var("PORT").context("PORT env var must be set")?;
    let clerk_key =
        dotenvy::var("CLERK_SECRET_KEY").context("CLERK_SECRET_KEY env var must be set")?;

    let config = ClerkConfiguration::new(None, None, Some(clerk_key), None);

    let clerk = Clerk::new(config);
    let db = db_connect().await?;

    let cron_db = db.clone();
    tokio::spawn(async move {
        loop {
            // TODO: don't ignore the error here
            food_aggregator::aggregate_food_data(cron_db.clone())
                .await
                .ok();

            const ONE_DAY: u64 = 60 * 60 * 24;
            tokio::time::sleep(tokio::time::Duration::from_secs(ONE_DAY)).await;
        }
    });

    let state = AppState {
        clerk: clerk.clone(),
        db,
    };

    let clerk_layer = ClerkLayer::new(MemoryCacheJwksProvider::new(clerk), None, true);
    let auth_routes = routes::auth::auth_routes().layer(clerk_layer.clone());

    let aggregate_routes = routes::aggregator::aggregator_routes()
        .layer(from_fn_with_state(state.clone(), attach_user))
        .layer(clerk_layer);

    let app = Router::<AppState>::new()
        .nest("/auth", auth_routes)
        .nest("/aggregator", aggregate_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}
