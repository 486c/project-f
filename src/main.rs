use std::{net::SocketAddr, sync::Arc};

use axum::{http::{header, StatusCode}, response::IntoResponse, Router};
use dotenvy::dotenv;
use manager::Manager;
use sqlx::SqlitePool;
use tokio::{net::TcpListener, sync::Mutex};
use eyre::Result;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir, trace::{self, TraceLayer}};

mod manager;
mod routes;
mod state;

async fn error_404() -> impl IntoResponse {
  (StatusCode::NOT_FOUND, "404 Not Found")
}

async fn shutdown_signal() {
  tokio::signal::ctrl_c()
    .await
    .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() -> Result<()> {
  dotenv()?;

  tracing_subscriber::fmt()
      .with_max_level(tracing::Level::INFO)
      .with_target(false)
      .with_thread_names(true)
      .init();

  let port = 9999;

  let listener = {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).await?
  };

  tracing::info!("Starting server at :{port}");

  let pool = SqlitePool::connect("sqlite:db.db").await?;

  let manager = Manager::new(pool);
  let state = Arc::new(Mutex::new(manager));

  let app = Router::new()
    .nest("/files", routes::files::router())
    .nest("/manage", routes::manage::router())
    .nest_service("/", ServeDir::new("static"))
    // TODO: disable in production?
    .layer(
      CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::CONTENT_RANGE])
    )
    .layer(
        TraceLayer::new_for_http()
            .on_request(trace::DefaultOnRequest::new().level(tracing::Level::INFO))
            .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO))
    )
    .fallback(error_404)
    .with_state(state);

  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

  Ok(())
}
